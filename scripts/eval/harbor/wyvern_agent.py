"""wyvern_agent.py — Harbor installed-agent adapter for the `wyvern` binary.

Drives the flight-tier worker on real, containerized Terminal-Bench tasks. It
injects the locally-built `wyvern` binary into each task container and runs it
headless in `/app`, writing the trace to `/logs/agent`. The binary is injected
(not package-installed) only because wyvern has no public release yet.

Run it:

    WYVERN_BENCH_BIN=target/release/wyvern \\
    WYVERN_BENCH_ENDPOINT=http://host:8080 \\
    WYVERN_BENCH_MODEL=some-model \\
    WYVERN_BENCH_CONTEXT_WINDOW=65536 \\
    PYTHONPATH=scripts/eval/harbor \\
    harbor run --config <job.json>

The endpoint is host-secret: it lives ONLY in WYVERN_BENCH_ENDPOINT (a local
env), never in this file or the job config.
"""

from __future__ import annotations

import os
import shlex
import tempfile
from typing import override

from harbor.agents.installed.base import BaseInstalledAgent, with_prompt_template
from harbor.environments.base import BaseEnvironment
from harbor.models.agent.context import AgentContext

_WYVERN_BIN = os.environ.get("WYVERN_BENCH_BIN", os.path.expanduser("~/bin/wyvern"))
_ENDPOINT = os.environ.get("WYVERN_BENCH_ENDPOINT", "")
_MODEL = os.environ.get("WYVERN_BENCH_MODEL", "")
_MAX_ROUNDS = os.environ.get("WYVERN_BENCH_MAX_ROUNDS", "40")
_CONTEXT_WINDOW = os.environ.get("WYVERN_BENCH_CONTEXT_WINDOW", "65536")


class WyvernAgent(BaseInstalledAgent):
    """Fly `wyvern` inside a Harbor task container."""

    @staticmethod
    @override
    def name() -> str:
        return "wyvern"

    @override
    def get_version_command(self) -> str | None:
        return "wyvern --help >/dev/null 2>&1; echo wyvern-0.1"

    @override
    def parse_version(self, stdout: str) -> str:
        return stdout.strip().splitlines()[-1] if stdout.strip() else "wyvern"

    @override
    async def install(self, environment: BaseEnvironment) -> None:
        if not _ENDPOINT or not _MODEL:
            raise ValueError(
                "WYVERN_BENCH_ENDPOINT and WYVERN_BENCH_MODEL must be set "
                "(the host-secret endpoint + the served model)."
            )
        await environment.upload_file(_WYVERN_BIN, "/usr/local/bin/wyvern")
        await self.exec_as_root(environment, command="chmod 0755 /usr/local/bin/wyvern")

    @override
    @with_prompt_template
    async def run(
        self,
        instruction: str,
        environment: BaseEnvironment,
        context: AgentContext,
    ) -> None:
        with tempfile.NamedTemporaryFile("w", suffix=".md", delete=False) as fh:
            fh.write(instruction)
            local_instr = fh.name
        try:
            await environment.upload_file(local_instr, "/tmp/wyvern-task.md")
        finally:
            os.unlink(local_instr)

        ctx = f" --context-window {shlex.quote(_CONTEXT_WINDOW)}" if _CONTEXT_WINDOW else ""
        command = (
            "mkdir -p /logs/agent; "
            "wyvern --cwd /app "
            f"--endpoint {shlex.quote(_ENDPOINT)} "
            f"--model {shlex.quote(_MODEL)} "
            "--instruction-file /tmp/wyvern-task.md "
            "--events /logs/agent/wyvern-events.jsonl "
            f"--max-rounds {shlex.quote(_MAX_ROUNDS)}{ctx}"
        )
        await self.exec_as_agent(environment, command=command)
