#!/bin/bash
set -eux

# depName=debian_11/ffmpeg
FFMPEG_VERSION="7:4.3.6-0+deb11u1"

apt-get update -qq

apt-get install -qq -y --no-install-recommends \
	"ffmpeg=$FFMPEG_VERSION"

rm -rf /var/lib/apt/lists/*

mkdir /var/discordtts
echo '{}' > /var/discordtts/state.json
