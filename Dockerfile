FROM debian:bullseye@sha256:3066ef83131c678999ce82e8473e8d017345a30f5573ad3e44f62e5c9c46442b

WORKDIR /init
COPY init.sh /init/
RUN ./init.sh

WORKDIR /
RUN rm -rf /init

COPY --chown=root:root \
	./target/release/discord-tts \
	/usr/bin/discord-tts

VOLUME /var/discordtts/
CMD ["/usr/bin/discord-tts"]
