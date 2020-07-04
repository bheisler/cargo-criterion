use bstr::ByteSlice;
use std::collections::HashSet;
use std::io::stdout;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::tempdir;
use walkdir::WalkDir;

// TODO: Could assert that the data files can be read?
// TODO: Could assert that the SVG files are valid?
// TODO: Could assert that the HTML files are valid and contain no broken links?

fn execute(args: &[&str], homedir: &Path) -> (Output, Output) {
    // TODO: This only works on unix-likes.
    let cargo_criterion_path = Path::new("../target/debug/cargo-criterion");
    assert!(cargo_criterion_path.exists());

    println!("Running cargo-criterion...");
    let first_output = Command::new(cargo_criterion_path)
        .arg("--debug") // Build benchmarks in test mode to avoid expensive optimized compile.
        .args(args)
        .env("CRITERION_HOME", homedir)
        .output()
        .expect("Failed to run cargo-criterion");
    if !first_output.status.success() {
        println!("Failed to run cargo-cargo criterion.");
        println!("stdout:");
        stdout().write_all(&first_output.stdout).unwrap();
        println!("stderr:");
        stdout().write_all(&first_output.stderr).unwrap();

        panic!("Failed to run cargo-criterion.")
    }

    // Run twice to get a history
    println!("Running cargo-criterion a second time...");
    let second_output = Command::new(cargo_criterion_path)
        .arg("--debug") // Build benchmarks in test mode to avoid expensive optimized compile.
        .args(args)
        .env("CRITERION_HOME", homedir)
        .output()
        .expect("Failed to run cargo-criterion");
    if !second_output.status.success() {
        println!("Failed to run cargo-cargo criterion.");
        println!("stdout:");
        stdout().write_all(&second_output.stdout).unwrap();
        println!("stderr:");
        stdout().write_all(&second_output.stderr).unwrap();

        panic!("Failed to run cargo-criterion.")
    }

    (first_output, second_output)
}

fn benchmark_names() -> &'static [&'static str] {
    &[
        "norm",
        "bencher_test",
        "\"*group/\"/\"*benchmark/\" '",
        "sampling_mode/Auto (short)",
        "sampling_mode/Auto (long)",
        "sampling_mode/Linear",
        "sampling_mode/Flat",
        "throughput/Bytes",
        "throughput/Elem",
    ]
}

fn file_safe_benchmark_names() -> &'static [&'static str] {
    &[
        "norm",
        //"bencher_test", // This one isn't a criterion.rs test, so we don't include it in the output files.
        "__group__/__benchmark__ '",
        "sampling_mode/Auto (short)",
        "sampling_mode/Auto (long)",
        "sampling_mode/Linear",
        "sampling_mode/Flat",
        "throughput/Bytes",
        "throughput/Elem",
    ]
}

fn file_safe_benchmark_group_names() -> &'static [&'static str] {
    &["norm", "__group__", "sampling_mode", "throughput"]
}

// Note that we run cargo-criterion just twice for each benchmark and make many assertions about
// the results. This isn't normally good testing practice, but cargo-criterion is expensive to
// execute. AssertionState tracks whether we've actually succeeded.
struct AssertionState {
    success: bool,
    expected_paths: HashSet<PathBuf>,
}
impl Default for AssertionState {
    fn default() -> Self {
        AssertionState {
            success: true,
            expected_paths: HashSet::new(),
        }
    }
}
impl AssertionState {
    fn assert_benchmarks_present(&mut self, first_second: &str, output: &[u8]) {
        for benchmark in benchmark_names() {
            if !output.contains_str(benchmark) {
                self.success = false;
                println!(
                    "Expected benchmark {} to appear in {} output, but it was not found.",
                    benchmark, first_second,
                );
            }
        }
    }

    fn does_exist<P: AsRef<Path>>(&mut self, expected_path: P) -> bool {
        let expected_path = expected_path.as_ref();
        self.expected_paths.insert(expected_path.to_owned());
        expected_path.exists()
    }

    fn assert_file_exists<P: AsRef<Path>>(&mut self, base_path: &Path, expected_path: P) {
        let expected_path = expected_path.as_ref();
        if !self.does_exist(base_path.join(expected_path)) {
            println!("Expected to find file {:?}", expected_path);
            self.success = false;
        }
    }

    fn assert_data_files_present(&mut self, homedir: &Path) {
        let main_dir = homedir.join("data/main");
        if !self.does_exist(&main_dir) {
            println!("Found no data from benchmark.");
            self.success = false;
            return;
        }

        for benchmark_dir_name in file_safe_benchmark_names() {
            let bench_dir = main_dir.join(benchmark_dir_name);
            if !self.does_exist(&bench_dir) {
                println!("Expected to find directory {:?}", bench_dir);
                self.success = false;
                continue;
            }

            let benchmark_record_file = bench_dir.join("benchmark.cbor");
            if !self.does_exist(&benchmark_record_file) {
                println!(
                    "Expected to find benchmark record file {:?}",
                    benchmark_record_file
                );
                self.success = false;
            }

            let read_dir = bench_dir
                .read_dir()
                .expect(&format!("Unable to read output dir {:?}", &bench_dir));
            let measurement_files = read_dir
                .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
                .filter(|file_name| {
                    file_name.starts_with("measurement_") && file_name.ends_with(".cbor")
                })
                .map(|name| bench_dir.join(name))
                .collect::<Vec<_>>();

            if measurement_files.len() != 2 {
                println!(
                    "Expected to find two benchmark measurement files in {:?}, but found {:?}",
                    bench_dir, measurement_files
                );
                self.success = false;
            }
            self.expected_paths.extend(measurement_files);
        }
    }

    fn assert_individual_benchmark_report_files(&mut self, homedir: &Path) {
        let reports_dir = homedir.join("reports");
        if !self.does_exist(&reports_dir) {
            println!("Found no reports from benchmark.");
            self.success = false;
            return;
        }

        for benchmark_dir_name in file_safe_benchmark_names() {
            let report_dir = reports_dir.join(benchmark_dir_name);
            if !self.does_exist(&report_dir) {
                println!("Expected to find directory {:?}", report_dir);
                self.success = false;
                continue;
            }

            if !self.does_exist(&report_dir.join("regression.svg"))
                && !self.does_exist(&report_dir.join("iteration_times.svg"))
            {
                println!("Expected to find either regression.svg (for lineary benchmarks) or iteration_times.svg (for flat benchmarks) but found neither.");
                self.success = false;
            }

            self.assert_file_exists(&report_dir, "MAD.svg");
            self.assert_file_exists(&report_dir, "both/pdf.svg");
            self.assert_file_exists(&report_dir, "change/mean.svg");
            self.assert_file_exists(&report_dir, "change/median.svg");
            self.assert_file_exists(&report_dir, "change/t-test.svg");
            self.assert_file_exists(&report_dir, "index.html");
            self.assert_file_exists(&report_dir, "mean.svg");
            self.assert_file_exists(&report_dir, "median.svg");
            self.assert_file_exists(&report_dir, "pdf.svg");
            self.assert_file_exists(&report_dir, "pdf_small.svg");
            self.assert_file_exists(&report_dir, "relative_pdf_small.svg");
            self.assert_file_exists(&report_dir, "typical.svg");

            if self.does_exist(&report_dir.join("regression.svg")) {
                self.assert_file_exists(&report_dir, "both/regression.svg");
                self.assert_file_exists(&report_dir, "relative_regression_small.svg");
                self.assert_file_exists(&report_dir, "regression_small.svg");
                self.assert_file_exists(&report_dir, "slope.svg");
            }
            if self.does_exist(&report_dir.join("iteration_times.svg")) {
                self.assert_file_exists(&report_dir, "both/iteration_times.svg");
                self.assert_file_exists(&report_dir, "relative_iteration_times_small.svg");
                self.assert_file_exists(&report_dir, "iteration_times_small.svg");
            }
        }
    }

    fn assert_group_summary_files(&mut self, homedir: &Path) {
        let reports_dir = homedir.join("reports");
        if !self.does_exist(&reports_dir) {
            println!("Found no reports from benchmark.");
            self.success = false;
            return;
        }

        for group_dir_name in file_safe_benchmark_group_names() {
            let summary_dir = reports_dir.join(group_dir_name);
            if !self.does_exist(&summary_dir) {
                println!("Expected to find directory {:?}", summary_dir);
                self.success = false;
                continue;
            }

            self.assert_file_exists(&summary_dir, "index.html");
            self.assert_file_exists(&summary_dir, "violin.svg");
        }
    }

    fn assert_overall_summary_report(&mut self, homedir: &Path) {
        self.assert_file_exists(homedir, "reports/index.html");
    }

    fn assert_no_unknown_files(&mut self, homedir: &Path) {
        for entry in WalkDir::new(homedir).into_iter().filter_map(|e| e.ok()) {
            let entry_path = entry.into_path();

            if entry_path.is_file() && !self.expected_paths.contains(&entry_path) {
                println!("Unexpected file {:?}", entry_path);
                self.success = false;
            }
        }
    }

    fn assert_success(&self) {
        assert!(self.success, "Test failed");
    }

    fn assert_benchmarks_in_json_messages(&mut self, output: &[u8]) {
        let stream = serde_json::Deserializer::from_slice(output).into_iter::<serde_json::Value>();

        let mut benchmark_ids_seen = HashSet::new();
        for value in stream {
            let value = value.unwrap();
            let reason = value["reason"].as_str().unwrap();
            if reason == "benchmark-complete" {
                let benchmark_id = value["id"].as_str().unwrap();
                benchmark_ids_seen.insert(benchmark_id.to_owned());
            }
        }

        for benchmark in benchmark_names() {
            // Only Criterion.rs benchmarks are expected.
            if *benchmark == "bencher_test" {
                continue;
            }

            if !benchmark_ids_seen.contains(*benchmark) {
                self.success = false;
                println!(
                    "Expected to find benchmark-complete message for {}.",
                    benchmark
                );
            }
        }
    }
}

#[test]
#[ignore]
fn test_cargo_criterion_gnuplot() {
    let homedir = tempdir().unwrap();
    let (first_output, second_output) = execute(&["--plotting-backend=gnuplot"], homedir.path());

    let mut state = AssertionState::default();
    state.assert_benchmarks_present("first", &first_output.stderr);
    state.assert_benchmarks_present("second", &second_output.stderr);
    state.assert_data_files_present(homedir.path());
    state.assert_individual_benchmark_report_files(homedir.path());
    state.assert_group_summary_files(homedir.path());
    state.assert_overall_summary_report(homedir.path());
    state.assert_no_unknown_files(homedir.path());
    state.assert_success();
}

#[test]
fn test_cargo_criterion_plotters() {
    let homedir = tempdir().unwrap();
    let (first_output, second_output) = execute(&["--plotting-backend=plotters"], homedir.path());

    let mut state = AssertionState::default();
    state.assert_benchmarks_present("first", &first_output.stderr);
    state.assert_benchmarks_present("second", &second_output.stderr);
    state.assert_data_files_present(homedir.path());
    state.assert_individual_benchmark_report_files(homedir.path());
    state.assert_group_summary_files(homedir.path());
    state.assert_overall_summary_report(homedir.path());
    state.assert_no_unknown_files(homedir.path());
    state.assert_success();
}

#[test]
fn test_json_message_format() {
    let homedir = tempdir().unwrap();
    let (first_output, second_output) = execute(&["--message-format=json"], homedir.path());

    let mut state = AssertionState::default();
    state.assert_benchmarks_present("first", &first_output.stderr);
    state.assert_benchmarks_present("second", &second_output.stderr);
    state.assert_benchmarks_in_json_messages(&second_output.stdout);
    state.assert_success();
}
