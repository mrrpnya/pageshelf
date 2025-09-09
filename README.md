<div align="center">

<img src="./branding/pageshelf_logo.webp" width="100" alt="Logo"/>

# Pageshelf

A free and open-source Pages server, written in Rust.

![GitHub License](https://img.shields.io/github/license/mrrpnya/pageshelf)

</div>

## Supported software

- [x] Forgejo

This project follows a modular design; You can add your own providers, caches, and so on if needed.

## Features

- [x] Dynamic hosting of sites
  - [x] Simple `example.domain/user/repo(:branch))` style subdirectories
  - [x] `((branch).repo).user.example.domain` style subdomains
  - [x] Custom domains
- [x] Integration Tested
  - [x] In-Memory Mock
- [x] Caching Support
  - [x] Redis
- [x] Smart cache invalidation
- [ ] Metrics
- [ ] Security
  - [ ] Whitelist/Blacklist
  - [ ] Auth-locking specific pages
  - [ ] Private repo serving

## License

Licensed under the terms of the MIT License. See [LICENSE](./LICENSE) for more information.
