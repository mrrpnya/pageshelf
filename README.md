<div align="center">

<img src="./branding/pageshelf_logo.webp" width="100" alt="Logo"/>

# Pageshelf

A free and open-source Pages server, written in **safe Rust.**

***Overhaul in progress - Full functionality not guaranteed at this time***

![GitHub branch check runs](https://img.shields.io/github/check-runs/mrrpnya/pageshelf/main)
![GitHub License](https://img.shields.io/github/license/mrrpnya/pageshelf)
![Static Badge](https://img.shields.io/badge/unsafe-forbidden-success)

</div>

## Why Pageshelf?

- **Easily hosted** - Tries to be as plug-and-play as possible for any provider instance
- **Written in safe Rust** - Rust is suited for high-performance and reliability
- **Multiple hosting styles** - Subdirectory, subdomain, and custom domains
- **Modular architecture** - Storage and caching capabilities are extendable
- **Open source** - Fully licensed under the MIT License
- **No-JS friendly** - Works on all browsers

## Features

| Feature | Status | Notes |
| :------- | :------: | -------: |
| Dynamic Hosting (Subdirectories)     | ✅   | `example.com/user/repo(:branch)`    |
| Dynamic Hosting (Subdomains)  | ✅  | `((branch).repo).user.example.com`  |
| Custom domains  | ✅  | Works, though some improvements may be in order  |
| Caching | ✅  |  See [Supported software](#supported-software) for more info |
| [Anubis](https://github.com/TecharoHQ/anubis) friendly | ✅ | Running this behind Anubis can help with AI scrapers |
| No-JS friendly | ✅ | There's no JavaScript or WASM to run! |
| Metrics | ⬜ | Coming soon |
| Whitelist/Blacklist | ⬜ | Coming soon |
| Private repos | ⬜ | |
| Plugin system | ⬜ | |
| Authentication locking | ⬜ | |

## Supported software

The core and general structure of the server is being focused on right now, so supported software may not be extensive.

| Software | For | Status | Notes |
| :------- | :------: | -------: | -------: |
| [Forgejo](https://forgejo.org/) | Upstream | ✅ | Uses raw file API for requests, automatically finds pages (plug-and-play) |
| Gitea | Upstream | ⚠️ | Some versions may work as Forgejo is a Gitea fork and thus is similar |
| GitLab | Upstream | ⬜ | Planned |
| [Redis](https://redis.io/) | Cache | ✅  |   |
| [Valkey](https://valkey.io/) | Cache | ✅  | Currently usable via Redis backend, dedicated Valkey backend planned |

## Getting Started

### Quick Start: Containerized Setup

A configuration example can be found [under the `/example` directory.](./example/config.toml)

#### [Docker Compose](./compose.yml)

This includes a Redis setup by default.

```bash
# There is a Docker included with this project - Ensure you have it in your directory
# And, in said  ensure the "build" attribute is commented out and the "image" attribute isn't
docker-compose up -d
```

#### Docker

```bash
sudo docker run --name pageshelf -p 8080:8080 ghcr.io/mrrpnya/pageshelf:canary pageshelf -c [config file]
```

## License

Licensed under the terms of the MIT License. See [LICENSE](./LICENSE) for more information.
