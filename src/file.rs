use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use regex::RegexSet;
use log::warn;

pub fn read_file_content(file_path: &Path) -> io::Result<(Vec<u8>, usize)> {
    let mut file = File::open(file_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    let file_size = buffer.len();
    Ok((buffer, file_size))
}

pub fn is_file_valid(
    path: &Path,
    include_set: &RegexSet,
    exclude_set: &RegexSet,
    max_size: Option<f64>,
) -> io::Result<bool> {
    let path_str = path.to_string_lossy();
    if !include_set.is_match(&path_str) || exclude_set.is_match(&path_str) {
        return Ok(false);
    }
    if let Some(max) = max_size {
        let file_size = fs::metadata(path)?.len() as f64 / 1_048_576.0;
        if file_size > max {
            warn!("檔案 {} 超過大小限制（{} MB > {} MB），跳過", path.display(), file_size, max);
            return Ok(false);
        }
    }
    Ok(true)
}

pub fn collect_files(
    path: &Path,
    files: &mut Vec<PathBuf>,
    include_set: &RegexSet,
    exclude_set: &RegexSet,
    max_size: Option<f64>,
) -> io::Result<()> {
    if path.is_file() {
        if is_file_valid(path, include_set, exclude_set, max_size)? {
            files.push(path.to_path_buf());
        }
    } else if path.is_dir() {
        for entry in fs::read_dir(path)? {
            collect_files(&entry?.path(), files, include_set, exclude_set, max_size)?;
        }
    }
    Ok(())
}

pub fn collect_and_measure_files(
    input_path: &Path,
    include_set: &RegexSet,
    exclude_set: &RegexSet,
    max_size: Option<f64>,
) -> io::Result<(Vec<PathBuf>, usize)> {
    let mut files = Vec::new();
    collect_files(input_path, &mut files, include_set, exclude_set, max_size)?;
    if files.is_empty() {
        return Err(io::Error::new(io::ErrorKind::Other, "無有效檔案可壓縮"));
    }

    let mut total_size = 0;
    for file_path in &files {
        let (data, _) = read_file_content(file_path)?;
        total_size += data.len();
    }

    Ok((files, total_size))
}