use crate::connection::{
    AxisScale, Connection, IncomingMessage, OutgoingMessage, PlotConfiguration,
};
use crate::model::Model;
use crate::report::{BenchmarkId, Report};
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
    pub fn execute(
        &self,
        criterion_home: &PathBuf,
        additional_args: &[OsString],
        report: &dyn Report,
        model: &mut Model,
    ) -> Result<()> {
        let listener = TcpListener::bind("localhost:0")
            .context("Unable to open socket to conect to Criterion.rs")?;
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

    fn communicate(
        &self,
        child: &mut Child,
        mut conn: Connection,
        report: &dyn Report,
        criterion_home: &std::path::Path,
        model: &mut Model,
    ) -> Result<()> {
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
                    model.check_benchmark_group(&self.name, &group);
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

    fn run_benchmark(
        &self,
        conn: &mut Connection,
        report: &dyn Report,
        criterion_home: &std::path::Path,
        model: &mut Model,
        id: BenchmarkId,
    ) -> Result<()> {
        let mut context = crate::report::ReportContext {
            output_directory: criterion_home.join("reports"),
            plot_config: PlotConfiguration {
                summary_scale: AxisScale::Linear,
            },
        };
        report.benchmark_start(&id, &context);

        conn.send(&OutgoingMessage::RunBenchmark).with_context(|| {
            format!(
                "Failed to send message to Criterion.rs benchmark {}",
                self.name
            )
        })?;

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
                    sampling_method: _,
                    benchmark_config,
                } => {
                    context.plot_config = plot_config;
                    report.analysis(&id, &context);

                    let avg_values: Vec<f64> = iters
                        .iter()
                        .zip(times.iter())
                        .map(|(iter, time)| *time / (*iter as f64))
                        .collect();

                    let saved_stats = if !report.requires_comparison() {
                        None
                    } else {
                        model.load_last_sample(&id).unwrap_or_else(|e| {
                            error!("Failed to load previous sample: {:?}", e);
                            None
                        })
                    };

                    let measured_data = crate::analysis::analysis(
                        &id,
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
