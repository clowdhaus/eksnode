# eksnode

## Local Development

The `protoc` Protocol Buffers compiler is required for compiling (containerd). See [grpc docs](https://grpc.io/docs/protoc-installation/) for installation instructions.

### MacOS

Note: not all tests run on MacOS since the binary is designed/intended to run on Amazon Linux 2023+

```sh
brew install FiloSottile/musl-cross/musl-cross

rustup target add x86_64-unknown-linux-musl

TARGET_CC=x86_64-linux-musl-gcc \
RUSTFLAGS="-C linker=x86_64-linux-musl-gcc" \
cargo build --target=x86_64-unknown-linux-musl --release &&  \
cp target/x86_64-unknown-linux-musl/release/eksnode target/release/eksnode && \
upx target/release/eksnode --ultra-brute
```
