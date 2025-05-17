use crate::connection::Throughput;
use crate::estimate::{ChangeEstimates, Estimates};
use crate::report::{BenchmarkId, ComparisonData, MeasurementData};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use linked_hash_map::LinkedHashMap;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs::File;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug)]
pub struct Benchmark {
    pub latest_stats: SavedStatistics,
    pub previous_stats: Option<SavedStatistics>,
    pub target: Option<String>,
}
impl Benchmark {
    fn new(stats: SavedStatistics) -> Self {
        Benchmark {
            latest_stats: stats,
            previous_stats: None,
            target: None,
        }
    }

    fn add_stats(&mut self, stats: SavedStatistics) {
        let previous_stats = std::mem::replace(&mut self.latest_stats, stats);
        self.previous_stats = Some(previous_stats);
    }
}

#[derive(Debug)]
pub struct BenchmarkGroup {
    pub benchmarks: LinkedHashMap<BenchmarkId, Benchmark>,
    pub target: Option<String>,
}
impl Default for BenchmarkGroup {
    fn default() -> Self {
        BenchmarkGroup {
            benchmarks: LinkedHashMap::new(),
            target: None,
        }
    }
}

/// The Model struct stores everything that we keep in-memory about the benchmarks and their
/// performance. It's loaded from disk at the beginning of a run and updated as benchmarks
/// are executed.
#[derive(Debug)]
pub struct Model {
    // Path to output directory
    data_directory: PathBuf,
    // Track all of the unique benchmark titles and directories we've seen, so we can uniquify them.
    all_titles: HashSet<String>,
    all_directories: HashSet<PathBuf>,
    // All of the known benchmark groups, stored in execution order (where possible).
    pub groups: LinkedHashMap<String, BenchmarkGroup>,

    history_id: Option<String>,
    history_description: Option<String>,
}
impl Model {
    /// Load the model from disk. The output directory is scanned for benchmark files. Any files
    /// found are loaded into the model so that we can include them in the reports even if this
    /// run doesn't execute that particular benchmark.
    pub fn load(
        criterion_home: PathBuf,
        timeline: PathBuf,
        history_id: Option<String>,
        history_description: Option<String>,
    ) -> Model {
        let mut model = Model {
            data_directory: path!(criterion_home, "data", timeline),
            all_titles: HashSet::new(),
            all_directories: HashSet::new(),
            groups: LinkedHashMap::new(),
            history_id,
            history_description,
        };

        for entry in WalkDir::new(&model.data_directory)
            .into_iter()
            // Ignore errors.
            .filter_map(::std::result::Result::ok)
            .filter(|entry| entry.file_name() == OsStr::new("benchmark.cbor"))
        {
            if let Err(e) = model.load_stored_benchmark(entry.path()) {
                error!("Encountered error while loading stored data: {}", e)
            }
        }

        model
    }

    fn load_stored_benchmark(&mut self, benchmark_path: &Path) -> Result<()> {
        if !benchmark_path.is_file() {
            return Ok(());
        }
        let mut benchmark_file = File::open(benchmark_path)
            .with_context(|| format!("Failed to open benchmark file {:?}", benchmark_path))?;
        let benchmark_record: BenchmarkRecord = serde_cbor::from_reader(&mut benchmark_file)
            .with_context(|| format!("Failed to read benchmark file {:?}", benchmark_path))?;

        let measurement_path = benchmark_path.with_file_name(benchmark_record.latest_record);
        if !measurement_path.is_file() {
            return Ok(());
        }
        let mut measurement_file = File::open(&measurement_path)
            .with_context(|| format!("Failed to open measurement file {:?}", measurement_path))?;
        let saved_stats: SavedStatistics = serde_cbor::from_reader(&mut measurement_file)
            .with_context(|| format!("Failed to read measurement file {:?}", measurement_path))?;

        self.groups
            .entry(benchmark_record.id.group_id.clone())
            .or_insert_with(Default::default)
            .benchmarks
            .insert(benchmark_record.id.into(), Benchmark::new(saved_stats));
        Ok(())
    }

    pub fn add_benchmark_id(&mut self, target: &str, id: &mut BenchmarkId) {
        id.ensure_directory_name_unique(&self.all_directories);
        self.all_directories
            .insert(id.as_directory_name().to_owned());

        id.ensure_title_unique(&self.all_titles);
        self.all_titles.insert(id.as_title().to_owned());

        let group = self
            .groups
            .entry(id.group_id.clone())
            .or_insert_with(Default::default);

        if let Some(mut benchmark) = group.benchmarks.remove(id) {
            if let Some(target) = &benchmark.target {
                warn!("Benchmark ID {} encountered multiple times. Benchmark IDs must be unique. First seen in the benchmark target '{}'", id.as_title(), target);
            } else {
                benchmark.target = Some(target.to_owned());
            }

            // Remove and re-insert to move the benchmark to the end of its list.
            group.benchmarks.insert(id.clone(), benchmark);
        }
    }

    pub fn benchmark_complete(
        &mut self,
        id: &BenchmarkId,
        analysis_results: &MeasurementData,
    ) -> Result<()> {
        let dir = path!(&self.data_directory, id.as_directory_name());

        std::fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create directory {:?}", dir))?;

        let measurement_name = chrono::Local::now()
            .format("measurement_%y%m%d%H%M%S.cbor")
            .to_string();

        let saved_stats = SavedStatistics {
            datetime: chrono::Utc::now(),
            iterations: analysis_results.iter_counts().to_vec(),
            values: analysis_results.sample_times().to_vec(),
            avg_values: analysis_results.avg_times.to_vec(),
            estimates: analysis_results.absolute_estimates.clone(),
            throughput: analysis_results.throughput.clone(),
            changes: analysis_results
                .comparison
                .as_ref()
                .map(|c| c.relative_estimates.clone()),
            change_direction: analysis_results
                .comparison
                .as_ref()
                .map(get_change_direction),
            history_id: self.history_id.clone(),
            history_description: self.history_description.clone(),
        };

        let measurement_path = dir.join(&measurement_name);
        let mut measurement_file = File::create(&measurement_path)
            .with_context(|| format!("Failed to create measurement file {:?}", measurement_path))?;
        serde_cbor::to_writer(&mut measurement_file, &saved_stats).with_context(|| {
            format!("Failed to save measurements to file {:?}", measurement_path)
        })?;

        let record = BenchmarkRecord {
            id: id.into(),
            latest_record: PathBuf::from(&measurement_name),
        };

        let benchmark_path = dir.join("benchmark.cbor");
        let mut benchmark_file = File::create(&benchmark_path)
            .with_context(|| format!("Failed to create benchmark file {:?}", benchmark_path))?;
        serde_cbor::to_writer(&mut benchmark_file, &record)
            .with_context(|| format!("Failed to save benchmark file {:?}", benchmark_path))?;

        let benchmark_entry = self
            .groups
            .get_mut(&id.group_id)
            .unwrap()
            .benchmarks
            .entry(id.clone());

        match benchmark_entry {
            vacant @ linked_hash_map::Entry::Vacant(_) => {
                vacant.or_insert(Benchmark::new(saved_stats));
            }
            linked_hash_map::Entry::Occupied(mut occupied) => {
                occupied.get_mut().add_stats(saved_stats)
            }
        };
        Ok(())
    }

    pub fn get_last_sample(&self, id: &BenchmarkId) -> Option<&SavedStatistics> {
        self.groups
            .get(&id.group_id)
            .and_then(|g| g.benchmarks.get(id))
            .map(|b| &b.latest_stats)
    }

    pub fn check_benchmark_group(&self, current_target: &str, group: &str) {
        if let Some(benchmark_group) = self.groups.get(group) {
            if let Some(target) = &benchmark_group.target {
                if target != current_target {
                    warn!("Benchmark group {} encountered again. Benchmark group IDs must be unique. First seen in the benchmark target '{}'", group, target);
                }
            }
        }
    }

    pub fn add_benchmark_group(&mut self, target: &str, group_name: &str) -> &BenchmarkGroup {
        // Remove and reinsert so that the group will be at the end of the map.
        let mut group = self.groups.remove(group_name).unwrap_or_default();
        group.target = Some(target.to_owned());
        self.groups.insert(group_name.to_owned(), group);
        self.groups.get(group_name).unwrap()
    }

    pub fn load_history(&self, id: &BenchmarkId) -> Result<Vec<SavedStatistics>> {
        let dir = path!(&self.data_directory, id.as_directory_name());

        fn load_from(measurement_path: &Path) -> Result<SavedStatistics> {
            let mut measurement_file = File::open(measurement_path).with_context(|| {
                format!("Failed to open measurement file {:?}", measurement_path)
            })?;
            serde_cbor::from_reader(&mut measurement_file)
                .with_context(|| format!("Failed to read measurement file {:?}", measurement_path))
        }

        let mut stats = Vec::new();
        for entry in WalkDir::new(dir)
            .max_depth(1)
            .into_iter()
            // Ignore errors.
            .filter_map(::std::result::Result::ok)
        {
            let name_str = entry.file_name().to_string_lossy();
            if name_str.starts_with("measurement_") && name_str.ends_with(".cbor") {
                match load_from(entry.path()) {
                    Ok(saved_stats) => stats.push(saved_stats),
                    Err(e) => error!(
                        "Unexpected error loading benchmark history from file {}: {:?}",
                        entry.path().display(),
                        e
                    ),
                }
            }
        }

        stats.sort_unstable_by_key(|st| st.datetime);

        Ok(stats)
    }
}

// These structs are saved to disk and may be read by future versions of cargo-criterion, so
// backwards compatibility is important.

#[derive(Debug, Deserialize, Serialize)]
pub struct SavedBenchmarkId {
    group_id: String,
    function_id: Option<String>,
    value_str: Option<String>,
    throughput: Option<Throughput>,
}
impl From<BenchmarkId> for SavedBenchmarkId {
    fn from(other: BenchmarkId) -> Self {
        SavedBenchmarkId {
            group_id: other.group_id,
            function_id: other.function_id,
            value_str: other.value_str,
            throughput: other.throughput,
        }
    }
}
impl From<&BenchmarkId> for SavedBenchmarkId {
    fn from(other: &BenchmarkId) -> Self {
        other.clone().into()
    }
}
impl From<SavedBenchmarkId> for BenchmarkId {
    fn from(other: SavedBenchmarkId) -> Self {
        BenchmarkId::new(
            other.group_id,
            other.function_id,
            other.value_str,
            other.throughput,
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct BenchmarkRecord {
    id: SavedBenchmarkId,
    latest_record: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ChangeDirection {
    NoChange,
    NotSignificant,
    Improved,
    Regressed,
}

fn get_change_direction(comp: &ComparisonData) -> ChangeDirection {
    if comp.p_value < comp.significance_threshold {
        return ChangeDirection::NoChange;
    }

    let ci = &comp.relative_estimates.mean.confidence_interval;
    let lb = ci.lower_bound;
    let ub = ci.upper_bound;
    let noise = comp.noise_threshold;

    if lb < -noise && ub < -noise {
        ChangeDirection::Improved
    } else if lb > noise && ub > noise {
        ChangeDirection::Regressed
    } else {
        ChangeDirection::NotSignificant
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SavedStatistics {
    // The timestamp of when these measurements were saved.
    pub datetime: DateTime<Utc>,
    // The number of iterations in each sample
    pub iterations: Vec<f64>,
    // The measured values from each sample
    pub values: Vec<f64>,
    // The average values from each sample, ie. values / iterations
    pub avg_values: Vec<f64>,
    // The statistical estimates from this run
    pub estimates: Estimates,
    // The throughput of this run
    pub throughput: Option<Throughput>,
    // The statistical differences compared to the last run. We save these so we don't have to
    // recompute them later for the history report.
    pub changes: Option<ChangeEstimates>,
    // Was the change (if any) significant?
    pub change_direction: Option<ChangeDirection>,

    // An optional user-provided identifier string. This might be a version control commit ID or
    // something custom
    pub history_id: Option<String>,
    // An optional user-provided description. This might be a version control commit message or
    // something custom.
    pub history_description: Option<String>,
}
