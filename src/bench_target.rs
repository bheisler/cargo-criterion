use crate::connection::{
    Connection, ConnectionError, IncomingMessage, MessageError, OutgoingMessage,
};
use crate::model::Model;
use crate::report::{BenchmarkId, Report};
use std::ffi::OsString;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};

#[derive(Debug)]
pub enum TargetError {
    IoError(String, std::io::Error),
    TargetFailed(String, ExitStatus),
    MessageError(String, MessageError),
    ConnectionError(String, ConnectionError),
}
impl std::fmt::Display for TargetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TargetError::TargetFailed(target_name, exit_status) => write!(
                f,
                "Benchmark target '{}' returned an error ({}).",
                target_name, exit_status
            ),
            TargetError::IoError(target_name, io_error) => write!(
                f,
                "Unexpected IO Error while running benchmark target '{}':\n{}",
                target_name, io_error
            ),
            TargetError::MessageError(target_name, message_error) => write!(
                f,
                "Unexpected error communicating with benchmark target '{}':\n{}",
                target_name, message_error
            ),
            TargetError::ConnectionError(target_name, connection_error) => write!(
                f,
                "Unexpected error connecting to benchmark target '{}':\n{}",
                target_name, connection_error
            ),
        }
    }
}
impl std::error::Error for TargetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TargetError::TargetFailed(_, _) => None,
            TargetError::IoError(_, io_error) => Some(io_error),
            TargetError::MessageError(_, message_error) => Some(message_error),
            TargetError::ConnectionError(_, connection_error) => Some(connection_error),
        }
    }
}

/// Structure representing a compiled benchmark executable.
#[derive(Debug)]
pub struct BenchTarget {
    pub name: String,
    pub executable: PathBuf,
}
impl BenchTarget {
    pub fn execute(
        &self,
        criterion_home: &PathBuf,
        additional_args: &[OsString],
        report: &dyn Report,
        model: &mut Model,
    ) -> Result<(), TargetError> {
        let listener = TcpListener::bind("localhost:0")
            .map_err(|err| TargetError::IoError(self.name.clone(), err))?;
        listener
            .set_nonblocking(true)
            .map_err(|err| TargetError::IoError(self.name.clone(), err))?;

        let addr = listener
            .local_addr()
            .map_err(|err| TargetError::IoError(self.name.clone(), err))?;
        let port = addr.port();

        let mut command = Command::new(&self.executable);
        command
            .arg("--bench")
            .args(additional_args)
            .env("CRITERION_HOME", criterion_home)
            .env("CARGO_CRITERION_PORT", &port.to_string())
            .stdin(Stdio::null())
            .stderr(Stdio::inherit())
            .stdout(Stdio::inherit());

        println!("{:?}", command);

        let mut child = command
            .spawn()
            .map_err(|err| TargetError::IoError(self.name.clone(), err))?;

        loop {
            match listener.accept() {
                Ok((socket, _)) => {
                    let conn = Connection::new(socket)
                        .map_err(|err| TargetError::ConnectionError(self.name.clone(), err))?;
                    return self.communicate(&mut child, conn, report, criterion_home, model);
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No connection yet, try again in a bit.
                }
                Err(e) => {
                    println!("Failed to accept connection");
                    return Err(TargetError::IoError(self.name.clone(), e));
                }
            };

            match child.try_wait() {
                Err(e) => {
                    println!("Failed to poll child process");
                    return Err(TargetError::IoError(self.name.clone(), e));
                }
                Ok(Some(exit_status)) => {
                    if exit_status.success() {
                        println!("Child exited successfully");
                        return Ok(());
                    } else {
                        println!("Child terminated");
                        return Err(TargetError::TargetFailed(self.name.clone(), exit_status));
                    }
                }
                Ok(None) => (), // Child still running, keep trying.
            };

            // Wait a bit then poll again.
            std::thread::yield_now();
        }
    }

    fn communicate(
        &self,
        child: &mut Child,
        mut conn: Connection,
        report: &dyn Report,
        criterion_home: &std::path::Path,
        model: &mut Model,
    ) -> Result<(), TargetError> {
        loop {
            let message = conn
                .recv()
                .map_err(|err| TargetError::MessageError(self.name.clone(), err))?;
            if message.is_none() {
                return Ok(());
            }
            let message = message.unwrap();
            match message {
                IncomingMessage::BeginningBenchmarkGroup { group } => {
                    model.check_benchmark_group(&group);
                }
                IncomingMessage::FinishedBenchmarkGroup { group } => {
                    model.add_benchmark_group(&self.name, group);
                }
                IncomingMessage::BeginningBenchmark { id } => {
                    let mut id = id.into();
                    model.add_benchmark_id(&self.name, &mut id);
                    self.run_benchmark(&mut conn, report, criterion_home, model, id)?;
                }
                IncomingMessage::SkippingBenchmark { id } => {
                    let mut id = id.into();
                    model.add_benchmark_id(&self.name, &mut id);
                }
                other => panic!("Unexpected message {:?}", other),
            }

            match child.try_wait() {
                Err(e) => {
                    println!("Failed to poll Criterion.rs child process");
                    return Err(TargetError::IoError(self.name.clone(), e));
                }
                Ok(Some(exit_status)) => {
                    if exit_status.success() {
                        println!("Criterion.rs child exited successfully");
                        return Ok(());
                    } else {
                        println!("Criterion.rs child terminated unsuccessfully");
                        return Err(TargetError::TargetFailed(self.name.clone(), exit_status));
                    }
                }
                Ok(None) => continue,
            };
        }
    }

    fn run_benchmark(
        &self,
        conn: &mut Connection,
        report: &dyn Report,
        criterion_home: &std::path::Path,
        model: &mut Model,
        id: BenchmarkId,
    ) -> Result<(), TargetError> {
        let context = crate::report::ReportContext {
            output_directory: criterion_home.to_owned(),
        };
        report.benchmark_start(&id, &context);

        conn.send(&OutgoingMessage::RunBenchmark)
            .map_err(|err| TargetError::MessageError(self.name.clone(), err))?;

        loop {
            let message = conn
                .recv()
                .map_err(|err| TargetError::MessageError(self.name.clone(), err))?;
            if message.is_none() {
                return Ok(());
            }
            let message = message.unwrap();
            match message {
                IncomingMessage::Warmup { nanos } => {
                    report.warmup(&id, &context, nanos);
                }
                IncomingMessage::MeasurementStart {
                    sample_count,
                    estimate_ns,
                    iter_count,
                } => {
                    report.measurement_start(&id, &context, sample_count, estimate_ns, iter_count);
                }
                IncomingMessage::MeasurementComplete {
                    iters,
                    times,
                    plot_config: _,
                    sampling_method: _,
                    benchmark_config,
                } => {
                    report.analysis(&id, &context);

                    let avg_values: Vec<f64> = iters
                        .iter()
                        .zip(times.iter())
                        .map(|(iter, time)| *time / (*iter as f64))
                        .collect();

                    let measured_data = crate::analysis::analysis(
                        &id,
                        &(benchmark_config).into(),
                        id.throughput.clone(),
                        crate::analysis::MeasuredValues {
                            iteration_count: &iters,
                            sample_values: &times,
                            avg_values: &avg_values,
                        },
                        None,
                    );

                    model.benchmark_complete(&id);

                    {
                        let formatter = crate::value_formatter::ConnectionValueFormatter::new(conn);
                        report.measurement_complete(&id, &context, &measured_data, &formatter);
                    }
                    return Ok(());
                }
                other => panic!("Unexpected message {:?}", other),
            }
        }
    }
}
