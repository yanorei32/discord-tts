FROM rust:1.81.0-bookworm as build-env
LABEL maintainer="yanorei32"

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

# depName=debian_12/cmake
ENV CMAKE_VERSION="3.25.1-1"

RUN apt-get update -qq && apt-get install -qq -y --no-install-recommends \
	"cmake=$CMAKE_VERSION"

WORKDIR /usr/src
RUN cargo new discord-tts
COPY LICENSE Cargo.toml Cargo.lock /usr/src/discord-tts/
WORKDIR /usr/src/discord-tts
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
RUN	cargo install cargo-license && cargo license \
	--authors \
	--do-not-bundle \
	--avoid-dev-deps \
	--avoid-build-deps \
	--filter-platform "$(rustc -vV | sed -n 's|host: ||p')" \
	> CREDITS && \
	echo 'emoji-ja: 94e387b36d2edd1239f3a2b8ca1324b963596855, "MIT", by, yag_ays' >> CREDITS

RUN cargo build --release
COPY src/ /usr/src/discord-tts/src/
COPY assets/ /usr/src/discord-tts/assets/
RUN touch src/**/* src/* && cargo build --release

FROM debian:bookworm-slim@sha256:7095ea629c4563714b9655137db2eacd456eb3eea0eb8a2b0a4a6b0b187220a9

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
