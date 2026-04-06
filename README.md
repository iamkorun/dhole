<p align="center">
  <h1 align="center">dhole</h1>
  <p align="center">Sniff out every hidden CLI dependency in your project</p>
</p>

<p align="center">
  <a href="https://github.com/iamkorun/dhole/actions/workflows/ci.yml"><img src="https://github.com/iamkorun/dhole/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://crates.io/crates/dhole"><img src="https://img.shields.io/crates/v/dhole.svg" alt="crates.io"></a>
  <a href="https://github.com/iamkorun/dhole/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT"></a>
  <a href="https://github.com/iamkorun/dhole/stargazers"><img src="https://img.shields.io/github/stars/iamkorun/dhole?style=social" alt="Stars"></a>
  <a href="https://buymeacoffee.com/iamkorun"><img src="https://img.shields.io/badge/Buy%20Me%20a%20Coffee-ffdd00?logo=buy-me-a-coffee&logoColor=black" alt="Buy Me a Coffee"></a>
</p>

---

<!-- TODO: Add demo GIF -->

## The Problem

Your project works on your machine. Then a new contributor clones it, runs `make build`, and gets `kubectl: command not found`. Your Makefile calls `docker`, your CI uses `jq`, your deploy script needs `aws` — but none of that is documented anywhere.

Hidden CLI dependencies are the #1 cause of "works on my machine" failures.

## The Solution

**dhole** scans your Makefiles, shell scripts, CI configs, Dockerfiles, and docker-compose files to find every external CLI tool your project depends on. It checks if each tool is installed and reports the version — all in a clean pass/fail table.

Named after the [dhole](https://en.wikipedia.org/wiki/Dhole) — a pack-hunting wild dog that sniffs out hidden prey.

## Demo

```
$ dhole

  Tool             Found in                          Installed  Version
  ────────────    ────────────────────────────────   ─────────  ───────
  aws              deploy.sh                          yes        2.15.30
  cargo            .github/workflows/ci.yml           yes        1.78.0
  curl             Makefile, deploy.sh                yes        8.7.1
  docker           Makefile, docker-compose.yml       yes        26.1.4
  docker-compose   docker-compose.yml                 yes        2.27.0
  helm             Makefile                           NO         —
  jq               deploy.sh                          yes        1.7.1
  kubectl          Makefile                           NO         —
  npm              .github/workflows/ci.yml           yes        10.8.1

  ✗ 7/9 tools installed, 2 missing.
```

## Quick Start

```sh
cargo install dhole
cd your-project/
dhole
```

## Installation

### From crates.io

```sh
cargo install dhole
```

### From source

```sh
git clone https://github.com/iamkorun/dhole.git
cd dhole
cargo install --path .
```

### Binary releases

Pre-built binaries for Linux, macOS, and Windows are available on the [Releases](https://github.com/iamkorun/dhole/releases) page.

## Usage

### Basic scan (current directory)

```sh
dhole
```

### Scan a specific directory

```sh
dhole --dir /path/to/project
```

### Quiet mode (for CI)

```sh
dhole --quiet
# Exit code 0 = all tools found
# Exit code 1 = some tools missing
# Exit code 2 = error (e.g., invalid directory)
```

## Flags

| Flag | Short | Description |
|------|-------|-------------|
| `--dir <path>` | `-d` | Directory to scan (defaults to `.`) |
| `--quiet` | `-q` | Suppress output, only set exit code |
| `--verbose` | `-v` | Show which files are being scanned |
| `--help` | `-h` | Print help |
| `--version` | `-V` | Print version |

## Scanned Files

| File / Pattern | What it catches |
|----------------|-----------------|
| `Makefile`, `GNUmakefile` | Build commands, deploy targets |
| `*.sh` (recursive, max depth 5) | Shell scripts anywhere in the project |
| `.github/workflows/*.yml` | GitHub Actions CI/CD |
| `.gitlab-ci.yml` | GitLab CI/CD |
| `docker-compose.yml` / `.yaml` | Docker Compose services |
| `Dockerfile` | Container build commands |
| `Justfile` | Just command runner |
| `Taskfile.yml` | Task runner |

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | All detected tools are installed |
| `1` | One or more tools are missing |
| `2` | Runtime error (invalid directory, permission denied) |

## CI Integration

Add dhole to your GitHub Actions workflow to catch missing dependencies early:

```yaml
name: Check Dependencies
on: [push, pull_request]

jobs:
  deps:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install dhole
        run: cargo install dhole

      - name: Check CLI dependencies
        run: dhole --quiet
```

## Detected Tools

dhole recognizes 50+ common CLI tools including:

`ansible` `aws` `az` `cargo` `cmake` `curl` `docker` `docker-compose` `ffmpeg` `gcc` `gcloud` `git` `go` `grep` `helm` `jq` `kubectl` `make` `mongosh` `mysql` `node` `npm` `npx` `openssl` `pip` `pnpm` `psql` `python` `redis-cli` `rsync` `rustc` `sed` `ssh` `tar` `terraform` `wget` `yarn` `yq` and more.

## Features

- **Zero config** — point it at a directory and go
- **Smart scanning** — word-boundary matching avoids false positives (`node_modules` won't trigger `node`)
- **Version detection** — tries `--version`, `-v`, and `version` subcommand with a 5s timeout
- **CI-friendly** — `--quiet` mode returns only an exit code
- **Fast** — pure Rust, no regex dependency, scans instantly
- **Cross-source dedup** — a tool found in 3 files shows up once with all sources listed

## Contributing

Contributions are welcome! Please open an issue first to discuss what you'd like to change.

```sh
git clone https://github.com/iamkorun/dhole.git
cd dhole
cargo test
```

## License

[MIT](LICENSE)

---

## Star History

<a href="https://star-history.com/#iamkorun/dhole&Date">
  <img src="https://api.star-history.com/svg?repos=iamkorun/dhole&type=Date" alt="Star History Chart" width="600">
</a>

---

<p align="center">
  <a href="https://buymeacoffee.com/iamkorun"><img src="https://cdn.buymeacoffee.com/buttons/v2/default-yellow.png" alt="Buy Me a Coffee" width="200"></a>
</p>
