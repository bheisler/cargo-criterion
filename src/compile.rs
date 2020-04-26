//! Module that handles calling out to `cargo bench` and parsing the machine-readable messages
//! to compile the benchmarks and collect the information on the benchmark executables that it
//! emits.

use crate::args::CargoArguments;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[derive(Debug)]
/// Enum representing the different ways calling Cargo might fail
pub enum CompileError {
    CompileFailed,
    JsonError(serde_json::error::Error),
    IoError(std::io::Error),
}
impl From<std::io::Error> for CompileError {
    fn from(other: std::io::Error) -> CompileError {
        CompileError::IoError(other)
    }
}
impl From<serde_json::error::Error> for CompileError {
    fn from(other: serde_json::error::Error) -> CompileError {
        CompileError::JsonError(other)
    }
}
impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::CompileFailed => {
                write!(f, "'cargo bench' returned an error; unable to continue.")
            }
            CompileError::IoError(io_error) => write!(
                f,
                "Unexpected IO Error while running 'cargo bench':\n{}",
                io_error
            ),
            CompileError::JsonError(json_error) => write!(
                f,
                "Unable to parse messages from 'cargo bench':\n{}",
                json_error
            ),
        }
    }
}
impl std::error::Error for CompileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CompileError::CompileFailed => None,
            CompileError::IoError(io_error) => Some(io_error),
            CompileError::JsonError(json_error) => Some(json_error),
        }
    }
}

/// Structure representing a compiled benchmark executable.
#[derive(Debug)]
pub struct Benchmark {
    name: String,
    executable: PathBuf,
}

// These structs match the parts of Cargo's message format that we care about.
#[derive(Serialize, Deserialize, Debug)]
struct Target {
    name: String,
    kind: Vec<String>,
}

/// Enum listing out the different types of messages that Cargo can send. We only care about the
/// compiler-artifact message.
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "reason")]
enum Message {
    #[serde(rename = "compiler-artifact")]
    CompilerArtifact {
        target: Target,
        executable: Option<PathBuf>,
    },

    #[serde(rename = "compiler-message")]
    CompilerMessage {},

    #[serde(rename = "build-script-executed")]
    BuildScriptExecuted {},
}

/// Launches `cargo bench` with the given additional arguments, with some additional arguments to
/// list out the benchmarks and their executables and parses that information. This compiles the
/// benchmarks but doesn't run them. Returns information on the compiled benchmarks that we can use
/// to run them directly.
pub fn compile(cargo_args: &CargoArguments) -> Result<Vec<Benchmark>, CompileError> {
    let mut cargo = Command::new("cargo")
        .arg("bench")
        .args(&cargo_args.to_arguments())
        .args(&["--no-run", "--message-format", "json"])
        .stdin(Stdio::null())
        .stderr(Stdio::inherit()) // Cargo writes its normal compile output to stderr
        .stdout(Stdio::piped()) // Capture the JSON messages on stdout
        .spawn()?;

    // Build a message stream reading from the child process
    let cargo_stdout = cargo
        .stdout
        .take()
        .expect("Child process doesn't have a stdout handle");
    let stream = serde_json::Deserializer::from_reader(cargo_stdout).into_iter::<Message>();

    // Collect the benchmark artifacts from the message stream
    let mut benchmarks = vec![];
    for message in stream {
        let message = message?;

        if let Message::CompilerArtifact { target, executable } = message {
            // We only care about benchmark artifacts
            if target.kind.iter().any(|kind| kind == "bench") {
                let bench = Benchmark {
                    name: target.name,
                    executable: executable.expect("Benchmark artifact had no executable."),
                };
                benchmarks.push(bench);
            }
        }
    }

    if !(cargo.wait()?.success()) {
        Err(CompileError::CompileFailed)
    } else {
        Ok(benchmarks)
    }
}
