from __future__ import annotations

import os
from typing import Any

import httpx


class NeportApiError(RuntimeError):
    def __init__(self, status_code: int, payload: Any) -> None:
        self.status_code = status_code
        self.payload = payload
        super().__init__(f"Neport returned HTTP {status_code}: {payload}")


class NeportClient:
    def __init__(self) -> None:
        base_url = os.environ.get("NEPORT_API_URL", "").strip().rstrip("/")
        token = os.environ.get("NEPORT_API_TOKEN", "").strip()
        if not base_url:
            raise RuntimeError("NEPORT_API_URL is required")
        if not token:
            raise RuntimeError("NEPORT_API_TOKEN is required")
        timeout_ms = int(os.environ.get("NEPORT_MCP_TIMEOUT_MS", "15000"))
        self._client = httpx.AsyncClient(
            base_url=f"{base_url}/api/v1",
            headers={"Authorization": f"Bearer {token}"},
            timeout=timeout_ms / 1000,
        )

    async def close(self) -> None:
        await self._client.aclose()

    async def request(self, method: str, path: str, **kwargs: Any) -> Any:
        try:
            response = await self._client.request(method, path, **kwargs)
        except httpx.HTTPError as exc:
            raise RuntimeError(f"Unable to reach Neport: {exc}") from exc
        try:
            payload = response.json()
        except ValueError:
            payload = response.text
        if not response.is_success:
            raise NeportApiError(response.status_code, payload)
        return payload
