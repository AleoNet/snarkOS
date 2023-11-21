FROM rust:1.70-slim-buster
RUN apt-get update -y && apt-get install git -y
RUN ls
RUN git clone \
    https://github.com/puzzlehq/snarkos.git \
    --depth 1
WORKDIR snarkos
RUN ["chmod", "+x", "build_ubuntu.sh"]
RUN ./build_ubuntu.sh
EXPOSE 5000/tcp
EXPOSE 3033/tcp
EXPOSE 4133/tcp
ENTRYPOINT ["./devnet.sh"]