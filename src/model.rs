use crate::connection::Throughput;
use crate::estimate::Estimates;
use crate::report::{BenchmarkId, MeasurementData};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::PathBuf;

pub struct Model {
    // Path to output directory
    criterion_home: PathBuf,
    // Name of the timeline we're writing to.
    timeline: PathBuf,
    // Maps benchmark IDs to their targets so we can give a better warning.
    all_benchmark_ids: HashMap<BenchmarkId, String>,
    // Also track benchmark group IDs, since those also need to be unique.
    benchmark_groups: HashMap<String, String>,
    // Track all of the unique benchmark titles and directories we've seen, so we can uniquify them.
    all_titles: HashSet<String>,
    all_directories: HashSet<PathBuf>,
}
impl Model {
    pub fn new(criterion_home: PathBuf, timeline: PathBuf) -> Model {
        Model {
            criterion_home,
            timeline,
            all_benchmark_ids: HashMap::new(),
            all_titles: HashSet::new(),
            all_directories: HashSet::new(),
            benchmark_groups: HashMap::new(),
        }
    }

    pub fn add_benchmark_id(&mut self, target: &str, id: &mut BenchmarkId) {
        id.ensure_directory_name_unique(&self.all_directories);
        self.all_directories
            .insert(id.as_directory_name().to_owned());

        id.ensure_title_unique(&self.all_titles);
        self.all_titles.insert(id.as_title().to_owned());

        if let Some(target) = self.all_benchmark_ids.get(id) {
            warn!("Benchmark ID {} encountered multiple times. Benchmark IDs must be unique. First seen in the benchmark target '{}'", id.as_title(), target);
        } else {
            self.all_benchmark_ids.insert(id.clone(), target.to_owned());
        }
    }

    pub fn benchmark_complete(
        &self,
        id: &BenchmarkId,
        analysis_results: &MeasurementData,
    ) -> Result<()> {
        let dir = path!(
            &self.criterion_home,
            "data",
            &self.timeline,
            id.as_directory_name()
        );

        std::fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create directory {:?}", dir));

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
        };

        let measurement_path = dir.join(&measurement_name);
        let mut measurement_file = File::create(&measurement_path)
            .with_context(|| format!("Failed to create measurement file {:?}", measurement_path))?;
        serde_cbor::to_writer(&mut measurement_file, &saved_stats).with_context(|| {
            format!("Failed to save measurements to file {:?}", measurement_path)
        })?;

        let record = BenchmarkRecord {
            id: id.clone(),
            latest_record: PathBuf::from(&measurement_name),
        };

        let benchmark_path = dir.join("benchmark.cbor");
        let mut benchmark_file = File::create(&benchmark_path)
            .with_context(|| format!("Failed to create benchmark file {:?}", benchmark_path))?;
        serde_cbor::to_writer(&mut benchmark_file, &record)
            .with_context(|| format!("Failed to save benchmark file {:?}", benchmark_path))?;
        Ok(())
    }

    pub fn load_last_sample(&self, id: &BenchmarkId) -> Result<Option<SavedStatistics>> {
        let dir = path!(
            &self.criterion_home,
            "data",
            &self.timeline,
            id.as_directory_name()
        );

        let benchmark_path = dir.join("benchmark.cbor");
        if !benchmark_path.is_file() {
            return Ok(None);
        }
        let mut benchmark_file = File::open(&benchmark_path)
            .with_context(|| format!("Failed to open benchmark file {:?}", benchmark_path))?;
        let benchmark_record: BenchmarkRecord = serde_cbor::from_reader(&mut benchmark_file)
            .with_context(|| format!("Failed to read benchmark file {:?}", benchmark_path))?;

        let measurement_path = dir.join(&benchmark_record.latest_record);
        if !measurement_path.is_file() {
            return Ok(None);
        }
        let mut measurement_file = File::open(&measurement_path)
            .with_context(|| format!("Failed to open measurement file {:?}", measurement_path))?;
        let saved_stats: SavedStatistics = serde_cbor::from_reader(&mut measurement_file)
            .with_context(|| format!("Failed to read benchmark file {:?}", measurement_path))?;
        Ok(Some(saved_stats))
    }

    pub fn check_benchmark_group(&self, group: &str) {
        if let Some(target) = self.benchmark_groups.get(group) {
            warn!("Benchmark group {} encountered again. Benchmark group IDs must be unique. First seen in the benchmark target '{}'", group, target);
        }
    }

    pub fn add_benchmark_group(&mut self, target: &str, group: String) {
        self.benchmark_groups
            .entry(group)
            .or_insert(target.to_owned());
    }
}

// These structs are saved to disk and may be read by future versions of cargo-criterion, so
// backwards compatibility is important.

#[derive(Debug, Serialize, Deserialize)]
struct BenchmarkRecord {
    id: BenchmarkId,
    latest_record: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SavedStatistics {
    pub datetime: DateTime<Utc>,
    pub iterations: Vec<f64>,
    pub values: Vec<f64>,
    pub avg_values: Vec<f64>,
    pub estimates: Estimates,
    pub throughput: Option<Throughput>,
}
