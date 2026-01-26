# Role Catalog

Baseline role prompts for AI agent sessions in docprims.

**Schema**: [`role-prompt.schema.json`](https://schemas.3leaps.dev/agentic/v0/role-prompt.schema.json)

## Available Roles

| Role                                  | Slug       | Category   | Purpose                              |
| ------------------------------------- | ---------- | ---------- | ------------------------------------ |
| [Development Lead](devlead.yaml)      | `devlead`  | agentic    | Implementation, architecture         |
| [Development Reviewer](devrev.yaml)   | `devrev`   | review     | Code review, four-eyes audit         |
| [Enterprise Architect](entarch.yaml)  | `entarch`  | governance | Ecosystem integration                |
| [Security Review](secrev.yaml)        | `secrev`   | review     | Security analysis, input validation  |
| [Product Marketing](prodmktg.yaml)    | `prodmktg` | agentic    | Positioning, messaging               |
| [Release Engineering](releng.yaml)    | `releng`   | automation | Versioning, releases                 |

## Usage

Reference roles by slug in `AGENTS.md`:

```yaml
roles:
  - slug: devlead
    source: config/agentic/roles/devlead.yaml
```

## Schema Validation

All role files conform to the [role-prompt schema](https://schemas.3leaps.dev/agentic/v0/role-prompt.schema.json).

Validate with:

```bash
# Using goneat
goneat schema validate --schema schemas/agentic/v0/role-prompt.schema.json config/agentic/roles/*.yaml
```

## Extending Roles

To extend a baseline role:

```yaml
slug: devlead
extends: https://schemas.3leaps.dev/roles/devlead.yaml
# Add or override fields
scope:
  - ...additional scope items...
```

## docprims-Specific Notes

- **secrev** is particularly important - parsing untrusted documents requires careful input validation
- **entarch** coordinates with Gentry and other consumers
- **prodmktg** positions the GPL-free value proposition
