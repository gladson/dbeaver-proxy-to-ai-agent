from __future__ import annotations

import json
from typing import Any, AsyncIterator

from dotenv import load_dotenv
from fastapi import FastAPI, Request
from fastapi.responses import JSONResponse
from fastapi.responses import StreamingResponse
from dbeaver_mistral_proxy.config import load_settings
from dbeaver_mistral_proxy.mistral_client import MistralClient
from dbeaver_mistral_proxy.openai_responses import (
    build_dbeaver_responses_output_text,
    build_models_response,
    extract_text_from_dbeaver_responses_input,
)

load_dotenv()

app = FastAPI()


@app.get("/models")
@app.get("/v1/models")
async def list_models() -> dict[str, Any]:
    settings = load_settings()
    return build_models_response(settings.advertised_models)


@app.post("/responses")
@app.post("/v1/responses")
async def responses(request: Request):
    settings = load_settings()
    mistral = MistralClient(settings)

    payload: dict[str, Any] = await request.json()

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
    settings = load_settings()
    mistral = MistralClient(settings)

    payload: dict[str, Any] = await request.json()
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
