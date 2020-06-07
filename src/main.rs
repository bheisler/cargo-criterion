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
mod report;
mod stats;
mod value_formatter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let configuration = config::configure()?;
    let self_config = &configuration.self_config;

    let bench_targets = compile::compile(&configuration.cargo_args)?;

    let report = crate::report::CliReport::new(true, true, false);

    if self_config.do_run {
        for bench in bench_targets {
            println!("Executing {} - {:?}", bench.name, bench.executable);
            let err = bench.execute(
                &self_config.criterion_home,
                &configuration.additional_args,
                &report,
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
