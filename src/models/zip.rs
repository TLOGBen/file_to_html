use std::path::PathBuf;
use zip::write::SimpleFileOptions;

#[derive(Clone)]
pub struct ZipCompressInput {
    pub files: Vec<PathBuf>,
    pub input_path: PathBuf,
    pub options: SimpleFileOptions,
    pub password: Option<String>,
    pub aes_mode: zip::AesMode,
    pub no_progress: bool,
}

#[derive(Debug)]
pub struct ZipCompressOutput {
    pub zip_buffer: Vec<u8>,
    pub total_size: usize,
}