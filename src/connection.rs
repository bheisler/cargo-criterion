use serde::de::DeserializeOwned;
use std::convert::TryFrom;
use std::io::{ErrorKind, Read, Write};
use std::mem::size_of;
use std::net::TcpStream;

#[derive(Debug)]
pub enum MessageError {
    SerializationError(serde_cbor::Error),
    IoError(std::io::Error),
}
impl From<serde_cbor::Error> for MessageError {
    fn from(other: serde_cbor::Error) -> Self {
        MessageError::SerializationError(other)
    }
}
impl From<std::io::Error> for MessageError {
    fn from(other: std::io::Error) -> Self {
        MessageError::IoError(other)
    }
}
impl std::fmt::Display for MessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageError::SerializationError(error) => write!(
                f,
                "Failed to serialize or deserialize message to Criterion.rs benchmark:\n{}",
                error
            ),
            MessageError::IoError(error) => write!(
                f,
                "Failed to read or write message to Criterion.rs benchmark:\n{}",
                error
            ),
        }
    }
}
impl std::error::Error for MessageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            MessageError::SerializationError(err) => Some(err),
            MessageError::IoError(err) => Some(err),
        }
    }
}

#[derive(Debug)]
pub enum ConnectionError {
    HelloFailed(&'static str),
    IoError(std::io::Error),
}
impl From<std::io::Error> for ConnectionError {
    fn from(other: std::io::Error) -> Self {
        ConnectionError::IoError(other)
    }
}
impl std::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionError::HelloFailed(error) => {
                write!(f, "Failed to connect to Criterion.rs benchmark:\n{}", error)
            }
            ConnectionError::IoError(error) => write!(
                f,
                "Failed to read or write message to Criterion.rs benchmark:\n{}",
                error
            ),
        }
    }
}
impl std::error::Error for ConnectionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConnectionError::HelloFailed(_) => None,
            ConnectionError::IoError(err) => Some(err),
        }
    }
}

#[derive(Debug)]
#[repr(u16)]
enum ProtocolFormat {
    CBOR = 1,
}
impl ProtocolFormat {
    fn from_u16(format: u16) -> Result<Self, ConnectionError> {
        match format {
            1 => Ok(ProtocolFormat::CBOR),
            _ => Err(ConnectionError::HelloFailed("Unknown format")),
        }
    }
}

const MAGIC_NUMBER: &str = "Criterion";
const HELLO_SIZE: usize = MAGIC_NUMBER.len() // magic number
    + (size_of::<u8>() * 3) // criterion.rs version
    + size_of::<u16>() // protocol version
    + size_of::<u16>(); // protocol format

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
    pub fn new(mut socket: TcpStream) -> Result<Self, ConnectionError> {
        // Read the connection hello message right away.
        let mut hello_buf = [0u8; HELLO_SIZE];
        socket.read_exact(&mut hello_buf)?;

        if &hello_buf[0..MAGIC_NUMBER.len()] != MAGIC_NUMBER.as_bytes() {
            return Err(ConnectionError::HelloFailed(
                "Not connected to a Criterion.rs benchmark.",
            ));
        }
        let mut i = MAGIC_NUMBER.len();
        let criterion_rs_version = [hello_buf[i], hello_buf[i + 1], hello_buf[i + 2]];
        i += 3;
        let protocol_version = u16::from_be_bytes([hello_buf[i], hello_buf[i + 1]]);
        i += 2;
        let protocol_format = u16::from_be_bytes([hello_buf[i], hello_buf[i + 1]]);
        let protocol_format = ProtocolFormat::from_u16(protocol_format)?;

        println!("Criterion.rs version: {:?}", criterion_rs_version);
        println!("Protocol version: {}", protocol_version);
        println!("Protocol Format: {:?}", protocol_format);

        Ok(Connection {
            socket,
            receive_buffer: vec![],
            send_buffer: vec![],

            criterion_rs_version,
            protocol_version,
            protocol_format,
        })
    }

    pub fn recv<T: DeserializeOwned>(&mut self) -> Result<Option<T>, MessageError> {
        let mut length_buf = [0u8; 4];
        match self.socket.read_exact(&mut length_buf) {
            Err(err) if err.kind() == ErrorKind::UnexpectedEof => return Ok(None),
            Err(err) => return Err(err.into()),
            Ok(val) => val,
        };
        let length = u32::from_be_bytes(length_buf);
        self.receive_buffer.resize(length as usize, 0u8);
        self.socket.read_exact(&mut self.receive_buffer)?;
        let value: T = serde_cbor::from_slice(&self.receive_buffer)?;
        Ok(Some(value))
    }

    pub fn send(&mut self, message: &OutgoingMessage) -> Result<(), MessageError> {
        self.send_buffer.truncate(0);
        serde_cbor::to_writer(&mut self.send_buffer, message)?;
        let size = u32::try_from(self.send_buffer.len()).unwrap();
        let length_buf = size.to_be_bytes();
        self.socket.write_all(&length_buf)?;
        self.socket.write_all(&self.send_buffer)?;
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
//#[serde(tag = "event")]
pub enum IncomingMessage {
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
        id: RawBenchmarkId,
        nanos: f64,
    },
    MeasurementStart {
        id: RawBenchmarkId,
        sample_count: u64,
        estimate_ns: f64,
        iter_count: u64,
        added_runner: Option<f64>,
    },
    MeasurementComplete {
        id: RawBenchmarkId,
        iters: Vec<u64>,
        times: Vec<f64>,
    },
}

#[derive(Debug, Serialize)]
//#[serde(tag = "event")]
pub enum OutgoingMessage {
    RunBenchmark,
    SkipBenchmark,
}

#[derive(Debug, Deserialize)]
pub struct RawBenchmarkId {
    group_id: String,
    function_id: Option<String>,
    value_str: Option<String>,
}
