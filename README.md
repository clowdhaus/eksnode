# eksnode

## MacOS

```sh
brew install FiloSottile/musl-cross/musl-cross

TARGET_CC=x86_64-linux-musl-gcc \
RUSTFLAGS="-C linker=x86_64-linux-musl-gcc" \
cargo build --target=x86_64-unknown-linux-musl --release
```
