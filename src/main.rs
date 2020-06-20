#[macro_use]
extern crate serde_derive;

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let configuration = config::configure()?;
    let self_config = &configuration.self_config;

    let bench_targets = compile::compile(&configuration.cargo_args)?;

    // TODO: Configure the CLI output properly
    // TODO: Set up a proper logger for cargo-criterion
    // TODO: Handle more errors without unwrapping, probably switch to a single central error type
    //   or just use anyhow
    // TODO: Make sure that test & profile mode still works
    // TODO: Handle filter requests properly
    // TODO: Add machine-readable output
    // TODO: Add alternate sampling modes (at least in the messaging)
    // TODO: Add support (at least in the messaging, so we can add it later) for multiple throughputs

    let mut run_model = model::Model::new(self_config.criterion_home.clone(), "main".into());

    let report = crate::report::CliReport::new(true, true, false);

    if self_config.do_run {
        for bench in bench_targets {
            println!("Executing {} - {:?}", bench.name, bench.executable);
            let err = bench.execute(
                &self_config.criterion_home,
                &configuration.additional_args,
                &report,
                &mut run_model,
            );

            if let Err(err) = err {
                if self_config.do_fail_fast {
                    return Err(Box::new(err));
                } else {
                    println!(
                        "Failed to execute benchmark target {}:\n{}",
                        bench.name, err
                    );
                }
            }
        }
    }

    Ok(())
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
