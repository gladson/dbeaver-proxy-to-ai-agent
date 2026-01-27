from __future__ import annotations

from pydantic import BaseModel


class Settings(BaseModel):
    mistral_api_key: str
    mistral_base_url: str = "https://api.mistral.ai/v1"
    default_model: str = "mistral-large-latest"
    advertised_models: list[str] = ["mistral-large-latest"]

    request_timeout_seconds: float = 60.0


def load_settings() -> Settings:
    import os

    api_key = os.environ.get("MISTRAL_API_KEY")
    if not api_key:
        raise RuntimeError("MISTRAL_API_KEY is required")

    base_url = os.environ.get("MISTRAL_BASE_URL", "https://api.mistral.ai/v1").rstrip("/")
    default_model = os.environ.get("MISTRAL_MODEL", "mistral-large-latest")

    raw_models = os.environ.get("MISTRAL_MODELS")
    if raw_models:
        advertised_models = [m.strip() for m in raw_models.split(",") if m.strip()]
    else:
        advertised_models = [default_model]

    timeout_str = os.environ.get("REQUEST_TIMEOUT_SECONDS")
    timeout = float(timeout_str) if timeout_str else 60.0

    return Settings(
        mistral_api_key=api_key,
        mistral_base_url=base_url,
        default_model=default_model,
        advertised_models=advertised_models,
        request_timeout_seconds=timeout,
    )
