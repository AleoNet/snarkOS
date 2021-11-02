FROM ubuntu:18.04 AS builder

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH \
    DEBIAN_FRONTEND=noninteractive

RUN set -eux ; \
    apt-get update -y && \
    apt-get dist-upgrade -y && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        gcc \
        libc6-dev \
        wget \
        build-essential \
        clang \
        gcc \
        libssl-dev \
        make \
        pkg-config \
        xz-utils && \
    dpkgArch="$(dpkg --print-architecture)"; \
    case "${dpkgArch##*-}" in \
        amd64) rustArch='x86_64-unknown-linux-gnu' ;; \
        arm64) rustArch='aarch64-unknown-linux-gnu' ;; \
        *) echo >&2 "unsupported architecture: ${dpkgArch}"; exit 1 ;; \
    esac; \
    \
    url="https://static.rust-lang.org/rustup/dist/${rustArch}/rustup-init"; \
    wget "$url"; \
    chmod +x rustup-init; \
    ./rustup-init -y --no-modify-path --default-toolchain stable; \
    rm rustup-init; \
    chmod -R a+w $RUSTUP_HOME $CARGO_HOME; \
    rustup --version; \
    cargo --version; \
    rustc --version; \
    apt-get remove -y --auto-remove wget && \
    apt-get purge -y --auto-remove -o APT::AutoRemove::RecommendsImportant=false && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*;

WORKDIR /usr/src/snarkOS

COPY . .

RUN cargo build --release && \
    cargo build --release --bin sync_provider && \
    cargo build --release --bin beacon && \
    cargo build --release --bin crawler

FROM ubuntu:18.04

SHELL ["/bin/bash", "-c"]

VOLUME ["/aleo/data"]

RUN set -ex && \
    apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get dist-upgrade -y -o DPkg::Options::=--force-confold && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends ca-certificates && \
    apt-get purge -y --auto-remove -o APT::AutoRemove::RecommendsImportant=false && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* && \
    mkdir -p /aleo/{bin,data} && \
    mkdir -p /aleo/data/params/{git,registry} && \
    mkdir -p /usr/local/cargo/git/checkouts/snarkvm-f1160780ffe17de8/ef32edc/parameters/src/ && \
    mkdir -p /usr/local/cargo/registry/src/github.com-1ecc6299db9ec823/snarkvm-parameters-0.7.9/src/ && \
    ln -s /aleo/data/params/registry /usr/local/cargo/registry/src/github.com-1ecc6299db9ec823/snarkvm-parameters-0.7.9/src/testnet1

COPY --from=builder /usr/src/snarkOS/target/release/snarkos /aleo/bin/
COPY --from=builder /usr/src/snarkOS/target/release/sync_provider /aleo/bin/
COPY --from=builder /usr/src/snarkOS/target/release/beacon /aleo/bin/
COPY --from=builder /usr/src/snarkOS/target/release/crawler /aleo/bin/
COPY --from=builder /usr/src/snarkOS/target/release/snarkos /aleo/bin/

CMD ["bash", "-c", "/aleo/bin/snarkos -d /aleo/data"]
