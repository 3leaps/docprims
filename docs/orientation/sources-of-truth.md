# Sources of Truth

docprims intentionally separates canonical project intent from planning artifacts.
This keeps the repo stable, reviewable, and safe for security-sensitive parser work.

### Canonical (in-repo)

- `README.md`: user-facing overview, supported surfaces, and dev commands
- `MAINTAINERS.md`: ownership, governance, and escalation
- `docs/architecture/`: stable boundaries + contracts (not a roadmap)
- `docs/decisions/`: ADR/SDR/DDR records (why we made a choice)
- `schemas/`: machine contracts (JSON schema, versioning)
- `config/agentic/roles/`: role definitions + operating constraints for agents

### Tactical (local-only)

- `AGENTS.local.md` (gitignored): session/sprint guidance from maintainers
- `.plans/` (gitignored): personal notes, scratch plans, prompts, roadmaps

### Out-of-band (OOB, non-canonical)

Planning systems are useful context, but they are not source-of-truth:

- roadmaps, backlogs, kanban boards
- issues/tickets, PR descriptions (unless information is promoted into docs)
- chat logs, meeting notes

If something in OOB changes how the library behaves or what it promises, promote it
into the canonical set above (usually an ADR + schema/docs updates).
