FROM rust:1.43

RUN apt-get update \
 && apt-get install -y --no-install-recommends \
   make \
   clang \
   pkg-config \
   xz-utils \
   libssl-dev

WORKDIR /usr/src/snarkOS
COPY . .

RUN cargo build --release

CMD ["./target/release/snarkos"]