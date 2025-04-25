use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use regex::RegexSet;
use log::{info, warn};
use jwalk::WalkDir;
use rayon::prelude::*;

// 讀取檔案內容，保持串流讀寫
pub fn read_file_content(file_path: &Path) -> io::Result<(Vec<u8>, usize)> {
    let mut buffer = Vec::new();
    let file_size = crate::utils::copy_file_content(file_path, &mut buffer)?;
    Ok((buffer, file_size))
}

// 檢查檔案是否有效，批次處理正則表達式
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
            warn!("檔案 {} 超過大小限制（{} MB > {} MB)，跳過", path.display(), file_size, max);
            return Ok(false);
        }
    }
    Ok(true)
}

// 過濾目錄，記錄跳過的目錄數
fn filter_entry(
    entry: &jwalk::DirEntry<((), ())>,
    exclude_set: &RegexSet,
    skipped_dirs: &mut u64,
) -> bool {
    let path_str = entry.path().to_string_lossy().into_owned();
    if exclude_set.is_match(&path_str) {
        if entry.file_type().is_dir() {
            *skipped_dirs += 1;
        }
        false
    } else {
        true
    }
}

// 檔案蒐集器結構體，移除 pm 字段
pub struct FileCollector {
    include_set: RegexSet,
    exclude_set: RegexSet,
    max_size: Option<f64>,
    no_progress: bool,
}

impl FileCollector {
    pub fn new(
        include_set: RegexSet,
        exclude_set: RegexSet,
        max_size: Option<f64>,
        no_progress: bool,
    ) -> Self {
        FileCollector {
            include_set,
            exclude_set,
            max_size,
            no_progress,
        }
    }

    pub fn collect_files(&self, input_path: &Path) -> io::Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        let pm = crate::utils::create_progress_bar(0, self.no_progress);
        self.collect_and_measure_files(input_path, &mut files, false, &pm)?;
        Ok(files)
    }

    pub fn collect_and_measure_files(
        &self,
        input_path: &Path,
        files: &mut Vec<PathBuf>,
        measure_size: bool,
        pm: &crate::utils::ProgressManager,
    ) -> io::Result<usize> {
        let mut total_size = 0;
        let mut skipped_dirs = 0;
        let start = std::time::Instant::now();

        // 使用 jwalk 進行平行遍歷
        let entries: Vec<_> = WalkDir::new(input_path)
            .skip_hidden(false)
            .parallelism(jwalk::Parallelism::RayonNewPool(4))
            .into_iter()
            .filter(|e| e.as_ref().map_or(true, |e| filter_entry(e, &self.exclude_set, &mut skipped_dirs)))
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .collect();

        // 批次檢查檔案有效性
        let batch_size = 1000;
        for chunk in entries.chunks(batch_size) {
            let batch_results: Vec<_> = chunk
                .par_iter()
                .filter_map(|entry| {
                    let path = entry.path();
                    match is_file_valid(&path, &self.include_set, &self.exclude_set, self.max_size) {
                        Ok(true) => {
                            let size = if measure_size {
                                fs::metadata(&path).map(|m| m.len() as usize).unwrap_or(0)
                            } else {
                                0
                            };
                            Some((path.to_path_buf(), size))
                        }
                        Ok(false) => None,
                        Err(e) => {
                            warn!("檢查檔案 {} 失敗: {}", path.display(), e);
                            None
                        }
                    }
                })
                .collect();

            for (path, size) in batch_results {
                files.push(path);
                total_size += size;
                if !self.no_progress && files.len() % 1000 == 0 {
                    pm.update(
                        files.len() as u64,
                        if measure_size { Some(total_size) } else { None },
                        "蒐集檔案",
                    );
                }
            }
        }

        if !self.no_progress && files.len() % 1000 != 0 {
            pm.update(
                files.len() as u64,
                if measure_size { Some(total_size) } else { None },
                "蒐集檔案",
            );
        }

        if files.is_empty() {
            pm.finish(0, None, skipped_dirs);
            return Err(io::Error::new(io::ErrorKind::Other, "無有效檔案可壓縮"));
        }

        pm.finish(files.len() as u64, if measure_size { Some(total_size) } else { None }, skipped_dirs);
        info!(
            "蒐集檔案完成，共 {} 個檔案，總大小：{} 位元組，跳過 {} 個目錄",
            files.len(),
            total_size,
            skipped_dirs
        );
        Ok(total_size)
    }
}

// 更新 collect_files
pub fn collect_files(
    path: &Path,
    files: &mut Vec<PathBuf>,
    include_set: &RegexSet,
    exclude_set: &RegexSet,
    max_size: Option<f64>,
    no_progress: bool,
) -> io::Result<()> {
    let collector = FileCollector::new(
        include_set.clone(),
        exclude_set.clone(),
        max_size,
        no_progress,
    );
    let pm = crate::utils::create_progress_bar(0, no_progress);
    collector.collect_and_measure_files(path, files, false, &pm)?;
    Ok(())
}

// 更新 collect_and_measure_files
pub fn collect_and_measure_files(
    input_path: &Path,
    include_set: &RegexSet,
    exclude_set: &RegexSet,
    max_size: Option<f64>,
    no_progress: bool,
) -> io::Result<(Vec<PathBuf>, usize)> {
    let collector = FileCollector::new(
        include_set.clone(),
        exclude_set.clone(),
        max_size,
        no_progress,
    );
    let pm = crate::utils::create_progress_bar(0, no_progress);
    let mut files = Vec::new();
    let total_size = collector.collect_and_measure_files(input_path, &mut files, true, &pm)?;
    Ok((files, total_size))
}