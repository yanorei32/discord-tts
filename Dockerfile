FROM rust:1.67.1 as build-env
LABEL maintainer="yanorei32"

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

# depName=debian_11/cmake
ENV CMAKE_VERSION="3.18.4-2+deb11u1"

RUN apt-get update -qq && apt-get install -qq -y --no-install-recommends \
	"cmake=$CMAKE_VERSION"

WORKDIR /usr/src
RUN cargo new discord-tts
COPY LICENSE Cargo.toml Cargo.lock /usr/src/discord-tts/
WORKDIR /usr/src/discord-tts
RUN	cargo install cargo-license && cargo license \
	--authors \
	--do-not-bundle \
	--avoid-dev-deps \
	--avoid-build-deps \
	--filter-platform "$(rustc -vV | sed -n 's|host: ||p')" \
	> CREDITS

RUN cargo build --release
COPY src/ /usr/src/discord-tts/src/
RUN touch src/**/* src/* && cargo build --release

FROM debian:bullseye@sha256:43ef0c6c3585d5b406caa7a0f232ff5a19c1402aeb415f68bcd1cf9d10180af8

WORKDIR /init
COPY init.sh /init/
RUN ./init.sh

WORKDIR /
RUN rm -rf /init

COPY --chown=root:root --from=build-env \
	/usr/src/discord-tts/CREDITS \
	/usr/src/discord-tts/LICENSE \
	/usr/share/licenses/discord-tts/

COPY --chown=root:root --from=build-env \
	/usr/src/discord-tts/target/release/discord-tts \
	/usr/bin/discord-tts

VOLUME /var/discordtts/
CMD ["/usr/bin/discord-tts"]
