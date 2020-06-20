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
mod model;
mod report;
mod stats;
mod value_formatter;

use crate::config::{OutputFormat, TextColor};
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
    // TODO: Implement charting
    // TODO: Document the code
    // TODO: Add a section to the user guide

    let mut run_model = model::Model::new(self_config.criterion_home.clone(), "main".into());

    let cli_report = configure_cli_output(self_config);
    let bencher_report = crate::report::BencherReport;

    let report: &dyn crate::report::Report = match self_config.output_format {
        OutputFormat::Bencher => &bencher_report,
        OutputFormat::Criterion | OutputFormat::Quiet | OutputFormat::Verbose => &cli_report,
    };

    if self_config.do_run {
        for bench in bench_targets {
            info!("Executing {} - {:?}", bench.name, bench.executable);
            let err = bench.execute(
                &self_config.criterion_home,
                &configuration.additional_args,
                report,
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

trait DurationExt {
    fn to_nanos(&self) -> u64;
}

const NANOS_PER_SEC: u64 = 1_000_000_000;

impl DurationExt for std::time::Duration {
    fn to_nanos(&self) -> u64 {
        self.as_secs() * NANOS_PER_SEC + u64::from(self.subsec_nanos())
    }
}
