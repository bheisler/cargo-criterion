use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use std::convert::TryFrom;
use std::io::{ErrorKind, Read, Write};
use std::mem::size_of;
use std::net::TcpStream;

#[derive(Debug)]
pub enum ConnectionError {
    HelloFailed(&'static str),
}
impl std::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionError::HelloFailed(error) => {
                write!(f, "Failed to connect to Criterion.rs benchmark:\n{}", error)
            }
        }
    }
}
impl std::error::Error for ConnectionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConnectionError::HelloFailed(_) => None,
        }
    }
}

#[derive(Debug)]
#[repr(u16)]
enum ProtocolFormat {
    Cbor = 1,
}
impl ProtocolFormat {
    fn from_u16(format: u16) -> Result<Self, ConnectionError> {
        match format {
            1 => Ok(ProtocolFormat::Cbor),
            _ => Err(ConnectionError::HelloFailed("Unknown format value sent by Criterion.rs benchmark; please update cargo-criterion.")),
        }
    }
}

const RUNNER_MAGIC_NUMBER: &str = "cargo-criterion";
const RUNNER_HELLO_SIZE: usize = RUNNER_MAGIC_NUMBER.len() // magic number
    + (size_of::<u8>() * 3); // version number

const BENCHMARK_MAGIC_NUMBER: &str = "Criterion";
const BENCHMARK_HELLO_SIZE: usize = BENCHMARK_MAGIC_NUMBER.len() // magic number
    + (size_of::<u8>() * 3) // version number
    + size_of::<u16>() // protocol version
    + size_of::<u16>(); // protocol format

/// This struct represents an open socket connection to a Criterion.rs benchmark.
///
/// When the benchmark connects, a small handshake is performed to verify that we've connected to
/// the right process and that the version of Criterion.rs on the other side is valid, etc.
/// Afterwards, we exchange messages (currently using CBOR) with the benchmark.
#[derive(Debug)]
pub struct Connection {
    socket: TcpStream,
    receive_buffer: Vec<u8>,
    send_buffer: Vec<u8>,

    criterion_rs_version: [u8; 3],
    protocol_version: u16,
    protocol_format: ProtocolFormat,
}
impl Connection {
    /// Perform the connection handshake and wrap the TCP stream in a Connection object if successful.
    pub fn new(mut socket: TcpStream) -> Result<Self> {
        // Send the runner-hello message.
        let mut hello_buf = [0u8; RUNNER_HELLO_SIZE];
        hello_buf[0..RUNNER_MAGIC_NUMBER.len()].copy_from_slice(RUNNER_MAGIC_NUMBER.as_bytes());
        let i = RUNNER_MAGIC_NUMBER.len();
        hello_buf[i] = env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap();
        hello_buf[i + 1] = env!("CARGO_PKG_VERSION_MINOR").parse().unwrap();
        hello_buf[i + 2] = env!("CARGO_PKG_VERSION_PATCH").parse().unwrap();

        socket.write_all(&hello_buf)?;

        // Read the benchmark hello message.
        let mut hello_buf = [0u8; BENCHMARK_HELLO_SIZE];
        socket.read_exact(&mut hello_buf)?;

        if &hello_buf[0..BENCHMARK_MAGIC_NUMBER.len()] != BENCHMARK_MAGIC_NUMBER.as_bytes() {
            return Err(
                ConnectionError::HelloFailed("Not connected to a Criterion.rs benchmark.").into(),
            );
        }
        let mut i = BENCHMARK_MAGIC_NUMBER.len();
        let criterion_rs_version = [hello_buf[i], hello_buf[i + 1], hello_buf[i + 2]];
        i += 3;
        let protocol_version = u16::from_be_bytes([hello_buf[i], hello_buf[i + 1]]);
        i += 2;
        let protocol_format = u16::from_be_bytes([hello_buf[i], hello_buf[i + 1]]);
        let protocol_format = ProtocolFormat::from_u16(protocol_format)?;

        info!("Criterion.rs version: {:?}", criterion_rs_version);
        info!("Protocol version: {}", protocol_version);
        info!("Protocol Format: {:?}", protocol_format);

        Ok(Connection {
            socket,
            receive_buffer: vec![],
            send_buffer: vec![],

            criterion_rs_version,
            protocol_version,
            protocol_format,
        })
    }

    /// Receive a message from the benchmark. If the benchmark has closed the connection, returns
    /// Ok(None).
    pub fn recv<T: DeserializeOwned>(&mut self) -> Result<Option<T>> {
        let mut length_buf = [0u8; 4];
        match self.socket.read_exact(&mut length_buf) {
            Err(err) if err.kind() == ErrorKind::UnexpectedEof => return Ok(None),
            Err(err) => return Err(err.into()),
            Ok(val) => val,
        };
        let length = u32::from_be_bytes(length_buf);
        self.receive_buffer.resize(length as usize, 0u8);
        self.socket
            .read_exact(&mut self.receive_buffer)
            .context("Failed to read message from Criterion.rs benchmark")?;
        let value: T = serde_cbor::from_slice(&self.receive_buffer)
            .context("Failed to parse message from Criterion.rs benchmark")?;
        Ok(Some(value))
    }

    /// Send a message to the benchmark.
    pub fn send(&mut self, message: &OutgoingMessage) -> Result<()> {
        self.send_buffer.truncate(0);
        serde_cbor::to_writer(&mut self.send_buffer, message)
            .with_context(|| format!("Failed to serialize message {:?}", message))?;
        let size = u32::try_from(self.send_buffer.len()).unwrap();
        let length_buf = size.to_be_bytes();
        self.socket
            .write_all(&length_buf)
            .context("Failed to send message header")?;
        self.socket
            .write_all(&self.send_buffer)
            .context("Failed to send message")?;
        Ok(())
    }
}

// All of these structs are used to communicate with Criterion.rs. The benchmarks may be running
// any version of Criterion.rs that supports cargo-criterion, so backwards compatibility is
// important.

#[derive(Debug, Deserialize)]
pub enum IncomingMessage {
    // Benchmark lifecycle messages
    BeginningBenchmarkGroup {
        group: String,
    },
    FinishedBenchmarkGroup {
        group: String,
    },
    BeginningBenchmark {
        id: RawBenchmarkId,
    },
    SkippingBenchmark {
        id: RawBenchmarkId,
    },
    Warmup {
        nanos: f64,
    },
    MeasurementStart {
        sample_count: u64,
        estimate_ns: f64,
        iter_count: u64,
    },
    MeasurementComplete {
        iters: Vec<f64>,
        times: Vec<f64>,
        plot_config: PlotConfiguration,
        sampling_method: SamplingMethod,
        benchmark_config: BenchmarkConfig,
    },
    // Value formatting responses
    FormattedValue {
        value: String,
    },
    ScaledValues {
        scaled_values: Vec<f64>,
        unit: String,
    },
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub enum OutgoingMessage<'a> {
    FormatValue {
        value: f64,
    },
    FormatThroughput {
        value: f64,
        throughput: Throughput,
    },
    ScaleValues {
        typical_value: f64,
        values: &'a [f64],
    },
    ScaleThroughputs {
        typical_value: f64,
        values: &'a [f64],
        throughput: Throughput,
    },
    ScaleForMachines {
        values: &'a [f64],
    },
    Continue,
}

#[derive(Debug, Deserialize)]
pub struct RawBenchmarkId {
    group_id: String,
    function_id: Option<String>,
    value_str: Option<String>,
    throughput: Vec<Throughput>,
}
impl From<RawBenchmarkId> for crate::report::BenchmarkId {
    fn from(other: RawBenchmarkId) -> Self {
        crate::report::BenchmarkId::new(
            other.group_id,
            other.function_id,
            other.value_str,
            other.throughput.first().cloned(),
        )
    }
}

#[derive(Debug, Deserialize, Clone, Copy)]
pub enum AxisScale {
    Linear,
    Logarithmic,
}

#[derive(Debug, Deserialize)]
pub struct PlotConfiguration {
    pub summary_scale: AxisScale,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Throughput {
    Bytes(u64),
    Elements(u64),
}

#[derive(Debug, Deserialize)]
pub enum SamplingMethod {
    Linear,
    Flat,
}
impl SamplingMethod {
    pub fn is_linear(&self) -> bool {
        matches!(self, SamplingMethod::Linear)
    }
}

#[derive(Debug, Deserialize)]
struct Duration {
    secs: u64,
    nanos: u32,
}
#[derive(Debug, Deserialize)]
pub struct BenchmarkConfig {
    confidence_level: f64,
    measurement_time: Duration,
    noise_threshold: f64,
    nresamples: usize,
    sample_size: usize,
    significance_level: f64,
    warm_up_time: Duration,
}
impl From<BenchmarkConfig> for crate::analysis::BenchmarkConfig {
    fn from(other: BenchmarkConfig) -> Self {
        crate::analysis::BenchmarkConfig {
            confidence_level: other.confidence_level,
            measurement_time: std::time::Duration::new(
                other.measurement_time.secs,
                other.measurement_time.nanos,
            ),
            noise_threshold: other.noise_threshold,
            nresamples: other.nresamples,
            sample_size: other.sample_size,
            significance_level: other.significance_level,
            warm_up_time: std::time::Duration::new(
                other.warm_up_time.secs,
                other.warm_up_time.nanos,
            ),
        }
    }
}
