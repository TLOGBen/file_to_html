use std::path::PathBuf;

#[derive(Clone)]
pub struct FileCollectInput {
    pub input_path: PathBuf,
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Option<Vec<String>>,
    pub max_size: Option<f64>,
    pub no_progress: bool,
}

#[derive(Debug)]
pub struct FileCollectOutput {
    pub files: Vec<PathBuf>,
    pub total_size: usize,
}