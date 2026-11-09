# Decision Records

This directory contains architectural, design, and security decision records for docprims.

## Record Types

| Prefix | Type                          | Purpose                              |
| ------ | ----------------------------- | ------------------------------------ |
| ADR    | Architecture Decision Record  | Technical architecture choices       |
| DDR    | Design Decision Record        | API design, data structures          |
| SDR    | Security Decision Record      | Security-related decisions           |

## Index

### Architecture Decision Records (ADR)

| ID       | Title                                               | Status   | Date       |
| -------- | --------------------------------------------------- | -------- | ---------- |
| [ADR-0001](ADR-0001-license-policy.md) | License Policy | Accepted | 2025-01-25 |
| [ADR-0002](ADR-0002-crate-structure.md) | Crate Structure | Superseded by ADR-0007 | 2025-01-25 |
| [ADR-0003](ADR-0003-input-validation-policy.md) | Input Validation Policy | Accepted | 2025-01-25 |
| [ADR-0004](ADR-0004-stdout-purity.md) | Stdout Purity for CLI Composability | Accepted | 2025-01-25 |
| [ADR-0006](ADR-0006-typescript-npm-publishing.md) | TypeScript npm Publishing Standard | Accepted | 2026-01-31 |
| [ADR-0007](ADR-0007-profile-b-go-twin-and-wasm-pyo3-bindings.md) | Profile B Go Twin, Single Rust Crate, and WASM/PyO3 Bindings | Accepted | 2026-11-09 |

### Design Decision Records (DDR)

| ID       | Title                | Status   | Date       |
| -------- | -------------------- | -------- | ---------- |
| [DDR-0001](DDR-0001-extract-v1-receipts-refusals-and-limits.md) | Extract v1, Receipts, Refusals, and Limits Profiles | Proposed | 2026-11-09 |

### Security Decision Records (SDR)

| ID       | Title                | Status   | Date       |
| -------- | -------------------- | -------- | ---------- |
| [SDR-0002](SDR-0002-ooxml-v0-block-granularity.md) | OOXML v0 Block Granularity | Accepted | 2026-01-27 |
| [SDR-0003](SDR-0003-pdf-scope-limits-and-refusals.md) | PDF Scope, Resource Limits, and Refusals | Accepted | 2026-11-09 |

## Creating a New Record

1. Copy the appropriate template:
   - `adr-template.md` for architecture decisions
   - `ddr-template.md` for design decisions
   - `sdr-template.md` for security decisions

2. Name the file: `<TYPE>-<NNNN>-<slug-form-title>.md`
   - Example: `ADR-0003-ffi-design.md`

3. Fill in the template sections

4. Update this README index

5. Submit for review

## Decision Lifecycle

```
Proposed -> Accepted -> (Deprecated | Superseded)
```

- **Proposed**: Under discussion, not yet approved
- **Accepted**: Approved and in effect
- **Deprecated**: No longer recommended but not replaced
- **Superseded**: Replaced by another decision (link to successor)
