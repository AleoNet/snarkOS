FROM aleohq/devnet-cache:latest-amd AS builder

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH \
    DEBIAN_FRONTEND=noninteractive

WORKDIR /usr/src/snarkOS

COPY . .

RUN set -eux ; \
    cargo build --release

#---
FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive

VOLUME ["/aleo/data"]

RUN set -ex && \
    apt-get update && \
    apt-get dist-upgrade -y -o DPkg::Options::=--force-confold && \
    apt-get install -y --no-install-recommends \
    ca-certificates && \
    apt-get purge -y --auto-remove -o APT::AutoRemove::RecommendsImportant=false && \
    apt-get clean && \
    ln -s /aleo/data /root/.aleo && \
    rm -rf /var/lib/apt/lists/* && \
    mkdir -p /aleo/bin && \
    mkdir -p /aleo/data

COPY --from=builder /usr/src/snarkOS/target/release/snarkos /aleo/bin/
COPY --from=builder /usr/src/snarkOS/entrypoint.sh /aleo/

CMD ["/aleo/entrypoint.sh"]

