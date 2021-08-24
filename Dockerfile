FROM rust:1.54.0 as builder

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


FROM debian:buster-slim

RUN apt-get update \
  && apt-get install -y libssl1.1 libcurl4 \
  && apt-get clean \
  && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

COPY --from=builder /usr/src/snarkOS/target/release/snarkos /usr/local/bin/

CMD ["snarkos"]
