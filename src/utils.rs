use std::io::{self, Read, Write};
use regex::RegexSet;
use rand::{Rng};
use chrono::Local;
use indicatif::{ProgressBar, ProgressStyle};
use log;
use std::time::Instant;
use crate::config::PasswordMode;
use std::path::Path;
use std::fs::File;
use rand::distr::Alphanumeric;

pub fn setup_logging(log_level: &str) -> io::Result<()> {
    let log_level_filter = match log_level {
        "info" => log::LevelFilter::Info,
        "warn" => log::LevelFilter::Warn,
        "error" => log::LevelFilter::Error,
        _ => log::LevelFilter::Info,
    };
    env_logger::Builder::new()
        .filter_level(log_level_filter)
        .init();
    Ok(())
}

pub struct ProgressManager {
    pb: ProgressBar,
    no_progress: bool,
    start: Instant,
}

impl ProgressManager {
    pub fn new(total: u64, no_progress: bool) -> Self {
        let pb = if no_progress {
            ProgressBar::hidden()
        } else if total == 0 {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{msg} {spinner} 已處理: {pos} 檔案, 大小: {wide_msg}")
                    .unwrap(),
            );
            pb
        } else {
            let pb = ProgressBar::new(total);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{msg} [{bar:40}] {pos}/{len} ETA: {eta_precise}")
                    .unwrap()
                    .progress_chars("##-"),
            );
            pb
        };
        ProgressManager {
            pb,
            no_progress,
            start: Instant::now(),
        }
    }

    pub fn update(&self, count: u64, total_size: Option<usize>, action: &str) {
        if self.no_progress {
            return;
        }
        let elapsed = self.start.elapsed().as_secs_f64();
        let speed = if elapsed > 0.0 { count as f64 / elapsed } else { 0.0 };
        let msg = match total_size {
            Some(size) => format!(
                "{}：{} 檔案，{:.2} MB，速度：{:.0} 檔案/秒",
                action, count, size as f64 / 1_048_576.0, speed
            ),
            None => format!(
                "{}：{} 檔案，速度：{:.0} 檔案/秒",
                action, count, speed
            ),
        };
        self.pb.set_message(msg);
        self.pb.set_position(count);
    }

    pub fn finish(&self, file_count: u64, total_size: Option<usize>, skipped_dirs: u64) {
        if self.no_progress {
            return;
        }
        let msg = match total_size {
            Some(size) => format!(
                "完成，共 {} 個檔案，總大小：{:.2} MB，跳過 {} 個目錄",
                file_count,
                size as f64 / 1_048_576.0,
                skipped_dirs
            ),
            None => format!(
                "完成，共 {} 個檔案，跳過 {} 個目錄",
                file_count,
                skipped_dirs
            ),
        };
        self.pb.finish_with_message(msg);
    }
}

pub fn create_progress_bar(total: u64, no_progress: bool) -> ProgressManager {
    ProgressManager::new(total, no_progress)
}

pub fn manage_progress(
    pm: &ProgressManager,
    count: u64,
    total_size: Option<usize>,
    _start: Instant,
    no_progress: bool,
    action: &str,
) {
    if no_progress {
        return;
    }
    pm.update(count, total_size, action);
}

pub fn finalize_progress(
    pm: &ProgressManager,
    file_count: u64,
    total_size: Option<usize>,
    skipped_dirs: u64,
    no_progress: bool,
) {
    if no_progress {
        return;
    }
    pm.finish(file_count, total_size, skipped_dirs);
}

pub fn get_file_name(path: &Path, layer: &str) -> (String, String) {
    let file_name = path.file_name()
        .unwrap_or(std::ffi::OsStr::new("archive"))
        .to_string_lossy()
        .to_string();
    let download_zip_name = match layer {
        "none" => file_name.clone(),
        "single" => format!("{}.zip", file_name),
        _ => format!("{}_outer.zip", file_name),
    };
    (file_name, download_zip_name)
}

pub fn copy_file_content<W: Write>(file_path: &Path, writer: &mut W) -> io::Result<usize> {
    let file = File::open(file_path)?;
    let metadata = file.metadata()?;
    let file_size = metadata.len() as usize;
    let mut reader = std::io::BufReader::with_capacity(4 * 1024 * 1024, file);
    std::io::copy(&mut reader, writer)?;
    Ok(file_size)
}

pub fn generate_random_password(length: usize) -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

pub fn generate_password(password_mode: &PasswordMode, preset_password: Option<String>) -> io::Result<Option<String>> {
    match password_mode {
        PasswordMode::Random => {
            let pwd = generate_random_password(16);
            log::info!("生成隨機密碼：{}", pwd);
            Ok(Some(pwd))
        }
        PasswordMode::Manual => {
            if let Some(pwd) = preset_password {
                log::info!("使用預設手動輸入密碼");
                Ok(Some(pwd))
            } else {
                let pwd = dialoguer::Password::new()
                    .with_prompt("請輸入 ZIP 加密密碼")
                    .interact()
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("密碼輸入失敗: {}", e)))?;
                let confirm_pwd = dialoguer::Password::new()
                    .with_prompt("請再次輸入密碼以確認")
                    .interact()
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("密碼確認失敗: {}", e)))?;
                if pwd != confirm_pwd {
                    Err(io::Error::new(io::ErrorKind::InvalidInput, "密碼不匹配"))
                } else {
                    log::info!("使用手動輸入密碼");
                    Ok(Some(pwd))
                }
            }
        }
        PasswordMode::Timestamp => {
            let pwd = Local::now().format("%Y%m%d%H%M%S").to_string();
            log::info!("使用時間戳密碼：{}", pwd);
            Ok(Some(pwd))
        }
        PasswordMode::None => {
            log::info!("選擇無密碼模式，ZIP 不加密");
            Ok(None)
        }
    }
}

pub fn format_file_size(size: usize) -> String {
    if size < 1024 * 1024 {
        format!("{:.2} KB", size as f64 / 1024.0)
    } else {
        format!("{:.2} MB", size as f64 / (1024.0 * 1024.0))
    }
}

pub fn create_regex_sets(include: &[String], exclude: &[String]) -> (RegexSet, RegexSet) {
    let include_patterns: Vec<_> = include.iter()
        .map(|p| p.replace(".", "\\.").replace("*", ".*"))
        .collect();
    let exclude_patterns: Vec<_> = exclude.iter()
        .map(|p| p.replace(".", "\\.").replace("*", ".*"))
        .collect();

    let include_set = RegexSet::new(&include_patterns)
        .unwrap_or_else(|e| {
            log::warn!("無效的包含模式: {}，使用空集作為回退", e);
            RegexSet::empty()
        });

    let exclude_set = RegexSet::new(&exclude_patterns)
        .unwrap_or_else(|e| {
            log::warn!("無效的排除模式: {}，使用空集作為回退", e);
            RegexSet::empty()
        });

    (include_set, exclude_set)
}