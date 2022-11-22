FROM rust:1.65.0 as build-env
LABEL maintainer="yanorei32"

WORKDIR /usr/src

# depName=debian_11/cmake
ENV CMAKE_VERSION="3.18.4"

RUN apt-get update -y && apt-get install -y "cmake=${CMAKE_VERSION}" --no-install-recommends && cargo new discord-tts
COPY Cargo.toml Cargo.lock /usr/src/discord-tts/
WORKDIR /usr/src/discord-tts
RUN cargo build --release

COPY src/* /usr/src/discord-tts/src/
RUN touch src/* && cargo build --release

FROM debian:bullseye@sha256:3066ef83131c678999ce82e8473e8d017345a30f5573ad3e44f62e5c9c46442b

WORKDIR /init
COPY init.sh /init/
RUN ./init.sh

WORKDIR /
RUN rm -rf /init

COPY --from=build-env \
	/usr/src/discord-tts/target/release/discord-tts \
	/usr/bin/discord-tts

VOLUME /var/discordtts/
CMD ["/usr/bin/discord-tts"]
