# AGENTS

Policy note:
- Do not leave optional helper scripts, temporary tooling, or "just in case" artifacts in the repository unless the user explicitly asks to keep them.
- Prefer the minimal durable solution needed for the requested outcome.
- Prefer small files with a single responsibility, ideally under 100 lines when practical.
- Prefer fail-fast behavior over fallbacks or best-effort logic: when required state is missing or invalid, stop and surface a clear error immediately.
