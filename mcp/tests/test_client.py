import pytest

from neport_mcp.client import NeportClient


def test_client_requires_api_url(monkeypatch):
    monkeypatch.delenv("NEPORT_API_URL", raising=False)
    monkeypatch.setenv("NEPORT_API_TOKEN", "npt_test")
    with pytest.raises(RuntimeError, match="NEPORT_API_URL"):
        NeportClient()


def test_client_requires_token(monkeypatch):
    monkeypatch.setenv("NEPORT_API_URL", "https://example.test")
    monkeypatch.delenv("NEPORT_API_TOKEN", raising=False)
    with pytest.raises(RuntimeError, match="NEPORT_API_TOKEN"):
        NeportClient()
