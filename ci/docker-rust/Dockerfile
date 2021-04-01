FROM buildpack-deps:buster

FROM cimg/base:2020.09

ENV RUST_VERSION=nightly-2021-03-25 \
	PATH=/home/circleci/.cargo/bin:$PATH

RUN curl -O https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init && \
	chmod +x rustup-init && \
	./rustup-init -y --no-modify-path --default-toolchain $RUST_VERSION && \
	rm rustup-init && \
	rustc --version && \
	cargo --version

RUN rustup component add rustfmt
