//! A Cargo extension for running [Criterion.rs] benchmarks and reporting the results.
//!
//! This crate is a Cargo extension which can be used as a replacement for `cargo bench` when
//! running [Criterion.rs] benchmarks.

#![cfg_attr(
    feature = "cargo-clippy",
    allow(
        clippy::just_underscores_and_digits, // Used in the stats code
        clippy::transmute_ptr_to_ptr, // Used in the stats code
    )
)]

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate log;

#[macro_use]
mod macros_private;

mod analysis;
mod bench_target;
mod compile;
mod config;
mod connection;
mod estimate;
mod format;
mod html;
mod kde;
mod model;
mod plot;
mod report;
mod stats;
mod value_formatter;

use crate::config::{OutputFormat, PlottingBackend, SelfConfig, TextColor};
use crate::connection::{AxisScale, PlotConfiguration};
use crate::plot::Plotter;
use crate::report::{Report, ReportContext};
use anyhow::Error;
use lazy_static::lazy_static;

lazy_static! {
    static ref DEBUG_ENABLED: bool = std::env::var_os("CRITERION_DEBUG").is_some();
}

fn debug_enabled() -> bool {
    *DEBUG_ENABLED
}

/// Configures the logger based on the debug environment variable.
fn configure_log() {
    use simplelog::*;
    let filter = if debug_enabled() {
        LevelFilter::max()
    } else {
        LevelFilter::Warn
    };
    TermLogger::init(filter, Default::default(), TerminalMode::Stderr).unwrap();
}

// TODO: Add a section to the user guide
// TODO: Configure CI for cargo-criterion
// TODO: Add support for timelines & historical charts
// TODO: Add machine-readable output
// TODO: Notify burntsushi/critcmp about the internal file format change after I've added support for flat sampling
// TODO: Look through github issues and implement them in cargo-criterion if sensible.
// TODO: Split cargo-criterion into separate repo?
// TODO: Write unit tests for serialization.

/// Main entry point for cargo-criterion.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    configure_log();

    // First, load the config file and parse the command-line args.
    let configuration = config::configure()?;
    let self_config = &configuration.self_config;

    // Launch cargo to compile the crate and produce a list of the benchmark targets to run.
    let bench_targets = compile::compile(&configuration.cargo_args)?;

    // Load the saved measurements from the last run.
    let mut run_model = model::Model::load(self_config.criterion_home.clone(), "main".into());

    // Set up the reports. These receive notifications as the benchmarks proceed and generate output for the user.
    let cli_report = configure_cli_output(self_config);
    let bencher_report = crate::report::BencherReport;
    let html_report = get_plotter(self_config)?.map(|plotter| crate::html::Html::new(plotter));

    let mut reports: Vec<&dyn crate::report::Report> = Vec::new();
    match self_config.output_format {
        OutputFormat::Bencher => reports.push(&bencher_report),
        OutputFormat::Criterion | OutputFormat::Quiet | OutputFormat::Verbose => {
            reports.push(&cli_report)
        }
    }
    if let Some(html_report) = &html_report {
        reports.push(html_report);
    }
    let reports = crate::report::Reports::new(reports);

    if self_config.do_run {
        // Execute each benchmark target, updating the model as we go.
        for bench in bench_targets {
            info!("Executing {} - {:?}", bench.name, bench.executable);
            let err = bench.execute(
                &self_config.criterion_home,
                &configuration.additional_args,
                &reports,
                &mut run_model,
            );

            if let Err(err) = err {
                if self_config.do_fail_fast {
                    return Err(err.into());
                } else {
                    error!(
                        "Failed to execute benchmark target {}:\n{}",
                        bench.name, err
                    );
                }
            }
        }

        // Generate the overall summary report using all of the records in the model.
        let final_context = ReportContext {
            output_directory: self_config.criterion_home.join("reports"),
            plot_config: PlotConfiguration {
                summary_scale: AxisScale::Linear,
            },
        };

        reports.final_summary(&final_context, &run_model);
    }
    Ok(())
}

/// Configure and return a Report object that prints benchmark information to the command-line.
fn configure_cli_output(self_config: &crate::config::SelfConfig) -> crate::report::CliReport {
    let stdout_isatty = atty::is(atty::Stream::Stdout);
    let mut enable_text_overwrite = stdout_isatty && !debug_enabled();
    let enable_text_coloring = match self_config.text_color {
        TextColor::Auto => stdout_isatty,
        TextColor::Never => {
            enable_text_overwrite = false;
            false
        }
        TextColor::Always => true,
    };

    let show_differences = match self_config.output_format {
        OutputFormat::Criterion | OutputFormat::Verbose => true,
        OutputFormat::Quiet | OutputFormat::Bencher => false,
    };
    let verbose = match self_config.output_format {
        OutputFormat::Verbose => true,
        OutputFormat::Criterion | OutputFormat::Quiet | OutputFormat::Bencher => debug_enabled(),
    };

    crate::report::CliReport::new(
        enable_text_overwrite,
        enable_text_coloring,
        show_differences,
        verbose,
    )
}

/// Configure and return a Gnuplot plotting backend, if available.
#[cfg(feature = "gnuplot_backend")]
fn gnuplot_plotter() -> Result<Box<dyn Plotter>, Error> {
    match criterion_plot::version() {
        Ok(_) => Ok(Box::new(crate::plot::Gnuplot::new())),
        Err(_) => Err(anyhow::anyhow!("Gnuplot is not available. To continue, either install Gnuplot or allow cargo-criterion to fall back to using plotters.")),
    }
}

/// Gnuplot support was not compiled in, so the gnuplot backend is not available.
#[cfg(not(feature = "gnuplot_backend"))]
fn gnuplot_plotter() -> Result<Box<dyn Plotter>, Error> {
    anyhow::bail!("Gnuplot backend is disabled. To use gnuplot backend, install cargo-criterion with the 'gnuplot_backend' feature enabled")
}

/// Configure and return a Plotters plotting backend.
#[cfg(feature = "plotters_backend")]
fn plotters_plotter() -> Result<Box<dyn Plotter>, Error> {
    Ok(Box::new(crate::plot::PlottersBackend))
}

/// Plotters support was not compiled in, so the plotters backend is not available.
#[cfg(not(feature = "plotters_backend"))]
fn plotters_plotter() -> Result<Box<dyn Plotter>, Error> {
    anyhow::bail!("Plotters backend is disabled. To use plotters backend, install cargo-criterion with the 'plotters_backend' feature enabled")
}

/// Configure and return a plotting backend.
#[cfg(any(feature = "gnuplot_backend", feature = "plotters_backend"))]
fn get_plotter(config: &SelfConfig) -> Result<Option<Box<dyn Plotter>>, Error> {
    match config.plotting_backend {
        PlottingBackend::Gnuplot => gnuplot_plotter().map(|p| Some(p)),
        PlottingBackend::Plotters => plotters_plotter().map(|p| Some(p)),
        PlottingBackend::Auto => gnuplot_plotter().or(plotters_plotter()).map(|p| Some(p)),
        PlottingBackend::Disabled => Ok(None),
    }
}

/// No plotting backend was compiled in. Plotting is disabled.
#[cfg(not(any(feature = "gnuplot_backend", feature = "plotters_backend")))]
fn get_plotter(config: &SelfConfig) -> Result<Option<Box<dyn Plotter>>, Error> {
    match config.plotting_backend {
        PlottingBackend::Disabled => Ok(None),
        _ => anyhow::bail!("No plotting backend is available. At least one of the 'gnuplot_backend' or 'plotters_backend' features must be included.")
    }
}

/// Helper trait which adds a function for converting Duration to nanoseconds.
trait DurationExt {
    fn to_nanos(&self) -> u64;
}

const NANOS_PER_SEC: u64 = 1_000_000_000;

impl DurationExt for std::time::Duration {
    fn to_nanos(&self) -> u64 {
        self.as_secs() * NANOS_PER_SEC + u64::from(self.subsec_nanos())
    }
}
