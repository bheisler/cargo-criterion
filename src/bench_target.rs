use crate::connection::{AxisScale, Connection, IncomingMessage, PlotConfiguration};
use crate::model::Model;
use crate::report::{BenchmarkId, Report, ReportContext};
use anyhow::{anyhow, Context, Result};
use std::ffi::OsString;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

/// Structure representing a compiled benchmark executable.
#[derive(Debug)]
pub struct BenchTarget {
    pub name: String,
    pub executable: PathBuf,
}
impl BenchTarget {
    /// Launches this benchmark target with the given additional arguments.
    ///
    /// Opens a localhost socket on an arbitrary port. This port is passed to the target in an
    /// environment variable; recent versions of Criterion.rs will connect to that socket and use
    /// it to communicate information about the status of the run and the measurements taken. Other
    /// benchmark frameworks (or older versions of Criterion.rs) will ignore the port and perform
    /// their benchmarks as they normally do.
    ///
    /// The report will be notified about important events happening with the benchmark and the
    /// model will be updated with the new benchmark IDs and measurements as we go. This function
    /// will block until the benchmark target terminates.
    pub fn execute(
        &self,
        criterion_home: &PathBuf,
        additional_args: &[OsString],
        report: &dyn Report,
        model: &mut Model,
    ) -> Result<()> {
        let listener = TcpListener::bind("localhost:0")
            .context("Unable to open socket to connect to Criterion.rs")?;
        listener
            .set_nonblocking(true)
            .context("Unable to set socket to nonblocking")?;

        let addr = listener
            .local_addr()
            .context("Unable to get local address of socket")?;
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

        debug!("Running '{:?}'", command);

        let mut child = command
            .spawn()
            .with_context(|| format!("Unable to launch bench target {}", self.name))?;

        loop {
            match listener.accept() {
                Ok((socket, _)) => {
                    let conn = Connection::new(socket).with_context(|| {
                        format!("Unable to open connection to bench target {}", self.name)
                    })?;
                    return self.communicate(&mut child, conn, report, criterion_home, model);
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No connection yet, try again in a bit.
                }
                Err(e) => {
                    return Err(e).context("Unable to accept connection to socket");
                }
            };

            match child.try_wait() {
                Err(e) => {
                    return Err(e).context(format!(
                        "Failed to wait for non-Criterion.rs benchmark target {}",
                        self.name
                    ));
                }
                Ok(Some(exit_status)) => {
                    if exit_status.success() {
                        return Ok(());
                    } else {
                        return Err(anyhow!(
                            "Non-Criterion.rs benchmark target {} exited with error code {:?}",
                            self.name,
                            exit_status.code()
                        ));
                    }
                }
                Ok(None) => (), // Child still running, keep trying.
            };

            // Wait a bit then poll again.
            std::thread::yield_now();
        }
    }

    /// This function is called when a benchmark connects to the socket. It interacts with the
    /// benchmark target to receive information about the measurements and inform the report and
    /// model about the benchmarks. This function returns when the benchmark target terminates.
    fn communicate(
        &self,
        child: &mut Child,
        mut conn: Connection,
        report: &dyn Report,
        criterion_home: &std::path::Path,
        model: &mut Model,
    ) -> Result<()> {
        let mut context = ReportContext {
            output_directory: criterion_home.join("reports"),
            plot_config: PlotConfiguration {
                summary_scale: AxisScale::Linear,
            },
        };
        let mut any_from_group_executed = false;
        loop {
            let message = conn.recv().with_context(|| {
                format!(
                    "Failed to receive message from Criterion.rs benchmark target {}",
                    self.name
                )
            })?;
            if message.is_none() {
                return Ok(());
            }
            let message = message.unwrap();
            match message {
                IncomingMessage::BeginningBenchmarkGroup { group } => {
                    any_from_group_executed = false;
                    model.check_benchmark_group(&self.name, &group);
                }
                IncomingMessage::FinishedBenchmarkGroup { group } => {
                    let benchmark_group = model.add_benchmark_group(&self.name, &group);
                    {
                        let formatter =
                            crate::value_formatter::ConnectionValueFormatter::new(&mut conn);
                        report.summarize(&context, &group, benchmark_group, &formatter);
                        if any_from_group_executed {
                            report.group_separator();
                        }
                    }
                }
                IncomingMessage::BeginningBenchmark { id } => {
                    any_from_group_executed = true;
                    let mut id = id.into();
                    model.add_benchmark_id(&self.name, &mut id);
                    self.run_benchmark(&mut conn, report, model, id, &mut context)?;
                }
                IncomingMessage::SkippingBenchmark { id } => {
                    let mut id = id.into();
                    model.add_benchmark_id(&self.name, &mut id);
                }
                other => panic!("Unexpected message {:?}", other),
            }

            match child.try_wait() {
                Err(e) => {
                    return Err(e).context(format!(
                        "Failed to poll Criterion.rs child process {}",
                        self.name
                    ));
                }
                Ok(Some(exit_status)) => {
                    if exit_status.success() {
                        return Ok(());
                    } else {
                        return Err(anyhow!(
                            "Criterion.rs benchmark target {} exited with error code {:?}",
                            self.name,
                            exit_status.code()
                        ));
                    }
                }
                Ok(None) => continue,
            };
        }
    }

    /// Helper function for communicating with the benchmark target about a single benchmark.
    fn run_benchmark(
        &self,
        conn: &mut Connection,
        report: &dyn Report,
        model: &mut Model,
        id: BenchmarkId,
        context: &mut ReportContext,
    ) -> Result<()> {
        report.benchmark_start(&id, &context);

        loop {
            let message = conn.recv().with_context(|| {
                format!(
                    "Failed to receive message from Criterion.rs benchmark {}",
                    self.name
                )
            })?;
            let message = match message {
                Some(message) => message,
                None => return Ok(()),
            };
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
                    plot_config,
                    sampling_method,
                    benchmark_config,
                } => {
                    context.plot_config = plot_config;
                    report.analysis(&id, &context);

                    let avg_values: Vec<f64> = iters
                        .iter()
                        .zip(times.iter())
                        .map(|(iter, time)| *time / (*iter as f64))
                        .collect();

                    let saved_stats = model.get_last_sample(&id).cloned();

                    let measured_data = crate::analysis::analysis(
                        &(benchmark_config).into(),
                        id.throughput.clone(),
                        crate::analysis::MeasuredValues {
                            iteration_count: &iters,
                            sample_values: &times,
                            avg_values: &avg_values,
                        },
                        saved_stats.as_ref().map(|stats| {
                            let measured_values = crate::analysis::MeasuredValues {
                                iteration_count: &stats.iterations,
                                sample_values: &stats.values,
                                avg_values: &stats.avg_values,
                            };
                            (measured_values, &stats.estimates)
                        }),
                        sampling_method,
                    );

                    if let Err(e) = model.benchmark_complete(&id, &measured_data) {
                        error!(
                            "Failed to save results for target {} benchmark {}: {}",
                            self.name,
                            id.as_title(),
                            e
                        );
                    }

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
