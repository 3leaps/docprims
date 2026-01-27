//! docprims CLI - GPL-free document text extraction.
//!
//! Extracts text from documents (DOCX, XLSX, PPTX, Markdown, HTML, XML) into
//! structured output suitable for downstream processing.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use docprims_core::{ExtractLimits, ExtractedText};
use rsfulmen::foundry::exit_codes::{EXIT_DATA_INVALID, EXIT_FAILURE, EXIT_SUCCESS, EXIT_USAGE};
use tracing::{debug, error, info};
use tracing_subscriber::{filter::EnvFilter, fmt, prelude::*};

/// GPL-free document text extraction tool.
#[derive(Parser, Debug)]
#[command(name = "docprims")]
#[command(about = "GPL-free document text extraction", long_about = None)]
#[command(version)]
struct Cli {
    /// The format for log output.
    #[arg(long, value_name = "FORMAT", default_value = "text")]
    log_format: LogFormat,

    /// The minimum log level to display.
    #[arg(long, value_name = "LEVEL", default_value = "info")]
    log_level: tracing::Level,

    #[command(subcommand)]
    command: Commands,
}

/// Log output format.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum LogFormat {
    /// Human-readable text format.
    Text,
    /// Machine-readable JSON format.
    Json,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Extract text from documents.
    ///
    /// Extracts text content from one or more documents and outputs in the
    /// requested format. For multiple files with JSON output, uses NDJSON
    /// by default (one JSON object per line).
    Extract(ExtractArgs),

    /// List supported document formats.
    Formats,

    /// Print version information.
    Version,
}

#[derive(Parser, Debug)]
struct ExtractArgs {
    /// Input file(s) to extract.
    #[arg(required = true)]
    files: Vec<PathBuf>,

    /// Output format.
    #[arg(short, long, value_enum, default_value = "plain")]
    format: OutputFormat,

    /// Include document metadata in output.
    #[arg(long)]
    include_metadata: bool,

    /// Include structured blocks in JSON output.
    #[arg(long)]
    include_blocks: bool,

    /// Output as JSON array instead of NDJSON for multiple files.
    #[arg(long)]
    json_array: bool,

    /// Maximum input file size in bytes.
    #[arg(long, value_name = "BYTES", default_value = "104857600")]
    max_input_bytes: usize,

    /// Extraction timeout in milliseconds.
    #[arg(long, value_name = "MS", default_value = "30000")]
    timeout_ms: u64,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum OutputFormat {
    /// Plain text output.
    Plain,
    /// JSON structured output (DocprimsExtract schema).
    Json,
}

fn main() {
    let cli = Cli::parse();

    // Initialize tracing subscriber with format and level
    let filter = EnvFilter::from_default_env().add_directive(cli.log_level.into());

    match cli.log_format {
        LogFormat::Text => {
            tracing_subscriber::registry()
                .with(fmt::layer().with_writer(std::io::stderr))
                .with(filter)
                .init();
        }
        LogFormat::Json => {
            tracing_subscriber::registry()
                .with(fmt::layer().json().with_writer(std::io::stderr))
                .with(filter)
                .init();
        }
    }

    debug!("docprims CLI initialized");

    let exit_code = match run_command(cli.command) {
        Ok(code) => code,
        Err(e) => {
            error!(error = %e, "Command failed");
            eprintln!("Error: {e}");
            e.exit_code()
        }
    };

    info!(exit_code = exit_code, "Exiting");
    std::process::exit(exit_code);
}

/// Run the selected command.
fn run_command(command: Commands) -> Result<i32, CliError> {
    match command {
        Commands::Extract(args) => run_extract(args),
        Commands::Formats => {
            run_formats();
            Ok(EXIT_SUCCESS)
        }
        Commands::Version => {
            run_version();
            Ok(EXIT_SUCCESS)
        }
    }
}

/// Extract text from documents.
fn run_extract(args: ExtractArgs) -> Result<i32, CliError> {
    let file_count = args.files.len();
    let is_multi = file_count > 1;
    let use_array = args.json_array && is_multi && args.format == OutputFormat::Json;

    info!(
        files = file_count,
        format = ?args.format,
        "Starting extraction"
    );

    if use_array {
        print!("[");
    }

    let mut first = true;
    let mut had_errors = false;

    for file in &args.files {
        debug!(file = %file.display(), "Extracting");

        match extract_file(file, &args) {
            Ok(text) => {
                if use_array {
                    if !first {
                        print!(",");
                    }
                    first = false;
                }
                output_result(file, &text, &args);
            }
            Err(e) => {
                error!(file = %file.display(), error = %e, "Extraction failed");
                eprintln!("Error extracting {}: {}", file.display(), e);
                had_errors = true;
            }
        }
    }

    if use_array {
        println!("]");
    }

    if had_errors {
        Ok(EXIT_DATA_INVALID)
    } else {
        Ok(EXIT_SUCCESS)
    }
}

/// Extract text from a single file.
fn extract_file(path: &PathBuf, args: &ExtractArgs) -> Result<ExtractedText, CliError> {
    // Check file size first
    let metadata = std::fs::metadata(path).map_err(|e| CliError::Io {
        path: path.clone(),
        source: e,
    })?;

    if metadata.len() as usize > args.max_input_bytes {
        return Err(CliError::ResourceLimit {
            path: path.clone(),
            reason: format!(
                "File size {} exceeds limit {}",
                metadata.len(),
                args.max_input_bytes
            ),
        });
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "docx" => docprims_ooxml::extract_docx(path).map_err(|e| CliError::Extraction {
            path: path.clone(),
            source: e.to_string(),
        }),
        "xlsx" => docprims_ooxml::extract_xlsx(path).map_err(|e| CliError::Extraction {
            path: path.clone(),
            source: e.to_string(),
        }),
        "pptx" => docprims_ooxml::extract_pptx(path).map_err(|e| CliError::Extraction {
            path: path.clone(),
            source: e.to_string(),
        }),
        "md" | "markdown" => {
            docprims_text::extract_markdown(path).map_err(|e| CliError::Extraction {
                path: path.clone(),
                source: e.to_string(),
            })
        }
        "html" | "htm" => docprims_text::extract_html(path).map_err(|e| CliError::Extraction {
            path: path.clone(),
            source: e.to_string(),
        }),
        "xml" => docprims_text::extract_xml(path).map_err(|e| CliError::Extraction {
            path: path.clone(),
            source: e.to_string(),
        }),
        _ => Err(CliError::UnsupportedFormat {
            path: path.clone(),
            extension: ext,
        }),
    }
}

/// Output extraction result in the requested format.
fn output_result(path: &std::path::Path, text: &ExtractedText, args: &ExtractArgs) {
    match args.format {
        OutputFormat::Plain => {
            println!("{}", text.content);
        }
        OutputFormat::Json => {
            // If requested, emit schema-conformant v0 output for formats that support blocks.
            if args.include_blocks {
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("unknown")
                    .to_lowercase();

                if ext == "md" || ext == "markdown" {
                    let limits = ExtractLimits {
                        max_input_bytes: args.max_input_bytes,
                        ..ExtractLimits::default()
                    };

                    match docprims_text::extract_markdown_v0(path, limits) {
                        Ok(extract) => {
                            let json = if args.json_array {
                                serde_json::to_string_pretty(&extract).unwrap_or_else(|e| {
                                    eprintln!("Error serializing JSON: {e}");
                                    "{}".to_string()
                                })
                            } else {
                                serde_json::to_string(&extract).unwrap_or_else(|e| {
                                    eprintln!("Error serializing JSON: {e}");
                                    "{}".to_string()
                                })
                            };
                            println!("{json}");
                            return;
                        }
                        Err(e) => {
                            eprintln!(
                                "Error extracting structured blocks for {}: {}",
                                path.display(),
                                e
                            );
                        }
                    }
                } else if ext == "docx" {
                    let limits = ExtractLimits {
                        max_input_bytes: args.max_input_bytes,
                        ..ExtractLimits::default()
                    };

                    match docprims_ooxml::extract_docx_v0(path, limits) {
                        Ok(extract) => {
                            let json = if args.json_array {
                                serde_json::to_string_pretty(&extract).unwrap_or_else(|e| {
                                    eprintln!("Error serializing JSON: {e}");
                                    "{}".to_string()
                                })
                            } else {
                                serde_json::to_string(&extract).unwrap_or_else(|e| {
                                    eprintln!("Error serializing JSON: {e}");
                                    "{}".to_string()
                                })
                            };
                            println!("{json}");
                            return;
                        }
                        Err(e) => {
                            eprintln!(
                                "Error extracting structured blocks for {}: {}",
                                path.display(),
                                e
                            );
                        }
                    }
                } else if ext == "xlsx" {
                    let limits = ExtractLimits {
                        max_input_bytes: args.max_input_bytes,
                        ..ExtractLimits::default()
                    };

                    match docprims_ooxml::extract_xlsx_v0(path, limits) {
                        Ok(extract) => {
                            let json = if args.json_array {
                                serde_json::to_string_pretty(&extract).unwrap_or_else(|e| {
                                    eprintln!("Error serializing JSON: {e}");
                                    "{}".to_string()
                                })
                            } else {
                                serde_json::to_string(&extract).unwrap_or_else(|e| {
                                    eprintln!("Error serializing JSON: {e}");
                                    "{}".to_string()
                                })
                            };
                            println!("{json}");
                            return;
                        }
                        Err(e) => {
                            eprintln!(
                                "Error extracting structured blocks for {}: {}",
                                path.display(),
                                e
                            );
                        }
                    }
                } else if ext == "pptx" {
                    let limits = ExtractLimits {
                        max_input_bytes: args.max_input_bytes,
                        ..ExtractLimits::default()
                    };

                    match docprims_ooxml::extract_pptx_v0(path, limits) {
                        Ok(extract) => {
                            let json = if args.json_array {
                                serde_json::to_string_pretty(&extract).unwrap_or_else(|e| {
                                    eprintln!("Error serializing JSON: {e}");
                                    "{}".to_string()
                                })
                            } else {
                                serde_json::to_string(&extract).unwrap_or_else(|e| {
                                    eprintln!("Error serializing JSON: {e}");
                                    "{}".to_string()
                                })
                            };
                            println!("{json}");
                            return;
                        }
                        Err(e) => {
                            eprintln!(
                                "Error extracting structured blocks for {}: {}",
                                path.display(),
                                e
                            );
                        }
                    }
                } else if ext == "html" || ext == "htm" {
                    let limits = ExtractLimits {
                        max_input_bytes: args.max_input_bytes,
                        ..ExtractLimits::default()
                    };

                    match docprims_text::extract_html_v0(path, limits) {
                        Ok(extract) => {
                            let json = if args.json_array {
                                serde_json::to_string_pretty(&extract).unwrap_or_else(|e| {
                                    eprintln!("Error serializing JSON: {e}");
                                    "{}".to_string()
                                })
                            } else {
                                serde_json::to_string(&extract).unwrap_or_else(|e| {
                                    eprintln!("Error serializing JSON: {e}");
                                    "{}".to_string()
                                })
                            };
                            println!("{json}");
                            return;
                        }
                        Err(e) => {
                            eprintln!(
                                "Error extracting structured blocks for {}: {}",
                                path.display(),
                                e
                            );
                        }
                    }
                } else if ext == "xml" {
                    let limits = ExtractLimits {
                        max_input_bytes: args.max_input_bytes,
                        ..ExtractLimits::default()
                    };

                    match docprims_text::extract_xml_v0(path, limits) {
                        Ok(extract) => {
                            let json = if args.json_array {
                                serde_json::to_string_pretty(&extract).unwrap_or_else(|e| {
                                    eprintln!("Error serializing JSON: {e}");
                                    "{}".to_string()
                                })
                            } else {
                                serde_json::to_string(&extract).unwrap_or_else(|e| {
                                    eprintln!("Error serializing JSON: {e}");
                                    "{}".to_string()
                                })
                            };
                            println!("{json}");
                            return;
                        }
                        Err(e) => {
                            eprintln!(
                                "Error extracting structured blocks for {}: {}",
                                path.display(),
                                e
                            );
                        }
                    }
                }
            }

            // Legacy JSON output (pre-contract); retained while other formats are implemented.
            #[derive(serde::Serialize)]
            struct JsonOutput<'a> {
                schema_version: &'static str,
                generator: Generator,
                source: Source<'a>,
                document: Document<'a>,
            }

            #[derive(serde::Serialize)]
            struct Generator {
                name: &'static str,
                version: &'static str,
            }

            #[derive(serde::Serialize)]
            struct Source<'a> {
                uri: &'a str,
                format: FormatInfo,
            }

            #[derive(serde::Serialize)]
            struct FormatInfo {
                family: &'static str,
                kind: String,
            }

            #[derive(serde::Serialize)]
            struct Document<'a> {
                quality: Quality,
                text: &'a str,
                #[serde(skip_serializing_if = "Option::is_none")]
                metadata: Option<&'a docprims_core::DocumentMetadata>,
            }

            #[derive(serde::Serialize)]
            struct Quality {
                status: &'static str,
            }

            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("unknown");

            let family = match ext {
                "docx" | "xlsx" | "pptx" => "ooxml",
                "md" | "markdown" | "html" | "htm" | "xml" => "text",
                _ => "unknown",
            };

            let output = JsonOutput {
                schema_version: "1.0.0",
                generator: Generator {
                    name: "docprims",
                    version: env!("CARGO_PKG_VERSION"),
                },
                source: Source {
                    uri: &path.display().to_string(),
                    format: FormatInfo {
                        family,
                        kind: ext.to_string(),
                    },
                },
                document: Document {
                    quality: Quality { status: "complete" },
                    text: &text.content,
                    metadata: if args.include_metadata {
                        text.metadata.as_ref()
                    } else {
                        None
                    },
                },
            };

            let json = if args.json_array {
                // Pretty print for array mode
                serde_json::to_string_pretty(&output).unwrap_or_else(|e| {
                    eprintln!("Error serializing JSON: {e}");
                    "{}".to_string()
                })
            } else {
                // Compact for NDJSON
                serde_json::to_string(&output).unwrap_or_else(|e| {
                    eprintln!("Error serializing JSON: {e}");
                    "{}".to_string()
                })
            };

            println!("{json}");
        }
    }
}

/// List supported formats.
fn run_formats() {
    println!("Supported formats:");
    println!();
    println!("  OOXML (Office Open XML):");
    println!("    docx  - Word documents");
    println!("    xlsx  - Excel spreadsheets");
    println!("    pptx  - PowerPoint presentations");
    println!();
    println!("  Text formats:");
    println!("    md    - Markdown");
    println!("    html  - HTML documents");
    println!("    xml   - XML documents");
}

/// Print version information.
fn run_version() {
    println!("docprims {}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("Components:");
    println!("  docprims-core:  {}", env!("CARGO_PKG_VERSION"));
    println!("  docprims-text:  {}", env!("CARGO_PKG_VERSION"));
    println!("  docprims-ooxml: {}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("License: MIT OR Apache-2.0");
    println!("Repository: https://github.com/3leaps/docprims");
}

/// CLI error types.
#[derive(Debug)]
enum CliError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Extraction {
        path: PathBuf,
        source: String,
    },
    UnsupportedFormat {
        path: PathBuf,
        extension: String,
    },
    ResourceLimit {
        path: PathBuf,
        reason: String,
    },
}

impl CliError {
    /// Map error to exit code.
    fn exit_code(&self) -> i32 {
        match self {
            CliError::Io { .. } => EXIT_FAILURE,
            CliError::Extraction { .. } => EXIT_DATA_INVALID,
            CliError::UnsupportedFormat { .. } => EXIT_USAGE,
            CliError::ResourceLimit { .. } => EXIT_DATA_INVALID,
        }
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::Io { path, source } => {
                write!(f, "I/O error for {}: {}", path.display(), source)
            }
            CliError::Extraction { path, source } => {
                write!(f, "Extraction failed for {}: {}", path.display(), source)
            }
            CliError::UnsupportedFormat { path, extension } => {
                write!(
                    f,
                    "Unsupported format '{}' for {}",
                    extension,
                    path.display()
                )
            }
            CliError::ResourceLimit { path, reason } => {
                write!(
                    f,
                    "Resource limit exceeded for {}: {}",
                    path.display(),
                    reason
                )
            }
        }
    }
}

impl std::error::Error for CliError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parses_extract_command() {
        let cli = Cli::try_parse_from(["docprims", "extract", "test.docx"]).unwrap();
        assert!(matches!(cli.command, Commands::Extract(_)));
    }

    #[test]
    fn cli_parses_formats_command() {
        let cli = Cli::try_parse_from(["docprims", "formats"]).unwrap();
        assert!(matches!(cli.command, Commands::Formats));
    }

    #[test]
    fn cli_parses_version_command() {
        let cli = Cli::try_parse_from(["docprims", "version"]).unwrap();
        assert!(matches!(cli.command, Commands::Version));
    }

    #[test]
    fn cli_parses_log_format() {
        let cli = Cli::try_parse_from(["docprims", "--log-format", "json", "formats"]).unwrap();
        assert_eq!(cli.log_format, LogFormat::Json);
    }

    #[test]
    fn cli_parses_json_output_format() {
        let cli =
            Cli::try_parse_from(["docprims", "extract", "--format", "json", "test.docx"]).unwrap();
        if let Commands::Extract(args) = cli.command {
            assert_eq!(args.format, OutputFormat::Json);
        } else {
            panic!("Expected Extract command");
        }
    }

    #[test]
    fn cli_error_exit_codes() {
        let io_err = CliError::Io {
            path: PathBuf::from("test"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
        };
        assert_eq!(io_err.exit_code(), EXIT_FAILURE);

        let extract_err = CliError::Extraction {
            path: PathBuf::from("test"),
            source: "parse error".to_string(),
        };
        assert_eq!(extract_err.exit_code(), EXIT_DATA_INVALID);

        let format_err = CliError::UnsupportedFormat {
            path: PathBuf::from("test"),
            extension: "xyz".to_string(),
        };
        assert_eq!(format_err.exit_code(), EXIT_USAGE);
    }
}
