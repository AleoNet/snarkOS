#
# Builder image
#

FROM rust:1-bookworm AS builder

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    clang \
    gcc \
    libssl-dev \
    llvm \
    make \
    pkg-config \
    xz-utils \
    cmake \
    libssl-dev \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

COPY . .

RUN cargo install --locked --path .

#
# Final image
#

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    libcurl4-openssl-dev \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /build/target/release/snarkos .

EXPOSE 4130
EXPOSE 3030
EXPOSE 5000
EXPOSE 3000
EXPOSE 3030
EXPOSE 9000
EXPOSE 9090

ENTRYPOINT ["/app/snarkos"]
