FROM rust:1.70-slim-buster
RUN apt-get update -y && apt-get install git -y && apt-get install tmux -y && apt-get install pkg-config -y && apt-get install openssl -y
RUN ls
RUN git clone \
    https://github.com/puzzlehq/snarkos.git \
    --depth 1
WORKDIR snarkos
RUN pwd
RUN ls
# RUN ["chmod", "+x", "build_ubuntu.sh"]
# RUN ./build_ubuntu.sh
EXPOSE 3033/tcp
EXPOSE 4133/tcp
# RUN pwd
# RUN ls
# RUN cargo install --path . --locked
ENTRYPOINT ["./devnet.sh"]