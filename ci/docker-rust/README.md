1. Go to [Rustup components availability page](https://rust-lang.github.io/rustup-components-history/)
2. Find out latest version that supports `rustfmt` component (e.g. 2020-08-27)
3. Edit Dockerfile. Change `RUST_VERSION` to the target version e.g. nightly-2020-08-27.
4. Build and push:

```bash
docker build -t howardwu/snarkos-ci:2020-08-27 .
docker push howardwu/snarkos-ci:2020-08-27
```
