# To Update

1. Go to [Rustup components availability page](https://rust-lang.github.io/rustup-components-history/)
2. Find the latest version that supports `rustfmt` component (e.g. 2021-01-31)
3. Update the Dockerfile by changing `RUST_VERSION` to the target version (e.g. nightly-2021-01-31)
4. Build and push:
```bash
docker build -t howardwu/snarkos-codecov:2021-01-31 .
docker push howardwu/snarkos-codecov:2021-01-31
```
