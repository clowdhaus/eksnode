# eksnode

protobuf is required for compiling (containerd)

## MacOS

```sh
brew install FiloSottile/musl-cross/musl-cross

rustup target add x86_64-unknown-linux-musl

TARGET_CC=x86_64-linux-musl-gcc \
RUSTFLAGS="-C linker=x86_64-linux-musl-gcc" \
cargo build --target=x86_64-unknown-linux-musl --release &&  \
cp target/x86_64-unknown-linux-musl/release/eksnode target/release/eksnode && \
upx target/release/eksnode --ultra-brute
```
