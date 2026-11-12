mod audio;
mod bridge;
mod config;
mod hardware;
mod install;
mod midi;
mod model;
mod native_install;
mod operation;
mod plugin_catalog;
mod runtime;
mod server;
mod song;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use rmcp::ServiceExt;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::{
    config::Config,
    operation::{Operation, Request},
    runtime::RuntimeHandle,
};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(flatten)]
    config: Config,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Serve MCP JSON-RPC over stdin and stdout.
    Serve,
    /// Install the DAW bridge and EditTasks; restart Studio Pro afterward.
    Install {
        #[arg(long)]
        force: bool,
    },
    /// Remove unchanged managed assets while preserving user modifications.
    Uninstall,
    /// Install a locally built native extension for the supported macOS host.
    NativeInstall {
        extension: std::path::PathBuf,
        #[arg(long)]
        force: bool,
    },
    /// Remove unchanged native extension files; restart Studio Pro afterward.
    NativeUninstall,
    /// Inspect installation and optionally check the live DAW connection.
    Doctor {
        #[arg(long)]
        live: bool,
    },
    /// Execute one validated operation and print its outcome receipt.
    Call {
        operation: String,
        #[arg(default_value = "{}")]
        arguments: String,
        #[arg(long)]
        expected_document: Option<String>,
    },
    /// Keep the native MIDI source available until Ctrl-C.
    Hold,
    /// Keep one owner alive and accept typed JSON-line probe requests.
    ProbeShell,
    /// Print the finite operation JSON Schema.
    Schema,
    /// Inspect a saved song without changing it or connecting to the DAW.
    SongRead { path: std::path::PathBuf },
    /// Analyze a WAV file's loudness, peaks and stereo measurements in Rust.
    AudioAnalyze { path: std::path::PathBuf },
    /// Read operating-system audio and MIDI metadata without opening a stream.
    HardwareDevices,
    /// Observe one exact MIDI source briefly without forwarding messages.
    MidiInputObserve {
        #[arg(long, allow_negative_numbers = true)]
        source_uid: i32,
        #[arg(long)]
        duration_ms: u32,
        #[arg(long)]
        max_events: u32,
    },
    /// Discover partial parameter candidates from installed plugin metadata.
    LocalCatalog {
        #[arg(long, default_value = "/Applications/Studio Pro 8.app/Contents")]
        app: std::path::PathBuf,
        #[arg(long)]
        plugin: Option<String>,
        #[arg(long)]
        class_id: Option<String>,
    },
}

#[derive(Deserialize)]
struct ProbeCall {
    #[serde(alias = "operation")]
    op: String,
    #[serde(default = "empty_json")]
    args: Value,
    expected_document: Option<String>,
}

fn empty_json() -> Value {
    json!({})
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let cli = Cli::parse();
    match cli.command {
        Command::Install { force } => print_json(&install::install(&cli.config, force)?)?,
        Command::Uninstall => print_json(&install::uninstall(&cli.config)?)?,
        Command::NativeInstall { extension, force } => {
            print_json(&native_install::install(&cli.config, &extension, force)?)?;
        }
        Command::NativeUninstall => print_json(&native_install::uninstall(&cli.config)?)?,
        Command::Schema => print_json(&operation::schema())?,
        Command::SongRead { path } => {
            let saved = song::read_song(&path).map_err(anyhow::Error::msg)?;
            print_json(&serde_json::to_value(saved)?)?;
        }
        Command::AudioAnalyze { path } => {
            let analysis = audio::analyze_file(&path).map_err(anyhow::Error::msg)?;
            print_json(&serde_json::to_value(analysis)?)?;
        }
        Command::HardwareDevices => {
            print_json(&hardware::inspect().map_err(anyhow::Error::msg)?)?;
        }
        Command::MidiInputObserve {
            source_uid,
            duration_ms,
            max_events,
        } => {
            let query = hardware::midi_input::ObserveMidiInput {
                source_uid,
                duration_ms,
                max_events,
            };
            hardware::midi_input::validate(&query)?;
            print_json(&hardware::midi_input::observe(&query)?)?;
        }
        Command::LocalCatalog {
            app,
            plugin,
            class_id,
        } => {
            let catalog = plugin_catalog::discover(&app).map_err(anyhow::Error::msg)?;
            let result = if let Some(plugin) = plugin {
                json!({"plugin":plugin,"completeness":catalog.completeness,
                    "candidate_names":catalog.candidate_names(&plugin,class_id.as_deref()),
                    "matches":catalog.matching(&plugin,class_id.as_deref()).collect::<Vec<_>>(),
                    "limitations":catalog.limitations})
            } else {
                serde_json::to_value(catalog)?
            };
            print_json(&result)?;
        }
        Command::Doctor { live } => {
            let mut report = install::doctor(&cli.config);
            if live {
                let runtime = RuntimeHandle::start(cli.config)?;
                let receipt = runtime
                    .execute(Request {
                        operation: Operation::from_parts("context", json!({}))?,
                        expected_document: None,
                    })
                    .await?;
                report["live_verified"] =
                    json!(receipt.outcome == model::Outcome::Observed && receipt.error.is_none());
                report["live_check"] = serde_json::to_value(receipt)?;
            }
            print_json(&report)?;
        }
        Command::Call {
            operation,
            arguments,
            expected_document,
        } => {
            let runtime = RuntimeHandle::start(cli.config)?;
            let receipt = runtime
                .execute(Request {
                    operation: Operation::from_parts(
                        &operation,
                        serde_json::from_str(&arguments)
                            .context("arguments must be a JSON object")?,
                    )?,
                    expected_document,
                })
                .await?;
            print_json(&serde_json::to_value(receipt)?)?;
        }
        Command::Hold => {
            let _runtime = RuntimeHandle::start(cli.config)?;
            print_json(&json!({"ready":true,"midi_port":midi::PORT_NAME,"mode":"hold"}))?;
            tokio::signal::ctrl_c().await?;
        }
        Command::ProbeShell => {
            let runtime = RuntimeHandle::start(cli.config)?;
            println!(
                "{}",
                json!({"ready":true,"midi_port":midi::PORT_NAME,"mode":"probe-shell"})
            );
            let mut lines = BufReader::new(tokio::io::stdin()).lines();
            let mut stdout = tokio::io::stdout();
            while let Some(line) = lines.next_line().await? {
                if line.trim().is_empty() {
                    continue;
                }
                let result = async {
                    let call: ProbeCall = serde_json::from_str(&line)
                        .map_err(|e| model::OperationError::new("invalid_json", e.to_string()))?;
                    runtime
                        .execute(Request {
                            operation: Operation::from_parts(&call.op, call.args)?,
                            expected_document: call.expected_document,
                        })
                        .await
                }
                .await;
                let value = match result {
                    Ok(receipt) => serde_json::to_value(receipt)?,
                    Err(error) => json!({"outcome":"not_applied","error":error}),
                };
                stdout
                    .write_all(format!("{}\n", serde_json::to_string(&value)?).as_bytes())
                    .await?;
                stdout.flush().await?;
            }
        }
        Command::Serve => {
            let runtime = RuntimeHandle::start(cli.config)?;
            let service = server::McpServer::new(runtime)
                .serve(rmcp::transport::stdio())
                .await?;
            service.waiting().await?;
        }
    }
    Ok(())
}

fn print_json(value: &Value) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

#[cfg(test)]
mod cli_tests {
    use super::*;

    #[test]
    fn hardware_cli_parses_without_a_daw_operation() {
        let cli = Cli::try_parse_from(["fsp", "hardware-devices"]).unwrap();
        assert!(matches!(cli.command, Command::HardwareDevices));
        assert!(Cli::try_parse_from(["fsp", "hardware-devices", "--path", "/unchecked"]).is_err());
    }

    #[test]
    fn passive_midi_cli_preserves_signed_source_uid() {
        let cli = Cli::try_parse_from([
            "fsp",
            "midi-input-observe",
            "--source-uid",
            "-123456789",
            "--duration-ms",
            "100",
            "--max-events",
            "16",
        ])
        .unwrap();
        match cli.command {
            Command::MidiInputObserve {
                source_uid,
                duration_ms,
                max_events,
            } => {
                assert_eq!((source_uid, duration_ms, max_events), (-123456789, 100, 16));
            }
            _ => panic!("Expected the finite observer command"),
        }
        assert!(Cli::try_parse_from(["fsp", "midi-input-observe", "--source-uid", "1"]).is_err());
    }
}
