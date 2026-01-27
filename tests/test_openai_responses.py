from __future__ import annotations

from dbeaver_mistral_proxy.openai_responses import (
    build_dbeaver_responses_output_text,
    extract_text_from_dbeaver_responses_input,
)


def test_extract_text_from_dbeaver_responses_input() -> None:
    payload = {
        "input": [
            {
                "type": "message",
                "role": "system",
                "content": [{"type": "input_text", "text": "Rules"}],
            },
            {
                "type": "message",
                "role": "user",
                "content": [{"type": "input_text", "text": "select 1"}],
            },
        ]
    }

    assert extract_text_from_dbeaver_responses_input(payload) == [
        {"role": "system", "content": "Rules"},
        {"role": "user", "content": "select 1"},
    ]


def test_build_dbeaver_responses_output_has_usage_details() -> None:
    out = build_dbeaver_responses_output_text(
        text="ok",
        model="mistral-large-latest",
        input_tokens=1,
        output_tokens=2,
    )

    assert out["usage"]["input_tokens_details"]["cached_tokens"] == 0
    assert out["usage"]["output_tokens_details"]["reasoning_tokens"] == 0
    assert out["output"][0]["content"][0]["type"] == "output_text"
