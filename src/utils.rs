use std::io;
use regex::RegexSet;
use rand::{Rng, distributions::Alphanumeric};
use chrono::Local;
use indicatif::{ProgressBar, ProgressStyle};
use log;

use crate::cli::PasswordMode;

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

pub fn create_progress_bar(total: u64, no_progress: bool) -> ProgressBar {
    if no_progress {
        ProgressBar::hidden()
    } else {
        let pb = ProgressBar::new(total);
        pb.set_style(ProgressStyle::default_bar().template("{msg} [{bar:40}] {pos}/{len}").unwrap());
        pb
    }
}

pub fn generate_random_password(length: usize) -> String {
    rand::thread_rng()
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