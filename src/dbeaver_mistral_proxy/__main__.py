from __future__ import annotations

import os

from dotenv import load_dotenv
import uvicorn


def main() -> None:
    load_dotenv()
    host = os.environ.get("HOST", "0.0.0.0")
    port = int(os.environ.get("PORT", "8080"))

    uvicorn.run("dbeaver_mistral_proxy.app:app", host=host, port=port, reload=False)


if __name__ == "__main__":
    main()
