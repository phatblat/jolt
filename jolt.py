#!/usr/bin/env python3
"""
jolt - JOb Log Ticket

Collect and search GitHub Actions workflow logs to extract interesting sections for tickets.

Usage:
    jolt --repo owner/repo [--workflow NAME] [--pr NUMBER] [--limit N]

Requirements:
    pip install requests click rich

Environment:
    GITHUB_TOKEN - GitHub personal access token with repo scope
"""

import sys
from datetime import datetime, timezone

import click
import requests
from rich.console import Console
from rich.panel import Panel
from rich.table import Table
from rich.text import Text

console = Console()


class GitHubClient:
    def __init__(self, token: str):
        self.token = token
        self.base_url = "https://api.github.com"
        self.session = requests.Session()
        self.session.headers.update(
            {
                "Authorization": f"Bearer {token}",
                "Accept": "application/vnd.github+json",
                "X-GitHub-Api-Version": "2022-11-28",
            }
        )

    def _get(self, endpoint: str, params: dict = None) -> dict:
        resp = self.session.get(f"{self.base_url}{endpoint}", params=params)
        resp.raise_for_status()
        return resp.json()

    def get_workflow_runs(
        self,
        owner: str,
        repo: str,
        workflow_name: str | None = None,
        status: str = "failure",
        per_page: int = 20,
    ) -> list[dict]:
        """Fetch workflow runs, optionally filtered by workflow name."""
        params = {"status": status, "per_page": per_page}

        runs = self._get(f"/repos/{owner}/{repo}/actions/runs", params)
        workflow_runs = runs.get("workflow_runs", [])

        if workflow_name:
            workflow_name_lower = workflow_name.lower()
            workflow_runs = [
                r for r in workflow_runs if workflow_name_lower in r["name"].lower()
            ]

        return workflow_runs

    def get_pr_workflow_runs(
        self, owner: str, repo: str, pr_number: int, status: str = "failure"
    ) -> list[dict]:
        """Fetch workflow runs for a specific PR."""
        # Get PR details to find the head SHA
        pr = self._get(f"/repos/{owner}/{repo}/pulls/{pr_number}")
        head_sha = pr["head"]["sha"]
        head_branch = pr["head"]["ref"]

        # Get check runs for this commit
        params = {"status": status, "branch": head_branch, "per_page": 50}
        runs = self._get(f"/repos/{owner}/{repo}/actions/runs", params)

        # Filter to runs associated with this PR
        pr_runs = [
            r
            for r in runs.get("workflow_runs", [])
            if r["head_sha"] == head_sha
            or any(
                pr_ref.get("number") == pr_number
                for pr_ref in r.get("pull_requests", [])
            )
        ]

        return pr_runs

    def get_jobs_for_run(self, owner: str, repo: str, run_id: int) -> list[dict]:
        """Fetch jobs for a specific workflow run."""
        jobs = self._get(f"/repos/{owner}/{repo}/actions/runs/{run_id}/jobs")
        return jobs.get("jobs", [])

    def get_failed_jobs(self, owner: str, repo: str, run_id: int) -> list[dict]:
        """Fetch only failed jobs for a workflow run."""
        jobs = self.get_jobs_for_run(owner, repo, run_id)
        return [j for j in jobs if j["conclusion"] == "failure"]


def format_time_ago(dt_str: str) -> str:
    """Convert ISO timestamp to human-readable 'time ago' format."""
    dt = datetime.fromisoformat(dt_str.replace("Z", "+00:00"))
    now = datetime.now(timezone.utc)
    delta = now - dt

    if delta.days > 0:
        return f"{delta.days}d ago"
    elif delta.seconds >= 3600:
        return f"{delta.seconds // 3600}h ago"
    elif delta.seconds >= 60:
        return f"{delta.seconds // 60}m ago"
    else:
        return "just now"


def display_failures(
    workflow_runs: list[dict], client: GitHubClient, owner: str, repo: str
):
    """Display workflow failures in a rich table."""
    if not workflow_runs:
        console.print("[yellow]No failed workflow runs found.[/yellow]")
        return

    for run in workflow_runs:
        failed_jobs = client.get_failed_jobs(owner, repo, run["id"])

        if not failed_jobs:
            continue

        # Create panel for each workflow run
        run_title = (
            f"[bold red]✗[/bold red] {run['name']} [dim]#{run['run_number']}[/dim]"
        )

        run_info = Text()
        run_info.append("Branch: ", style="dim")
        run_info.append(f"{run['head_branch']}\n", style="cyan")
        run_info.append("Commit: ", style="dim")
        run_info.append(f"{run['head_sha'][:7]}\n", style="yellow")

        # Add PR number if available
        if run.get("pull_requests"):
            pr_numbers = [str(pr["number"]) for pr in run["pull_requests"]]
            run_info.append("PR: ", style="dim")
            run_info.append(f"#{', #'.join(pr_numbers)}\n", style="magenta")

        run_info.append("Started: ", style="dim")
        run_info.append(f"{format_time_ago(run['created_at'])}\n", style="white")
        run_info.append("URL: ", style="dim")
        run_info.append(f"{run['html_url']}", style="blue underline")

        console.print(Panel(run_info, title=run_title, border_style="red"))

        # Table of failed jobs
        table = Table(show_header=True, header_style="bold", box=None, padding=(0, 2))
        table.add_column("Job", style="white")
        table.add_column("Duration", style="dim")
        table.add_column("Failed Step", style="red")
        table.add_column("Job URL", style="blue")

        for job in failed_jobs:
            # Find the failed step
            failed_step = next(
                (
                    s["name"]
                    for s in job.get("steps", [])
                    if s["conclusion"] == "failure"
                ),
                "Unknown",
            )

            # Calculate duration
            if job.get("started_at") and job.get("completed_at"):
                start = datetime.fromisoformat(job["started_at"].replace("Z", "+00:00"))
                end = datetime.fromisoformat(job["completed_at"].replace("Z", "+00:00"))
                duration = str(end - start).split(".")[0]  # Remove microseconds
            else:
                duration = "-"

            table.add_row(job["name"], duration, failed_step, job["html_url"])

        console.print(table)
        console.print()


@click.command()
@click.option(
    "--repo", "-r", required=True, help="GitHub repository in 'owner/repo' format"
)
@click.option(
    "--workflow", "-w", default=None, help="Filter by workflow name (partial match)"
)
@click.option("--pr", "-p", type=int, default=None, help="Filter by PR number")
@click.option(
    "--limit",
    "-l",
    type=int,
    default=10,
    help="Maximum number of workflow runs to fetch (default: 10)",
)
@click.option(
    "--token",
    "-t",
    envvar="GITHUB_TOKEN",
    help="GitHub token (or set GITHUB_TOKEN env var)",
)
def main(repo: str, workflow: str | None, pr: int | None, limit: int, token: str):
    """
    Collect and search GitHub Actions workflow logs to extract interesting sections for tickets.

    Examples:

        jolt --repo myorg/myrepo --workflow "CI"

        jolt --repo myorg/myrepo --pr 1234

        jolt -r myorg/myrepo -w "Build" -l 5
    """
    if not token:
        console.print(
            "[red]Error:[/red] GitHub token required. Set GITHUB_TOKEN or use --token"
        )
        sys.exit(1)

    try:
        owner, repo_name = repo.split("/")
    except ValueError:
        console.print("[red]Error:[/red] Repository must be in 'owner/repo' format")
        sys.exit(1)

    client = GitHubClient(token)

    console.print(
        f"\n[bold]Fetching failed workflow runs for [cyan]{repo}[/cyan]...[/bold]\n"
    )

    try:
        if pr:
            console.print(f"[dim]Filtering by PR #{pr}[/dim]")
            runs = client.get_pr_workflow_runs(owner, repo_name, pr)
        else:
            runs = client.get_workflow_runs(
                owner, repo_name, workflow_name=workflow, per_page=limit
            )

        if workflow:
            console.print(f"[dim]Filtering by workflow: {workflow}[/dim]")

        display_failures(runs[:limit], client, owner, repo_name)

    except requests.exceptions.HTTPError as e:
        if e.response.status_code == 404:
            console.print(
                f"[red]Error:[/red] Repository '{repo}' not found or not accessible"
            )
        elif e.response.status_code == 401:
            console.print("[red]Error:[/red] Invalid or expired GitHub token")
        else:
            console.print(f"[red]Error:[/red] GitHub API error: {e}")
        sys.exit(1)
    except Exception as e:
        console.print(f"[red]Error:[/red] {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()
