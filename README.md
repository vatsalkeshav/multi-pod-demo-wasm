### Steps to Reproduce the problem

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

# make the deployment.yaml and...
k3s kubectl apply -f deployment.yaml
# everything seems to be ok - all pods and service work as intended

# problem! :
curl http://198.19.249.197:30001/split/greg,greyson
# output :
# Error contacting greeter service
```
#### However, if greeters are configured for NodePort service, then they behave exactly as intended :
```sh
curl http://198.19.249.197:30002/grey
# output :
# Greetings grey
```