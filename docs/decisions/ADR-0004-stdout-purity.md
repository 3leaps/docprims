# ADR-0004: Stdout Purity for CLI Composability

> **Status**: Accepted
> **Date**: 2025-01-25
> **Authors**: 3leaps Architecture Council

## Context

docprims includes a CLI tool that extracts text from documents. Users will compose it with other Unix tools:

```bash
docprims extract document.docx --format json | jq '.content'
docprims extract *.xlsx --format json | process-documents
cat manifest.txt | xargs -I{} docprims extract {} >> output.txt
```

For pipeline composition to work correctly, the CLI must maintain **stdout purity**: structured output goes to stdout, everything else goes to stderr.

## Decision

### Stdout: Structured Output Only

Stdout is reserved exclusively for **parseable program output**:

| Output Type | Format | Stdout |
|-------------|--------|--------|
| Extracted text | Plain text (default) | Yes |
| Structured results | JSON (`--format json`) | Yes |
| Machine-readable reports | JSON/CSV | Yes |

Stdout must:

- Contain only the requested output format
- Be empty when using `--quiet` mode with no results
- Never include logging, diagnostics, or metadata

### Stderr: Everything Else

Stderr is used for all non-output information:

| Content | Stream |
|---------|--------|
| Version information (`--version`) | stderr |
| Help text (`--help`) | stderr |
| Error messages | stderr |
| Warnings | stderr |
| Progress indicators | stderr |
| Debug/trace logging | stderr |
| Diagnostic information | stderr |

### Implementation Requirements

1. **Logging framework must default to stderr**
   ```rust
   // Configure tracing to write to stderr
   tracing_subscriber::fmt()
       .with_writer(std::io::stderr)
       .init();
   ```

2. **Explicit output functions for stdout**
   ```rust
   // Output goes to stdout
   fn output_result(text: &ExtractedText, format: OutputFormat) {
       match format {
           OutputFormat::Plain => println!("{}", text.content),
           OutputFormat::Json => {
               println!("{}", serde_json::to_string(text).unwrap());
           }
       }
   }
   ```

3. **Errors and diagnostics to stderr**
   ```rust
   // Errors go to stderr
   fn report_error(err: &DocprimsError) {
       eprintln!("error: {}", err);
   }
   ```

### Exit Codes

Exit codes are part of the CLI contract:

| Code | Meaning |
|------|---------|
| 0 | Success - text extracted |
| 1 | Partial success - some files failed |
| 2 | Error - extraction failed |
| 3 | Usage error - invalid arguments |

### Design Override Process

If a specific feature requires different stream behavior (e.g., interactive mode, TUI), it must be:

1. Documented in the feature specification
2. Opt-in via explicit flag
3. Clearly marked as non-composable

## Consequences

### Positive

- Clean pipeline composition: `docprims extract doc.docx | jq '.content'`
- Errors visible even when stdout is redirected to a file
- Version/help text doesn't pollute captured output
- Consistent with Unix conventions

### Negative

- Users must use `2>&1` to capture version output: `docprims --version 2>&1`
- Cannot intermix progress with output (by design)

### Neutral

- Requires discipline in code review to maintain
- Standard practice in well-designed CLI tools

## Examples

### Correct Usage

```bash
# Extract to file, see errors on terminal
docprims extract large.docx > output.txt

# Chain with jq, logging stays separate
docprims extract doc.docx --format json 2>/dev/null | jq '.content'

# Batch process with xargs
find . -name "*.docx" | xargs -I{} docprims extract {} --format json >> results.jsonl

# Capture version for scripts
VERSION=$(docprims --version 2>&1)
```

### Verification

```bash
# Verify no logging leaks to stdout
docprims extract test.docx --format json > /tmp/out.json 2>/dev/null
jq . /tmp/out.json  # Should parse cleanly
```

## References

- [Unix Philosophy](https://en.wikipedia.org/wiki/Unix_philosophy) - "Write programs that do one thing and do it well"
- [shellsentry ADR-0002](https://github.com/3leaps/shellsentry/blob/main/docs/architecture/adr/ADR-0002-stdout-stderr-conventions-for-cli-composability.md) - Stdout/Stderr Conventions
- [12 Factor CLI Apps](https://medium.com/@jdxcode/12-factor-cli-apps-dd3c227a0e46) - Log to stderr
