from __future__ import annotations

from typing import Any

import httpx

from dbeaver_mistral_proxy.config import Settings


class MistralClient:
    def __init__(self, settings: Settings) -> None:
        self._settings = settings

    def _headers(self) -> dict[str, str]:
        if not self._settings.mistral_api_key:
            raise RuntimeError("MISTRAL_API_KEY is required")
        return {
            "Authorization": f"Bearer {self._settings.mistral_api_key}",
            "Content-Type": "application/json",
        }

    async def chat_completions(self, payload: dict[str, Any]) -> dict[str, Any]:
        url = f"{self._settings.mistral_base_url}/chat/completions"
        timeout = httpx.Timeout(self._settings.request_timeout_seconds)

        async with httpx.AsyncClient(timeout=timeout) as client:
            resp = await client.post(url, headers=self._headers(), json=payload)
            resp.raise_for_status()
            return resp.json()
