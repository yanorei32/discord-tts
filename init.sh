#!/bin/bash
set -eux

# depName=debian_11/opus
LIBOPUS_VERSION="1.3.1-0.1"

# depName=debian_11/ffmpeg
FFMPEG_VERSION="7:4.3.4-0+deb11u1"

apt-get update -qq

apt-get install -qq -y --no-install-recommends \
	"libopus0=$LIBOPUS_VERSION" "ffmpeg=$FFMPEG_VERSION"

rm -rf /var/lib/apt/lists/*

mkdir /var/discordtts
echo '{}' > /var/discordtts/state.json
