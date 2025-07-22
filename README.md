## K3s-Demo: wasm-pods-communication

### Architecture :
```
Make a request to the splitter container
  - http://<node's-internal-ip>:30001/split/Sherlock,Po
                |
                ▼
NodePort service configured to the splitter pod (at port 3001)
                |
                ▼
1 splitter pod
  - splits the request to 2 requests
  - sends the 2 requests to the greeter pods
    - using the ClusterIP service
                |
                ▼
ClusterIP loadbalances the 2 requests
                |
                ▼
2 greeter pods handle the requests
  - if name starts with an 'S/s' => response = Hello S_named_person!
    else                    => response = Greetings not_S_named_person!
                |
                ▼
Responses from the 2 pods get back to the splitter pod
  - Splitter pod concatenates the responses
                |
                ▼
final response = Hello Sherlock! and Greetings Po!
```

### Setup
#### 1. Build the images for splitter and greeter pods
```sh
cd name-greeter-pod
rm -fr Cargo.lock && cargo clean
cargo build --target wasm32-wasip1 --release
cd target/wasm32-wasip1/release
oci-tar-builder --name name-greeter-pod \
    --repo ghcr.io/second-state \
    --tag latest \
    --module ./name-greeter-pod.wasm \
    -o ./img-oci-1.tar
sudo k3s ctr image import --all-platforms ./img-oci-1.tar
cd ..
cd name-splitter-pod
rm -fr Cargo.lock && cargo clean
cargo build --target wasm32-wasip1 --release
cd target/wasm32-wasip1/release
oci-tar-builder --name name-splitter-pod \
    --repo ghcr.io/second-state \
    --tag latest \
    --module ./name-splitter-pod.wasm \
    -o ./img-oci-2.tar
sudo k3s ctr image import --all-platforms ./img-oci-2.tar
sudo k3s ctr images ls # verify
cd ..
```

#### 2. Configure in k3s 
  - 2 `name-greeter-pod` (listening on port 80) deployment
    - ClusterIP service `name-greeter-pod-service` exposed at port=70 with targetport=80
  - 1 `name-splitter-pod` (listening on port 90) deployment
    - NodePort service `name-splitter-pod-service` exposed at nodePort: 30001 with targetport=90
```sh
# refer the deployment.yaml and..
k3s kubectl apply -f deployment.yaml
k3s kubectl get all # verify
# all pods and services work as intended
```

#### 3. Query the splitter container
```sh
curl http://<node's-internal-ip>:30001/split/Sherlock,Po
# o/p :
# Hello Sherlock! and Greetings Po!

# if greeters are configured for NodePort service, then they behave exactly as intended :
curl http://<node's-internal-ip>:30002/Aang
# output :
# Greetings Aang
```