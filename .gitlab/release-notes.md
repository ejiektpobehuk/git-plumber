## What's Changed

CHANGELOG material

## Download ️

### Release binaries
Choose the appropriate binary for your platform:
- **Linux x86_64**: `git-plumber-linux-x86_64`
- **Linux x86_64 (static)**: `git-plumber-linux-x86_64-static` _(fully static, no dependencies)_
- **Linux ARM64**: `git-plumber-linux-aarch64`
- **Linux ARM64 (static)**: `git-plumber-linux-aarch64-static` _(fully static, no dependencies)_
- **Windows x86_64**: `git-plumber-windows-x86_64.exe`
- **Windows ARM64**: temporary unavailable
- **macOS x86_64**: temporary unavailable
- **macOS ARM64**: temporary unavailable

### Container Image
Available at [docker hub](https://hub.docker.com/r/ejiek/git-plumber/tags?name=v0.1.0) and [ghcr](https://github.com/ejiektpobehuk/git-plumber/pkgs/container/git-plumber) as a minimal Docker image (<1MB) using static binaries on scratch base.
Supports `x86_64` and `ARM64` architectures.


```
docker run --rm -v $(pwd):/workspace ejiek/git-plumber:$CI_COMMIT_TAG
```
```
docker run --rm -v $(pwd):/workspace ghcr.io/ejiektpobehuk/git-plumber:$CI_COMMIT_TAG
```

### Crates.io
Available at [crates.io](https://crates.io/crates/git-plumber) as a Rust crate:

```
cargo install git-plumber
```
To install this specific version, use:
```
cargo install git-plumber --version $CI_COMMIT_TAG
```

### Nix Flake

```
nix run github:ejiektpobehuk/git-plumber/$CI_COMMIT_TAG
```
