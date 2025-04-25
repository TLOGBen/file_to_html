use clap::{Parser, ValueEnum};
use std::io;
use std::path::Path;

#[derive(Parser)]
#[command(
    name = "file_to_html",
    about = "將檔案或目錄轉換為嵌入式 HTML 格式",
    long_about = "一個將檔案或目錄轉換為 HTML 格式的工具，支援單一檔案轉換或壓縮成單一 ZIP 檔案並嵌入 HTML，內嵌單層或雙層 ZIP（可選擇加密）。\n使用 `--help` 查看詳細用法。",
    arg_required_else_help = true
)]
pub struct Cli {
    pub input: String,
    #[arg(short, long, default_value = "output")]
    pub output: String,
    #[arg(long, default_value = "individual")]
    pub mode: Mode,
    #[arg(long, default_value = "*", value_delimiter = ',')]
    pub include: Vec<String>,
    #[arg(long, value_delimiter = ',')]
    pub exclude: Option<Vec<String>>,
    #[arg(long, default_value_t = true)]
    pub compress: bool,
    #[arg(long, default_value = "random", value_parser = ["random", "manual", "timestamp", "none"])]
    pub password_mode: String,
    #[arg(long)]
    pub display_password: Option<bool>,
    #[arg(long, default_value = "deflated", value_parser = ["stored", "deflated"])]
    pub compression_level: String,
    #[arg(long, default_value = "double", value_parser = ["none", "single", "double"])]
    pub layer: String,
    #[arg(long, default_value = "aes256", value_parser = ["aes128", "aes192", "aes256"])]
    pub encryption_method: String,
    #[arg(long, default_value_t = false)]
    pub no_progress: bool,
    #[arg(long)]
    pub max_size: Option<f64>,
    #[arg(long, default_value = "info", value_parser = ["info", "warn", "error"])]
    pub log_level: String,
}

#[derive(Clone, ValueEnum)]
pub enum Mode {
    Individual,
    Compressed,
}

#[derive(Clone, PartialEq, Debug)]
pub enum PasswordMode {
    Random,
    Manual,
    Timestamp,
    None,
}

pub fn validate_cli_args(cli: &Cli) -> io::Result<()> {
    validate_input_path(&cli.input)?;
    validate_file_patterns(&cli.include, &cli.exclude)?;
    if matches!(cli.mode, Mode::Compressed) && cli.layer == "none" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "壓縮模式下不支援 'none' 層數，請選擇 'single' 或 'double'"
        ));
    }
    Ok(())
}

pub fn validate_input_path(input: &str) -> io::Result<&Path> {
    let path = Path::new(input);
    if !path.exists() {
        log::error!("輸入路徑不存在：{}", input);
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("輸入路徑 '{}' 不存在", input)
        ));
    }
    Ok(path)
}

pub fn is_valid_pattern(pattern: &str) -> bool {
    let invalid_chars = ['/', '\\', ':', '?', '"', '<', '>', '|'];
    !pattern.is_empty() && !pattern.contains(&invalid_chars[..])
}

pub fn validate_file_patterns(include: &[String], exclude: &Option<Vec<String>>) -> io::Result<()> {
    for pattern in include {
        if !is_valid_pattern(pattern) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("無效的包含模式: {}", pattern)));
        }
    }
    if let Some(exclude_patterns) = exclude {
        for pattern in exclude_patterns {
            if !is_valid_pattern(pattern) {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("無效的排除模式: {}", pattern)));
            }
        }
    }
    Ok(())
}