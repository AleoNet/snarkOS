FROM rust:1.51.0

RUN apt-get update \
 && apt-get install -y \
   build-essential \
   clang \
   gcc \
   libssl-dev \
   make \
   pkg-config \
   xz-utils

WORKDIR /usr/src/snarkOS
COPY . .

RUN cargo build --release

CMD ["./target/release/snarkos"]
