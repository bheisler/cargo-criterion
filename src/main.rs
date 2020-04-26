#[macro_use]
extern crate serde_derive;

mod args;
mod compile;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dummy_args = args::CargoArguments {};

    let benchmarks = compile::compile(&dummy_args)?;

    println!("Found {} benchmarks", benchmarks.len());
    for bench in benchmarks.iter() {
        println!("Benchmark: {:?}", bench);
    }

    Ok(())
}
