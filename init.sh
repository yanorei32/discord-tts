#!/bin/bash
set -eux

# depName=debian_12/ca-certificates
CA_CERTIFICATES_VERSION="20230311+deb12u1"

apt-get update -qq

apt-get install -qq -y --no-install-recommends \
       "ca-certificates=$CA_CERTIFICATES_VERSION"

rm -rf /var/lib/apt/lists/*

mkdir /var/discordtts
echo '{"voice_settings":{}}' > /var/discordtts/state.json
