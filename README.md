# dbeaver-proxy-to-mistral

A small FastAPI proxy that makes **DBeaver CE** (and other OpenAI-compatible clients) work with the **Mistral API**.

DBeaver’s AI assistant speaks an *OpenAI-like* API, but it expects some very specific JSON fields and SSE event types.
This proxy:

- Exposes **OpenAI-compatible** endpoints (`/responses`, `/models`, legacy `/chat/completions`).
- Translates `POST /responses` (OpenAI Responses API shape used by DBeaver) into **Mistral** `POST /chat/completions`.
- Translates Mistral responses back to a payload that DBeaver can parse without `NullPointerException`.
- Supports **streaming** (Server-Sent Events) and non-streaming.
- Is configurable via environment variables and can run as a **systemd** service.

## Supported endpoints

The proxy exposes the following endpoints (both root and `/v1/*` aliases where applicable):

- `GET /models`
- `GET /v1/models`

Returns a list of advertised models (configured via `MISTRAL_MODELS`).

- `POST /responses`
- `POST /v1/responses`

Accepts a DBeaver/OpenAI Responses API request and forwards it to Mistral `chat/completions`.

- `POST /chat/completions`
- `POST /v1/chat/completions`

Legacy pass-through to Mistral `chat/completions` (no format conversion).

## Configuration

Configuration is done via environment variables.

You can use a local `.env` file (loaded via `python-dotenv`) or set variables in your shell/systemd environment.

### Environment variables

Required:

- `MISTRAL_API_KEY`

Optional:

- `MISTRAL_BASE_URL`
  - Default: `https://api.mistral.ai/v1`
- `MISTRAL_MODEL`
  - Default: `mistral-large-latest`
  - Used when the request does not explicitly specify a model.
- `MISTRAL_MODELS`
  - Comma-separated list of model ids that the proxy will **advertise** via `GET /models`.
  - If unset, the proxy advertises `MISTRAL_MODEL`.
- `HOST`
  - Default: `0.0.0.0`
- `PORT`
  - Default: `60916`
- `REQUEST_TIMEOUT_SECONDS`
  - Default: `60`

Notes:

- `GET /models` works even if `MISTRAL_API_KEY` is not set.
- Any route that calls the Mistral API (`/responses`, `/chat/completions`) will return **401** if `MISTRAL_API_KEY` is missing.

## Local development

### Requirements

- Python **3.12+**

### Setup

```bash
python -m venv .venv
. .venv/bin/activate
pip install -r requirements.txt -r requirements-dev.txt
pip install -e .
```

### Configure

Copy `.env.example` to `.env` and fill at least the API key:

```bash
cp .env.example .env
# edit .env
```

### Run

```bash
python -m dbeaver_mistral_proxy
```

The server will listen on `http://0.0.0.0:60916` by default.

### Lint & test

```bash
ruff check .
pytest
```

## Using with DBeaver

In DBeaver CE:

- Set the OpenAI endpoint/base URL to:
  - `http://<your-server>:60916/v1/`
- Set any token value (DBeaver requires one), but the proxy uses `MISTRAL_API_KEY` from the server environment.

DBeaver uses:

- `GET /v1/models`
- `POST /v1/responses`

## systemd service

This repo includes an example unit file and env file template:

- `deploy/systemd/dbeaver-mistral-proxy.service`
- `deploy/systemd/dbeaver-mistral-proxy.env.example`

### Install

1. Create a virtualenv and install deps (see Local development).

2. Create the environment file used by the service:

```bash
cp .env.example /home/cloud/not-safe/github/dbeaver-proxy-to-mistral/.env
# edit the file and set MISTRAL_API_KEY
```

3. Install the unit file:

```bash
sudo cp deploy/systemd/dbeaver-mistral-proxy.service /etc/systemd/system/dbeaver-mistral-proxy.service
sudo systemctl daemon-reload
sudo systemctl enable dbeaver-mistral-proxy
sudo systemctl restart dbeaver-mistral-proxy
```

### Logs

```bash
journalctl -u dbeaver-mistral-proxy -f
```

## Troubleshooting

### DBeaver error: "HTTP/1.1 header parser received no bytes" / "Connection reset"

This generally indicates the proxy failed before writing a response (invalid body, transport edge case, etc.).

Mitigations implemented:

- Robust request parsing for `/responses` and `/chat/completions` (handles empty bodies and `Content-Encoding: gzip`).
- Uvicorn forced to use the `h11` HTTP implementation for better compatibility.

If it still happens, check service logs:

```bash
journalctl -u dbeaver-mistral-proxy -n 200 --no-pager
```

### "Unsupported upgrade request" in logs

This can happen when a client attempts an upgrade (e.g. h2c/websocket-style upgrades). It is expected and harmless for normal DBeaver usage.
