FROM rust:1.57.0 as build-env

COPY . /root/build

RUN cd /root/build && cargo build --release

FROM debian:bullseye-20211220
MAINTAINER yanorei32
COPY --from=build-env \
	/root/build/target/release/discord-tts \
	/usr/bin/discord-tts

CMD ["/usr/bin/discord-tts"]

