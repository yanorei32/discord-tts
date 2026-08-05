# Discord TTS

## Example Deployments

The following is a deployment configuration intended for Japanese text‑to‑speech
using only the VOICEVOX Engine as the backend.

By configuring it appropriately, you can enable other engines as well.

### discord-tts.tts.toml

```toml
[default_style]
service_id = "VOICEVOX"
style_id = "0"

[tts_services.VOICEVOX.Voicevox]
url = "http://voicevox:50021"
max_chars = 240
```

### docker-compose.yml

```yaml
services:
  discord-tts:
    image: ghcr.io/yanorei32/discord-tts:master
    restart: unless-stopped

    environment:
      DISCORD_TOKEN: ${DISCORD_TOKEN}

    volumes:
      - type: bind
        source: ./discord-tts.tts.toml
        target: /etc/discord-tts.tts.toml
        read_only: true

      - type: volume
        source: data
        target: /var/discordtts/

  voicevox:
    image: voicevox/voicevox_engine:cpu-latest
    restart: unless-stopped

volumes:
  data:
```

### .env

```
DISCORD_TOKEN=ush9Zohzie6ahmohsoo6meCh.IThah7.jeephaijiachu8kuWoh0aephe5e
```

## Implemented TTS Backends

- VOICEVOX / AivisSpeech
  - Native VOICEVOX API support
- OmniVoice
  - Native OmniVoice API support
- VOICEROID
  - works with https://github.com/yanorei32/aitalked-server
- mirae-tts
  - works with https://github.com/yanorei32/mirae-tts
- kttsproject
  - works with https://github.com/yanorei32/libktts-server
- WinRT
  - works with https://github.com/yanorei32/winrt-tts-server
- macOS say
  - works with https://github.com/yanorei32/say-server
- Android TTS (TextToSpeech API)
  - works with https://github.com/REO2248/android-tts-server
- ⚠ Google Translate
  - based on https://github.com/zlargon/google-tts by [@sim1222](https://github.com/sim1222)
- ⚠ NAVER CLOVA
  - based on https://github.com/scottgigante/NaverTTS by [@REO2248](https://github.com/REO2248)
- ⚠ Bing Speech
  - based on https://github.com/rany2/edge-tts by [@REO2248](https://github.com/REO2248)
- ⚠ CoeFont Try
  - created by [@REO2248](https://github.com/REO2248)
- ⚠ CapCut
  - works with https://github.com/kuwacom/CapCut-TTS
- ⚠ Volcengine Translate

⚠ Non-official API. No warranty and not supported. Use at your own risk!
