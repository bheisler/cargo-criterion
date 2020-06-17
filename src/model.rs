use crate::report::BenchmarkId;
use std::collections::{HashMap, HashSet};
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

    pub fn benchmark_complete(&self, id: &BenchmarkId) {
        let path = path!(
            &self.criterion_home,
            "data",
            &self.timeline,
            id.as_directory_name(),
            "benchmark.cbor"
        );

        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut file = std::fs::File::create(path).unwrap();
        serde_cbor::to_writer(&mut file, id).unwrap();
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
