use crate::config::config::{PasswordMode};
use std::path::PathBuf;

#[derive(Clone)]
pub struct ConversionInput {
    pub input_path: PathBuf,
    pub output_dir: String,
    pub is_compressed: bool,
    pub compress: bool,
    pub include: Vec<String>,
    pub exclude: Option<Vec<String>>,
    pub password_mode: PasswordMode,
    pub display_password: bool,
    pub layer: String,
    pub encryption_method: String,
    pub no_progress: bool,
    pub max_size: Option<f64>,
}

#[derive(Debug)]
pub struct ConversionOutput {
    pub output_path: String,
    pub processed_files: usize,
}