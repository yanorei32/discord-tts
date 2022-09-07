FROM rust:1.63.0 as build-env
LABEL maintainer="yanorei32"

WORKDIR /usr/src

RUN cargo new discord-tts
COPY Cargo.toml Cargo.lock /usr/src/discord-tts/
WORKDIR /usr/src/discord-tts
RUN cargo build --release

COPY src/* /usr/src/discord-tts/src/
RUN touch src/* && cargo build --release

FROM debian:bullseye-20211220

# depName=debian_11/opus
ENV LIBOPUS_VERSION="1.3.1-0.1"

# depName=debian_11/ffmpeg
ENV FFMPEG_VERSION="7:4.3.4-0+deb11u1"

RUN set -ex; \
	apt-get update -qq; \
	apt-get install -qq -y --no-install-recommends \
		"libopus0=$LIBOPUS_VERSION" "ffmpeg=$FFMPEG_VERSION"; \
	rm -rf /var/lib/apt/lists/*; \
	mkdir /var/discordtts; \
	echo '{}' > /var/discordtts/state.json;

COPY --from=build-env \
	/usr/src/discord-tts/target/release/discord-tts \
	/usr/bin/discord-tts

VOLUME /var/discordtts/
WORKDIR "/"
CMD ["/usr/bin/discord-tts"]

