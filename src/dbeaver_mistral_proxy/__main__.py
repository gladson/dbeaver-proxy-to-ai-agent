from __future__ import annotations

import os

import uvicorn
from dotenv import load_dotenv


def main() -> None:
    load_dotenv()
    host = os.environ.get("HOST", "0.0.0.0")
    port = int(os.environ.get("PORT", "60916"))

    uvicorn.run(
        "dbeaver_mistral_proxy.app:app",
        host=host,
        port=port,
        reload=False,
        http="h11",
    )


if __name__ == "__main__":
    main()
