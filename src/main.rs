#[macro_use]
extern crate serde_derive;

mod compile;
mod config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let configuration = config::configure()?;
    println!("{:?}", configuration);

    let benchmarks = compile::compile(&configuration.cargo_args)?;

    println!("Found {} benchmarks", benchmarks.len());
    for bench in benchmarks.iter() {
        println!("Benchmark: {:?}", bench);
    }

    Ok(())
}
