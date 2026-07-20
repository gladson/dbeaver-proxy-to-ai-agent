# DBeaver Proxy — Rust

A high-performance, single-binary HTTP proxy that translates **DBeaver CE**'s OpenAI Responses API calls into standard Chat Completions requests for any AI backend (OpenAI, Mistral, OmniRoute, or any OpenAI-compatible API).

```
╔═════════════════════════════════════════════╗
║             DBeaver Proxy — Rust            ║
║     OpenAI Responses → Chat Completions     ║
║                                             ║
║             Created by Gladson              ║
║           gladsonbrito@gmail.com            ║
╚═════════════════════════════════════════════╝
```

## Features

- **🔄 Protocol translation** — Converts DBeaver's OpenAI Responses API to Chat Completions format
- **🔌 Backend-agnostic** — Works with OmniRoute, OpenAI, Mistral, or any OpenAI-compatible API
- **⚡ Blazing fast** — Built in pure Rust with axum + tokio, single static binary
- **🔧 CLI-first setup** — Interactive `init` wizard creates a local config file
- **📦 Zero dependencies** — Single statically-linked executable, no runtime required
- **📡 Streaming SSE** — Full SSE support (`response.output_text.delta` + `response.completed`)
- **🔐 API key validation** — Validates DBeaver's token against the proxy config
- **📊 Optional metrics** — Lightweight metrics for OmniRoute integration
- **🖥️ Cross-platform** — Pre-built binaries for Linux, Windows, and macOS (Intel + Apple Silicon)

<img width="1240" height="808" alt="image" src="https://github.com/user-attachments/assets/b46fbf67-745a-40e9-bbd1-a5589243062f" />



## Quick Start

### 1. Download

Download the binary for your platform from the [latest release](https://github.com/yourusername/dbeaver-proxy-rust/releases):

| Platform | Binary |
|----------|--------|
| Linux (x86_64) | `dbeaver-proxy-x86_64-linux` |
| Windows (x86_64) | `dbeaver-proxy-x86_64-windows.exe` |
| macOS (Intel + Apple Silicon) | `dbeaver-proxy-macos` |

> **macOS Gatekeeper:** The pre-built macOS binary is ad-hoc signed but not notarized (requires an Apple Developer account). When you first run it, macOS may block it. See [macOS Gatekeeper](#macos-gatekeeper) below to resolve.

### 2. Configure

Run the setup wizard:

```bash
./dbeaver-proxy init
```

This will prompt you for:
- **Backend Base URL** — Your AI backend endpoint (default: `https://api.openai.com/v1`)
- **API Key** — The API key for the backend
- **Default Model** — Model to use (e.g., `gpt-4o`, `g-force2`)

A `dbeaver-proxy.toml` file will be created:

```toml
base_url = "https://api.openai.com/v1"
api_key = "sk-..."
model = "gpt-4o"
```

### 3. Start

```bash
./dbeaver-proxy start
```

The proxy will start on `http://0.0.0.0:60916` and display configuration details:

```
✅ Configuration loaded from: dbeaver-proxy.toml
   Proxy URL:  http://localhost:60916/v1/
   Backend:    https://api.openai.com/v1
   Model:      gpt-4o

🚀 Starting proxy server...
   Configure DBeaver with:
   - Base URL: http://localhost:60916/v1/
   - API Key:  sk-...
   - Model:    gpt-4o
```

### 4. Configure DBeaver

In DBeaver CE:

1. Go to **Window → Preferences → AI**
2. Set the **Base URL** to `http://localhost:60916/v1/`
3. Set the **API Key** to the same value as in `dbeaver-proxy.toml`
4. Select the **Model** (`g-force3`, `gpt-4o`, etc.)
5. Click **Apply and Close**

> **Important:** The API Key in DBeaver must match the `api_key` in `dbeaver-proxy.toml`.

## macOS Gatekeeper

The pre-built macOS binary is **ad-hoc signed** (`codesign -s -`), which satisfies macOS's minimum code signing requirements but does **not** include Apple notarization (requires an Apple Developer account). When you first run it, macOS may display:

> **"dbeaver-proxy-macos" cannot be opened because the developer cannot be verified.**

To resolve this:

### Option 1: Right-click → Open (recommended)

1. Open **Finder** and locate the binary
2. **Right-click** → **Open**
3. Click **Open** in the dialog

This tells macOS to trust the binary for this session.

### Option 2: Remove quarantine attribute

```bash
chmod +x dbeaver-proxy-macos
xattr -d com.apple.quarantine dbeaver-proxy-macos
./dbeaver-proxy-macos init
```

### Option 3: Full notarization (requires Apple Developer account)

If you have an Apple Developer account, you can sign and notarize the binary:

```bash
codesign --force --deep -s "Developer ID Application: Your Name" dbeaver-proxy-macos
xcrun notarytool submit dbeaver-proxy-macos --apple-id your@email.com \
  --team-id YOUR_TEAM_ID --password @keychain:AC_PASSWORD --wait
xcrun stapler staple dbeaver-proxy-macos
```

These steps apply to both Intel and Apple Silicon Macs.

## CLI Reference

```
Usage: dbeaver-proxy <COMMAND>

Commands:
  init   Interactive first-run setup wizard
  start  Start the proxy server
  help   Print this message or the help of the given subcommand(s)
```

### `init`

```bash
dbeaver-proxy init [OPTIONS]
```

| Option | Env Var | Description |
|--------|---------|-------------|
| `--base-url` | `DBEAVER_PROXY_BASE_URL` | Backend base URL |
| `--api-key` | `DBEAVER_PROXY_API_KEY` | API key for the backend |
| `--model` | `DBEAVER_PROXY_MODEL` | Default model |
| `--config-path` | — | Config file path (default: `dbeaver-proxy.toml`) |

### `start`

```bash
dbeaver-proxy start [OPTIONS]
```

| Option | Env Var | Description |
|--------|---------|-------------|
| `--config-path` | — | Config file path (default: `dbeaver-proxy.toml`) |

## Configuration

The proxy configuration is stored in `dbeaver-proxy.toml`:

```toml
base_url = "https://api.openai.com/v1"
api_key = "sk-..."
model = "gpt-4o"
```

### Environment Variables

Environment variables override config file values at runtime:

| Variable | Overrides | Description |
|----------|-----------|-------------|
| `DBEAVER_PROXY_BASE_URL` or `BASE_URL` | `base_url` | Backend URL |
| `DBEAVER_PROXY_API_KEY` or `API_KEY` | `api_key` | Backend API key |
| `DBEAVER_PROXY_MODEL` or `MODEL` | `model` | Default model |
| `HOST` | — | Bind address (default: `0.0.0.0`) |
| `PORT` | — | Listen port (default: `60916`) |
| `LOG_FORMAT` | — | Log format: `text` or `json` (default: `text`) |
| `ENABLE_METRICS` | — | Enable metrics: `true` or `false` (default: `false`) |

## Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/v1/models` | GET | List available models |
| `/v1/responses` | POST | Main translation endpoint (OpenAI Responses API) |
| `/v1/chat/completions` | POST | Legacy passthrough |
| `/health` | GET | Health check with optional metrics |

## Architecture

```
DBeaver CE → dbeaver-proxy (port 60916) → AI Backend (OmniRoute, OpenAI, etc.)
                 │
                 ├─ Translates OpenAI Responses → Chat Completions
                 ├─ Forwards request to backend
                 ├─ Translates response back to Responses format
                 └─ Streams via SSE when requested
```

## Building from Source

### Prerequisites

- Rust 1.81+ ([rustup](https://rustup.rs/))

### Build

```bash
git clone https://github.com/yourusername/dbeaver-proxy-rust
cd dbeaver-proxy-rust
cargo build --release
./target/release/dbeaver-proxy --help
```

### Test

```bash
cargo test
cargo clippy -- -D warnings
```

## CI/CD

The project includes GitHub Actions workflows:

- **`ci.yml`** — Runs on every push/PR to `main`:
  - `cargo fmt --check` + `cargo clippy -- -D warnings`
  - `cargo test`
  - Build for 4 targets: Linux (x86_64), Windows (x86_64), macOS (Intel + Apple Silicon)

- **`release.yml`** — Manual trigger via GitHub Actions (`workflow_dispatch`):
  - Enter version number, builds for all 4 targets
  - Creates macOS Universal Binary via `lipo` + ad-hoc signing
  - Publishes GitHub Release with all binaries + SHA256 checksums

## License

MIT License — see [LICENSE](LICENSE).

Copyright (c) 2026 Gladson Brito
