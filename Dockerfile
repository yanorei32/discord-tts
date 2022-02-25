FROM rust:1.59.0 as build-env

WORKDIR /usr/src

RUN cargo new discord-tts
COPY Cargo.toml Cargo.lock /usr/src/discord-tts/
WORKDIR /usr/src/discord-tts
RUN cargo build --release

COPY src/* /usr/src/discord-tts/src/
RUN touch src/* && cargo build --release

FROM debian:bullseye-20211220
MAINTAINER yanorei32

RUN set -ex; \
	apt-get update -qq; \
	apt-get install -qq -y --no-install-recommends \
		libopus0 ffmpeg; \
	rm -rf /var/lib/apt/lists/*; \
	mkdir /var/discordtts; \
	echo '{}' > /var/discordtts/state.json;

COPY --from=build-env \
	/usr/src/discord-tts/target/release/discord-tts \
	/usr/bin/discord-tts

VOLUME /var/discordtts/
WORKDIR "/"
CMD ["/usr/bin/discord-tts"]

