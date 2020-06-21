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

fn configure_log() {
    use simplelog::*;
    let filter = if debug_enabled() {
        LevelFilter::max()
    } else {
        LevelFilter::Warn
    };
    TermLogger::init(filter, Default::default(), TerminalMode::Stderr).unwrap();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    configure_log();

    let configuration = config::configure()?;
    let self_config = &configuration.self_config;

    let bench_targets = compile::compile(&configuration.cargo_args)?;

    // TODO: Make sure that test & profile mode still works
    // TODO: Handle filter requests properly
    // TODO: Add machine-readable output
    // TODO: Add alternate sampling modes (at least in the messaging)
    // TODO: Add support (at least in the messaging, so we can add it later) for multiple throughputs
    // TODO: Document the code
    // TODO: Add a section to the user guide
    // TODO: Add support for timelines
    // TODO: Reorganize report files.
    // TODO: Stop criterion.rs producing its own reports when running with cargo-criterion
    // TODO: Notify burntsushi/critcmp about the internal file format change after I've added support for flat sampling

    let mut run_model = model::Model::load(self_config.criterion_home.clone(), "main".into());

    let cli_report = configure_cli_output(self_config);
    let bencher_report = crate::report::BencherReport;
    let html_report = crate::html::Html::new(get_plotter(self_config)?);

    let mut reports: Vec<&dyn crate::report::Report> = Vec::new();
    match self_config.output_format {
        OutputFormat::Bencher => reports.push(&bencher_report),
        OutputFormat::Criterion | OutputFormat::Quiet | OutputFormat::Verbose => {
            reports.push(&cli_report)
        }
    }
    reports.push(&html_report);
    let reports = crate::report::Reports::new(reports);

    if self_config.do_run {
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
    }

    let final_context = ReportContext {
        output_directory: self_config.criterion_home.join("reports"),
        plot_config: PlotConfiguration {
            summary_scale: AxisScale::Linear,
        },
    };

    reports.final_summary(&final_context, &run_model);
    Ok(())
}

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

#[cfg(feature = "gnuplot_backend")]
fn gnuplot_plotter() -> Result<Box<dyn Plotter>, Error> {
    match criterion_plot::version() {
        Ok(_) => Ok(Box::new(crate::plot::Gnuplot::new())),
        Err(_) => Err(anyhow::anyhow!("Gnuplot is not available. To continue, either install Gnuplot or allow cargo-criterion to fall back to using plotters.")),
    }
}

#[cfg(not(feature = "gnuplot_backend"))]
fn gnuplot_plotter() -> Result<Box<dyn Plotter>, Error> {
    anyhow::bail!("Gnuplot backend is disabled. To use gnuplot backend, install cargo-criterion with the 'gnuplot_backend' feature enabled")
}

#[cfg(feature = "plotters_backend")]
fn plotters_plotter() -> Result<Box<dyn Plotter>, Error> {
    Ok(Box::new(crate::plot::PlottersBackend))
}

#[cfg(not(feature = "plotters_backend"))]
fn plotters_plotter() -> Result<Box<dyn Plotter>, Error> {
    anyhow::bail!("Plotters backend is disabled. To use plotters backend, install cargo-criterion with the 'plotters_backend' feature enabled")
}

#[cfg(any(feature = "gnuplot_backend", feature = "plotters_backend"))]
fn get_plotter(config: &SelfConfig) -> Result<Box<dyn Plotter>, Error> {
    match config.plotting_backend {
        PlottingBackend::Gnuplot => gnuplot_plotter(),
        PlottingBackend::Plotters => plotters_plotter(),
        PlottingBackend::Auto => gnuplot_plotter().or(plotters_plotter()),
    }
}

#[cfg(not(any(feature = "gnuplot_backend", feature = "plotters_backend")))]
fn get_plotter(_: &SelfConfig) -> Result<Box<dyn Plotter>, Error> {
    anyhow::bail!("No plotting backend is available. At least one of the 'gnuplot_backend' or 'plotters_backend' features must be included.")
}

trait DurationExt {
    fn to_nanos(&self) -> u64;
}

const NANOS_PER_SEC: u64 = 1_000_000_000;

impl DurationExt for std::time::Duration {
    fn to_nanos(&self) -> u64 {
        self.as_secs() * NANOS_PER_SEC + u64::from(self.subsec_nanos())
    }
}
