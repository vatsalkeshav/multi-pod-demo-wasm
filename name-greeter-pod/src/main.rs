use std::env;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

async fn handle_client(mut stream: TcpStream) {
    // log the client's peer address
    let peer_addr = stream
        .peer_addr()
        .map_or_else(|_| "unknown".to_string(), |addr| addr.to_string());
    println!("Handling connection from: {}", peer_addr);

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

    // Convert the request to a string for processing
    let request = match String::from_utf8(buffer[..bytes_read].to_vec()) {
        Ok(req) => req,
        Err(e) => {
            eprintln!("Invalid UTF-8 sequence: {}", e);
            return;
        }
    };

    // Extract the path from the first line of the HTTP request
    let path = request
        .lines()
        .next()
        .unwrap_or("")
        .split_whitespace()
        .nth(1)
        .unwrap_or("/");

    // Extract the name from the path (remove leading '/')
    let name = path.trim_start_matches('/');

    // Generate the appropriate response
    let response = if name.is_empty() {
        "HTTP/1.1 200 OK\r\n\r\nPlease provide a name in the URL, e.g., /Alice".to_string()
    } else if name.to_lowercase().starts_with('s') {
        format!("HTTP/1.1 200 OK\r\n\r\nHello {}!", name)
    } else {
        format!("HTTP/1.1 200 OK\r\n\r\nGreetings {}!", name)
    };

    // Send the response (/Greeting)
    // The Ok case of write_all is implicitly accepted as the code continues silently
    if let Err(e) = stream.write_all(response.as_bytes()).await {
        eprintln!("Failed to write response: {}", e);
    }

    println!("Completed handling connection from {}", peer_addr);
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // holds an address
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "0.0.0.0:80".to_string());

    // a TCP listener
    let listener = TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to address");
    println!("Server listening on address: {}", addr);

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
