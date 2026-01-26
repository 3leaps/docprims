# ADR-0003: Input Validation Policy

> **Status**: Accepted
> **Date**: 2025-01-25
> **Authors**: 3leaps Architecture Council

## Context

docprims parses untrusted document formats (OOXML, HTML, XML, Markdown). These documents may come from:

- User uploads
- Network sources
- Adversarial actors

Malformed input can cause:

- Crashes (panics, segfaults)
- Resource exhaustion (memory, CPU, file handles)
- Security vulnerabilities (buffer overflows, XXE, path traversal)

Additionally, docprims uses structured data formats (JSON schemas for FFI contracts, configuration). Schema violations can cause:

- Silent data corruption
- Undefined behavior in downstream consumers
- Difficult-to-diagnose bugs

## Decision

### 1. All Untrusted Input Must Be Validated

**No exceptions without explicit maintainer approval and ADR documentation.**

Untrusted input includes:

| Source | Examples |
|--------|----------|
| Document content | OOXML files, HTML, XML, Markdown |
| File paths | User-provided paths for extraction |
| CLI arguments | All arguments from users |
| FFI inputs | All data crossing language boundaries |
| Network data | URLs, remote content (if supported) |

### 2. Validation Requirements

Every parser and input handler must:

1. **Validate structure before processing**
   - Check magic bytes/signatures where applicable
   - Verify structural integrity (well-formed XML, valid ZIP)
   - Reject malformed input with clear error messages

2. **Enforce resource limits**
   - Maximum file size
   - Maximum decompression ratio (zip bombs)
   - Maximum element count/depth
   - Timeout for long operations

3. **Sanitize paths and identifiers**
   - No path traversal (`../`)
   - No null bytes in strings
   - Validate character encoding

4. **Handle errors gracefully**
   - Return `Result<T, Error>`, never panic on bad input
   - Provide actionable error messages
   - Log validation failures for debugging

### 3. Schema-Backed Data Must Be Validated Against Schema

All structured data with a defined schema must be validated:

| Data Type | Validation Method |
|-----------|-------------------|
| JSON configuration | JSON Schema validation |
| FFI contracts | Schema-defined type checking |
| API responses | Contract validation |

Schema validation must occur:

- At deserialization time (not lazily)
- Before any business logic operates on the data
- With schema version compatibility checks

### 4. Exception Process

Exceptions to validation requirements require:

1. Written request to maintainer with justification
2. Risk assessment documenting potential impacts
3. Compensating controls if validation is reduced
4. ADR amendment documenting the exception
5. Explicit sign-off from maintainer

No exception may bypass validation for security-critical paths (file paths, resource limits).

## Consequences

### Positive

- Defense in depth against malformed/malicious input
- Consistent error handling across codebase
- Auditable validation coverage
- Schema drift detected early

### Negative

- Performance overhead for validation
- More verbose code with explicit checks
- Exception process adds friction (intentionally)

### Neutral

- Validation code is testable and documentable
- Clear contract between components

## Implementation Guidelines

### Parser Safety Pattern

```rust
pub fn extract(input: &[u8], options: &ExtractOptions) -> Result<ExtractedText> {
    // 1. Validate size limits
    if input.len() > options.max_size {
        return Err(DocprimsError::ResourceLimit("input exceeds max size"));
    }

    // 2. Validate structure
    let archive = validate_zip_structure(input)?;

    // 3. Validate content with limits
    let content = extract_with_limits(&archive, &options.limits)?;

    // 4. Validate output encoding
    let text = validate_utf8(content)?;

    Ok(ExtractedText::complete(text))
}
```

### Schema Validation Pattern

```rust
pub fn load_config(data: &[u8]) -> Result<Config> {
    // 1. Parse JSON
    let value: serde_json::Value = serde_json::from_slice(data)?;

    // 2. Validate against schema
    validate_schema(&value, CONFIG_SCHEMA)?;

    // 3. Deserialize to typed struct
    let config: Config = serde_json::from_value(value)?;

    Ok(config)
}
```

## References

- [OWASP Input Validation Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Input_Validation_Cheat_Sheet.html)
- [CWE-20: Improper Input Validation](https://cwe.mitre.org/data/definitions/20.html)
- [sysprims ADR-0011: PID Validation Safety](https://github.com/3leaps/sysprims/blob/main/docs/decisions/ADR-0011-pid-validation-safety.md)
- ADR-0001: License Policy (defense in depth principle)
