use std::path::PathBuf;

#[derive(Clone)]
pub struct HtmlGenerateInput {
    pub zip_buffer: Vec<u8>,
    pub input_path: PathBuf,
    pub output_dir: String,
    pub layer: String,
    pub password: Option<String>,
    pub display_password: bool,
    pub total_size: usize,
}

#[derive(Debug)]
pub struct HtmlGenerateOutput {
    pub html_file_path: String,
}