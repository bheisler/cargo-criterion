#[macro_use]
extern crate serde_derive;

mod bench_target;
mod compile;
mod config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let configuration = config::configure()?;
    let self_config = &configuration.self_config;

    let bench_targets = compile::compile(&configuration.cargo_args)?;

    if self_config.do_run {
        for bench in bench_targets {
            println!("Executing {} - {:?}", bench.name, bench.executable);
            let err = bench.execute(&self_config.criterion_home, &configuration.additional_args);

            if err.is_err() {
                let err = err.unwrap_err();
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
