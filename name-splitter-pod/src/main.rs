use std::env;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

async fn forward_to_greeter(
    name: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // connect to one of the greeter pods (using service name in Kubernetes)
    // let address = "name-greeter-pod-service:80";
    let address = "name-greeter-pod-service.default.svc.cluster.local:80";
    let mut stream = match TcpStream::connect(address).await {
        Ok(s) => {
            println!("Successfully connected to greeter service");
            s
        }
        Err(e) => {
            eprintln!("Connection failed to {}: {}", address, e);
            return Err(e.into());
        }
    };

    // send HTTP request
    let request = format!(
        "GET /{} HTTP/1.1\r\nHost: name-greeter-pod-service\r\n\r\n",
        name
    );
    stream.write_all(request.as_bytes()).await?;

    // read response
    let mut buffer = [0; 1024];
    let bytes_read = stream.read(&mut buffer).await?;

    if bytes_read == 0 {
        return Err("Empty response from greeter".into());
    }

    // simple HTTP response parsing
    let response = String::from_utf8_lossy(&buffer);
    if let Some(body_start) = response.find("\r\n\r\n") {
        Ok(response[body_start + 4..].trim().to_string())
    } else {
        Ok(response.trim().to_string())
    }
}

async fn handle_client(mut stream: TcpStream) {
    // log the client's peer address
    let peer_addr = stream
        .peer_addr()
        .map_or_else(|_| "unknown".to_string(), |addr| addr.to_string());
    println!("Splitter server handling connection from: {}", peer_addr);

    let mut buffer = [0; 1024];

    // Read the request
    let bytes_read = match stream.read(&mut buffer).await {
        Ok(no_of_read_bytes) => {
            // sanity check
            if no_of_read_bytes == 0 {
                println!(
                    "client {} closed the connection gracefully - zero bytes read : EOF",
                    peer_addr
                );
                return;
            }
            no_of_read_bytes
        }
        Err(e) => {
            // also wanted to handle std::io::ErrorKind::Interrupted to be allowed - but ig tokio does that for us
            match e.kind() {
                std::io::ErrorKind::ConnectionReset => {
                    println!("Client {} reset the connection", peer_addr);
                }
                _ => {
                    eprintln!("Read Error from Client {} : {}", peer_addr, e);
                }
            }
            return;
        }
    };

    let request = match String::from_utf8(buffer[..bytes_read].to_vec()) {
        Ok(req) => req,
        Err(e) => {
            eprintln!("Invalid UTF-8: {}", e);
            let _ = stream
                .write_all(b"HTTP/1.1 400 Bad Request\r\n\r\nInvalid request")
                .await;
            return;
        }
    };

    // parse path
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");

    // only handle /split/ paths
    if !path.starts_with("/split/") {
        let _ = stream
            .write_all(b"HTTP/1.1 404 Not Found\r\n\r\nUse /split/name1,name2")
            .await;
        return;
    }

    // extract names
    let names = path.trim_start_matches("/split/");
    if names.is_empty() {
        let _ = stream
            .write_all(b"HTTP/1.1 400 Bad Request\r\n\r\nProvide names like /split/Alice,Bob")
            .await;
        return;
    }

    let names: Vec<&str> = names.split(',').collect();
    let mut responses = Vec::with_capacity(names.len());

    // forward to greeter pods
    for name in names {
        match forward_to_greeter(name).await {
            Ok(response) => responses.push(response),
            Err(e) => {
                eprintln!("Error contacting greeter: {}", e);
                let _ = stream
                    .write_all(b"HTTP/1.1 502 Bad Gateway\r\n\r\nError contacting greeter service")
                    .await;
                return;
            }
        }
    }

    // combine responses
    let combined = responses.join(" and ");
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
        combined.len(),
        combined
    );

    if let Err(e) = stream.write_all(response.as_bytes()).await {
        eprintln!("Failed to send response to {}: {}", peer_addr, e);
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // holds an address
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "0.0.0.0:90".to_string()); // splitter on port 90 // greeter on port 80

    // a TCP listener
    let listener = TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to address");
    println!("Splitter server listening on address : {}", addr);

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                tokio::spawn(handle_client(stream));
            }
            Err(e) => {
                eprintln!("Failed to accept connection: {}", e);
            }
        }
    }
}
