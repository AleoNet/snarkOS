FROM debian:stable-slim AS builder

RUN apt-get update && \
    apt-get install -y \
        binutils-dev \
        build-essential \
        cmake \
        git \
        libcurl4-openssl-dev \
        libdw-dev \
        libiberty-dev \
        ninja-build \
        libelf-dev \
        cmake \
        python3 \
        gcc \
        libiberty-dev \
        libcurl4-openssl-dev \
        zlib1g-dev \
        ;
RUN apt-get install -y curl libssl-dev;

RUN curl -LO https://github.com/SimonKagstrom/kcov/archive/master.tar.gz &&  \
  tar xzf master.tar.gz && cd kcov-master &&  \
  mkdir ./build && \
    cd ./build && \
    cmake -G 'Ninja' .. && \
    cmake --build . && \
    cmake --build . --target install

RUN apt-get clean && \
    rm -rf /var/lib/apt/lists/*


CMD ["/usr/local/bin/kcov"]
