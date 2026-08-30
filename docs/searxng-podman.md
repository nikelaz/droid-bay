# Running SearXNG with podman

The `web_search` tool in `sdk/websearch.go` talks to a local SearXNG instance at `http://127.0.0.1:8888` (override with `SEARXNG_URL`). SearXNG must be running before any agent starts, since the SDK polls `/search?q=ping&format=json` until it responds with `200`.

This guide covers a rootless podman setup on Linux with the JSON API enabled, persistent config, and automatic startup at boot.

## 1. Install

On Fedora:

```sh
sudo dnf install podman podman-compose
```

## 2. Directory layout

```
~/searxng/
├── docker-compose.yml
├── .env
└── core-config/
    └── settings.yml
```

`docker-compose.yml`:

```yaml
name: searxng

services:
  core:
    container_name: searxng-core
    image: docker.io/searxng/searxng:${SEARXNG_VERSION:-latest}
    restart: always
    ports:
      - "${SEARXNG_PORT:-8080}:${SEARXNG_PORT:-8080}"
    volumes:
      - ./core-config/:/etc/searxng/:Z
      - core-data:/var/cache/searxng/

  valkey:
    container_name: searxng-valkey
    image: docker.io/valkey/valkey:9-alpine
    command: valkey-server --save 30 1 --loglevel warning
    restart: always
    volumes:
      - valkey-data:/data/

volumes:
  core-data:
  valkey-data:
```

`.env`:

```
SEARXNG_VERSION=latest
SEARXNG_PORT=8888
```

The `./core-config/` bind mount is what makes the configuration persistent. Without it, any change made inside the container is lost as soon as the container is recreated.

## 3. Enable the JSON API

By default SearXNG only serves HTML and rejects `format=json` with a `403`. Create `core-config/settings.yml`:

```yaml
use_default_settings: true

server:
  secret_key: "<generate with: openssl rand -hex 32>"
  image_proxy: true

search:
  formats:
    - html
    - json
```

## 4. Start

```sh
cd ~/searxng
podman-compose up -d
```

Verify:

```sh
curl -s "http://127.0.0.1:8888/search?q=test&format=json"
```

A `403` here means the JSON format is not enabled in `settings.yml`. A JSON body with a `results` array means you are done.

## 5. Start at boot

The `restart: always` policy in the compose file is not enough for rootless podman: it only restarts containers while your session is running, nothing brings them back after a reboot. A systemd user unit takes care of that. First enable lingering so user units run at boot without logging in:

```sh
loginctl enable-linger $USER
```

Then create `~/.config/systemd/user/searxng.service`:

```ini
[Unit]
Description=SearXNG compose stack (podman-compose)
After=podman.socket
Requires=podman.socket

[Service]
Type=oneshot
RemainAfterExit=yes
WorkingDirectory=%h/searxng
ExecStart=/usr/bin/podman-compose up -d
ExecStop=/usr/bin/podman-compose stop
TimeoutStartSec=120

[Install]
WantedBy=default.target
```

Enable it:

```sh
systemctl --user daemon-reload
systemctl --user enable --now searxng.service
```

Manage the stack with:

```sh
systemctl --user status searxng
systemctl --user restart searxng
journalctl --user -u searxng -f
```

## Troubleshooting

- **`403` on `/search?format=json`** - add `json` under `search.formats` in `settings.yml` and restart with `systemctl --user restart searxng`.
- **Changes to `settings.yml` have no effect** - restart the stack. The file is only read at startup.
- **`SEARXNG_URL`** - the SDK uses `http://127.0.0.1:8888` unless this environment variable is set.
