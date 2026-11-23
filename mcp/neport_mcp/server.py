from __future__ import annotations

import logging
import os
from typing import Any

from mcp.server.fastmcp import FastMCP

from .client import NeportClient

logging.basicConfig(
    level=os.environ.get("NEPORT_MCP_LOG_LEVEL", "WARNING"),
    stream=__import__("sys").stderr,
    format="%(levelname)s %(name)s: %(message)s",
)
log = logging.getLogger("neport-mcp")


mcp = FastMCP("Neport")


def client() -> NeportClient:
    # FastMCP creates one process-local server and the lifespan client is
    # intentionally created lazily for compatibility with SDK 2.x hosts.
    if not hasattr(client, "_instance"):
        client._instance = NeportClient()  # type: ignore[attr-defined]
    return client._instance  # type: ignore[attr-defined]


async def call(method: str, path: str, **kwargs: Any) -> str:
    import json

    result = await client().request(method, path, **kwargs)
    return json.dumps(result, ensure_ascii=False, indent=2, default=str)


@mcp.tool()
async def search_projects(q: str | None = None, limit: int | None = None) -> str:
    """[READ ONLY] Find projects visible to the authenticated user. Use before project or ticket operations."""
    params = {k: v for k, v in {"q": q, "limit": limit}.items() if v is not None}
    return await call("GET", "/projects/", params=params)


@mcp.tool()
async def get_project(id: int) -> str:
    """[READ ONLY] Get one project by ID. Requires project visibility."""
    return await call("GET", f"/projects/{id}")


@mcp.tool()
async def create_project(name: str, description: str | None = None) -> str:
    """[MUTATION] Create a project. Confirm the name and description first; administrator permission is required."""
    return await call("POST", "/projects/", json={"name": name, "description": description})


@mcp.tool()
async def list_project_members(project_id: int) -> str:
    """[READ ONLY] List members and roles for a project."""
    return await call("GET", f"/projects/{project_id}/members/")


@mcp.tool()
async def add_project_member(project_id: int, user_id: int, role: str) -> str:
    """[MUTATION] Add a user to a project. Confirm user ID and role; project management permission is required."""
    return await call("POST", f"/projects/{project_id}/members/", json={"user_id": user_id, "role": role})


@mcp.tool()
async def search_tickets(q: str, project_id: int | None = None) -> str:
    """[READ ONLY] Search visible tickets by text, optionally within one project."""
    params = {"q": q} if project_id is None else {"q": q, "project_id": project_id}
    return await call("GET", "/tickets/search", params=params)


@mcp.tool()
async def get_ticket(ticket_id: int) -> str:
    """[READ ONLY] Get a ticket by ID, including status and project context."""
    return await call("GET", f"/tickets/{ticket_id}")


@mcp.tool()
async def create_ticket(project_id: int, title: str, description: str | None = None, assignee_id: int | None = None) -> str:
    """[MUTATION] Create a ticket. Confirm project, title, and assignee before creating persistent data."""
    body = {"project_id": project_id, "title": title, "description": description, "assignee_id": assignee_id}
    return await call("POST", "/tickets/", json=body)


@mcp.tool()
async def update_ticket(
    ticket_id: int,
    title: str | None = None,
    description: str | None = None,
    assignee_id: int | None = None,
    expected_updated_at: str | None = None,
) -> str:
    """[MUTATION] Update editable fields. Pass updated_at from get_ticket to detect conflicts.
    On HTTP 409, fetch the latest ticket and review the changes before retrying."""
    fields = {
        "title": title,
        "description": description,
        "assignee_id": assignee_id,
        "expected_updated_at": expected_updated_at,
    }
    return await call("PATCH", f"/tickets/{ticket_id}", json={k: v for k, v in fields.items() if v is not None})


@mcp.tool()
async def add_comment(ticket_id: int, body: str) -> str:
    """[MUTATION] Add an audited comment visible to project members. Confirm the text before posting."""
    return await call("POST", f"/tickets/{ticket_id}/comments/", json={"body": body})


@mcp.tool()
async def add_dependency(ticket_id: int, depends_on_ticket_id: int) -> str:
    """[MUTATION] Declare that this ticket needs another ticket in the same project to finish first."""
    return await call(
        "POST",
        f"/tickets/{ticket_id}/dependencies",
        json={"direction": "predecessor", "other_ticket_id": depends_on_ticket_id},
    )


@mcp.tool()
async def start_work_session() -> str:
    """[MUTATION] Start the authenticated user's work timer. Only one open session is allowed."""
    return await call("POST", "/work-sessions/start")


@mcp.tool()
async def end_work_session() -> str:
    """[MUTATION] End the authenticated user's current work timer. Confirm before stopping it."""
    return await call("POST", "/work-sessions/end")


@mcp.tool()
async def get_current_work_session() -> str:
    """[READ ONLY] Get the authenticated user's currently open work session."""
    return await call("GET", "/work-sessions/current")


@mcp.tool()
async def get_daily_report(date: str) -> str:
    """[READ ONLY] Get a daily report by ISO date (YYYY-MM-DD)."""
    return await call("GET", f"/daily-reports/{date}")


@mcp.tool()
async def submit_daily_report(report_date: str | None = None, achievement: str | None = None, leftover: str | None = None, keep: str | None = None, problem: str | None = None, try_: str | None = None) -> str:
    """[MUTATION] Submit a daily report. Closed or approved months cannot be edited; confirm content first."""
    body = {"report_date": report_date, "achievement": achievement, "leftover": leftover, "keep": keep, "problem": problem, "try": try_}
    return await call("POST", "/daily-reports/", json={k: v for k, v in body.items() if v is not None})


@mcp.tool()
async def get_pending_approvals() -> str:
    """[READ ONLY] List subordinates and pending approval workload for the authenticated approver."""
    return await call("GET", "/approvals/subordinates")


@mcp.tool()
async def approve_daily_report(user_id: int, date: str, decision: str, comment: str = "") -> str:
    """[MUTATION] Approve or reject a subordinate daily report. Confirm employee, date, decision, and comment."""
    return await call("POST", f"/approvals/subordinates/{user_id}/day/{date}/decide", json={"decision": decision, "comment": comment})


@mcp.tool()
async def close_month(user_id: int, year: int, month: int) -> str:
    """[MUTATION] Close a subordinate's month. This locks reports and work sessions; confirm all approvals first."""
    return await call("POST", f"/approvals/subordinates/{user_id}/month/{year}/{month}/close")


def main() -> None:
    mcp.run(transport="stdio")


if __name__ == "__main__":
    main()
