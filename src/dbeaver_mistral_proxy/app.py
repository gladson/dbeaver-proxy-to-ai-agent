from __future__ import annotations

import gzip
import json
import logging
from collections.abc import AsyncIterator
from typing import Any

from dotenv import load_dotenv
from fastapi import FastAPI, Request
from fastapi.responses import JSONResponse, StreamingResponse

from dbeaver_mistral_proxy.config import MissingConfigError, load_settings
from dbeaver_mistral_proxy.mistral_client import MistralClient
from dbeaver_mistral_proxy.openai_responses import (
    build_dbeaver_responses_output_text,
    build_models_response,
    extract_text_from_dbeaver_responses_input,
)

load_dotenv()

app = FastAPI()

log = logging.getLogger("dbeaver_mistral_proxy")


async def read_json_payload(request: Request) -> dict[str, Any] | None:
    body = await request.body()
    if not body:
        log.warning(
            "empty request body method=%s path=%s content-length=%s content-encoding=%s expect=%s",
            request.method,
            request.url.path,
            request.headers.get("content-length"),
            request.headers.get("content-encoding"),
            request.headers.get("expect"),
        )
        return None

    content_encoding = (request.headers.get("content-encoding") or "").lower()
    if "gzip" in content_encoding:
        try:
            body = gzip.decompress(body)
        except OSError:
            log.warning(
                "gzip decompress failed method=%s path=%s content-length=%s",
                request.method,
                request.url.path,
                request.headers.get("content-length"),
            )
            return None

    try:
        text = body.decode("utf-8")
    except UnicodeDecodeError:
        log.warning(
            "utf-8 decode failed method=%s path=%s content-length=%s",
            request.method,
            request.url.path,
            request.headers.get("content-length"),
        )
        return None

    text = text.strip()
    if not text:
        return None

    try:
        payload = json.loads(text)
    except json.JSONDecodeError:
        log.warning(
            "json decode failed method=%s path=%s content-length=%s",
            request.method,
            request.url.path,
            request.headers.get("content-length"),
        )
        return None

    if not isinstance(payload, dict):
        return None

    return payload


@app.get("/models")
@app.get("/v1/models")
async def list_models() -> dict[str, Any]:
    settings = load_settings(require_api_key=False)
    return build_models_response(settings.advertised_models)


@app.post("/responses")
@app.post("/v1/responses")
async def responses(request: Request):
    try:
        settings = load_settings(require_api_key=True)
    except MissingConfigError as exc:
        return JSONResponse(status_code=401, content={"error": {"message": str(exc)}})
    mistral = MistralClient(settings)

    payload = await read_json_payload(request)
    if payload is None:
        return JSONResponse(
            status_code=400,
            content={
                "error": {
                    "message": "Invalid request body: expected JSON object"
                }
            },
        )

    model = payload.get("model") or settings.default_model
    messages = extract_text_from_dbeaver_responses_input(payload)

    stream = bool(payload.get("stream"))

    mistral_payload: dict[str, Any] = {
        "model": model,
        "messages": messages,
        "temperature": payload.get("temperature"),
        "stream": False,
        "tools": payload.get("tools"),
        "tool_choice": payload.get("tool_choice"),
    }

    mistral_payload = {k: v for k, v in mistral_payload.items() if v is not None}

    mistral_resp = await mistral.chat_completions(mistral_payload)

    text = (
        (((mistral_resp.get("choices") or [])[0] or {}).get("message") or {}).get("content")
        or ""
    )

    usage = mistral_resp.get("usage") or {}
    input_tokens = int(usage.get("prompt_tokens") or 0)
    output_tokens = int(usage.get("completion_tokens") or 0)

    response_json = build_dbeaver_responses_output_text(
        text=text,
        model=model,
        input_tokens=input_tokens,
        output_tokens=output_tokens,
    )

    if not stream:
        return JSONResponse(response_json)

    async def event_stream() -> AsyncIterator[str]:
        yield "event: response.output_text.delta\n"
        yield "data: " + json.dumps({"type": "response.output_text.delta", "delta": text}) + "\n\n"

        yield "event: response.completed\n"
        yield "data: " + json.dumps(
            {
                "type": "response.completed",
                "sequence_number": 1,
                "response": response_json,
            }
        ) + "\n\n"

    return StreamingResponse(event_stream(), media_type="text/event-stream")


@app.post("/chat/completions")
@app.post("/v1/chat/completions")
async def chat_completions(request: Request) -> JSONResponse:
    try:
        settings = load_settings(require_api_key=True)
    except MissingConfigError as exc:
        return JSONResponse(status_code=401, content={"error": {"message": str(exc)}})
    mistral = MistralClient(settings)

    payload = await read_json_payload(request)
    if payload is None:
        return JSONResponse(
            status_code=400,
            content={"error": {"message": "Invalid request body: expected JSON object"}},
        )
    model = payload.get("model") or settings.default_model

    mistral_payload: dict[str, Any] = {
        **payload,
        "model": model,
    }
    mistral_resp = await mistral.chat_completions(mistral_payload)
    return JSONResponse(mistral_resp)


@app.exception_handler(Exception)
async def unhandled_exception_handler(_: Request, exc: Exception) -> JSONResponse:
    return JSONResponse(
        status_code=500,
        content={"error": {"message": str(exc)}},
    )
