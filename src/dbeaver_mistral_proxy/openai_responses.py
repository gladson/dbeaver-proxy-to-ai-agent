from __future__ import annotations

import time
from typing import Any


def extract_text_from_dbeaver_responses_input(payload: dict[str, Any]) -> list[dict[str, str]]:
    messages: list[dict[str, str]] = []

    for msg in payload.get("input", []) or []:
        role = msg.get("role")
        if role not in {"system", "user", "assistant"}:
            role = "user"

        parts = msg.get("content") or []
        text = "".join((p or {}).get("text", "") for p in parts if isinstance(p, dict))
        if text:
            messages.append({"role": role, "content": text})

    return messages


def build_dbeaver_responses_output_text(
    *,
    text: str,
    model: str,
    input_tokens: int = 0,
    output_tokens: int = 0,
) -> dict[str, Any]:
    now = int(time.time())
    return {
        "id": f"resp_{now}",
        "object": "response",
        "created": now,
        "model": model,
        "output": [
            {
                "id": f"msg_{now}",
                "type": "message",
                "status": "completed",
                "role": "assistant",
                "content": [
                    {
                        "type": "output_text",
                        "text": text,
                        "annotations": None,
                        "logprobs": None,
                    }
                ],
            }
        ],
        "usage": {
            "input_tokens": input_tokens,
            "input_tokens_details": {"cached_tokens": 0},
            "output_tokens": output_tokens,
            "output_tokens_details": {"reasoning_tokens": 0},
        },
    }


def build_models_response(models: list[str]) -> dict[str, Any]:
    now = int(time.time())
    return {
        "object": "list",
        "data": [
            {
                "id": model,
                "object": "model",
                "created": now,
                "ownedBy": "dbeaver-mistral-proxy",
            }
            for model in models
        ],
    }
