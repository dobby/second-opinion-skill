# Binary Releases

Pre-built binaries for the `second-opinion` CLI are placed here.

## Available Binaries

| File | Platform |
|---|---|
| `second-opinion-darwin-arm64` | macOS Apple Silicon (M1/M2/M3) |
| `second-opinion-darwin-x86_64` | macOS Intel |
| `second-opinion-linux-x86_64` | Linux x86_64 |
| `second-opinion-linux-aarch64` | Linux ARM64 |

## Downloading Binaries

```bash
./scripts/refresh-binaries-from-release.sh
```

## Building from Source

```bash
./scripts/build-release-binary.sh
```

Or manually:
```bash
cargo build --release
cp target/release/second-opinion scripts/bin/second-opinion-$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m)
```
