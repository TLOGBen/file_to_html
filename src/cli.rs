use clap::{Parser, ValueEnum};
use dialoguer::{Input, Password, Select, Confirm};
use std::io;
use std::path::Path;
use regex::RegexSet;

use crate::utils::{setup_logging, create_regex_sets, generate_password};
use crate::convert::execute_conversion;

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

pub fn process_args(args: Vec<String>) -> io::Result<String> {
    if args.len() == 1 {
        process_interactive_mode()
    } else {
        process_cli_mode()
    }
}

pub fn process_interactive_mode() -> io::Result<String> {
    println!("=== 歡迎使用互動模式 ===");
    let input = get_input_path()?;
    let (is_compressed, password_mode, display_password, layer, encryption_method) = get_conversion_mode_and_password()?;
    let output = get_output_path()?;
    let (include, exclude) = get_file_patterns()?;
    let (compress, compression_level) = get_compression_options(is_compressed)?;
    let no_progress = get_no_progress_option()?;
    let max_size = get_max_size_option()?;
    let log_level = get_log_level_option()?;

    setup_logging(&log_level)?;
    execute_conversion(
        &input,
        &output,
        is_compressed,
        compress,
        &include,
        &exclude.unwrap_or_default(),
        password_mode,
        display_password,
        &compression_level,
        &layer,
        &encryption_method,
        no_progress,
        max_size,
    )
}

pub fn get_input_path() -> io::Result<String> {
    Input::new()
        .with_prompt("請輸入檔案或目錄路徑（例如：./myfile.txt 或 ./mydir）")
        .validate_with(|input: &String| -> Result<(), String> {
            if Path::new(input).exists() { Ok(()) } else { Err(format!("路徑 '{}' 不存在", input)) }
        })
        .interact_text()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
}

pub fn get_conversion_mode() -> io::Result<bool> {
    let is_compressed = Select::new()
        .with_prompt("選擇轉換模式（使用方向鍵選擇，按 Enter 確認）")
        .items(&["個別 - 為每個檔案生成單獨的 HTML", "壓縮 - 壓縮成單個 ZIP 嵌入 HTML"])
        .default(0)
        .interact()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("轉換模式選擇失敗: {}", e)))? == 1;
    Ok(is_compressed)
}

pub fn get_zip_layer(is_compressed: bool) -> io::Result<String> {
    let (items, default) = if is_compressed {
        (vec!["單層 - 僅生成一層 ZIP", "雙層 - 生成外層和內層 ZIP（預設）"], 1)
    } else {
        (vec!["不壓縮", "單層 - 僅生成一層 ZIP", "雙層 - 生成外層和內層 ZIP（預設）"], 0)
    };

    let layer = Select::new()
        .with_prompt("選擇 ZIP 層數（使用方向鍵選擇，按 Enter 確認）")
        .items(&items)
        .default(default)
        .interact()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("ZIP 層數選擇失敗: {}", e)))?;

    Ok(match (is_compressed, layer) {
        (true, 0) => "single".to_string(),
        (true, 1) => "double".to_string(),
        (false, 0) => "none".to_string(),
        (false, 1) => "single".to_string(),
        (false, 2) => "double".to_string(),
        _ => unreachable!(),
    })
}

pub fn get_password_options(layer: &str) -> io::Result<(PasswordMode, bool)> {
    if layer == "none" {
        return Ok((PasswordMode::None, false));
    }

    let modes = ["隨機生成（16 位，預設）", "手動輸入", "時間戳（yyyyMMddhhmmss）", "無密碼"];
    let mode = Select::new()
        .with_prompt("選擇密碼模式（使用方向鍵選擇，按 Enter 確認）")
        .items(&modes)
        .default(0)
        .interact()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("密碼模式選擇失敗: {}", e)))?;

    let password_mode = match mode {
        0 => PasswordMode::Random,
        1 => PasswordMode::Manual,
        2 => PasswordMode::Timestamp,
        3 => PasswordMode::None,
        _ => unreachable!(),
    };

    let display_password = match mode {
        0 => Confirm::new()
            .with_prompt("是否在 HTML 中顯示隨機生成的密碼？（預設為是）")
            .default(true)
            .interact()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("密碼顯示選項輸入失敗: {}", e)))?,
        3 => false,
        _ => Confirm::new()
            .with_prompt("是否在 HTML 中顯示密碼？（預設為否，將儲存至 .key 檔案）")
            .default(false)
            .interact()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("密碼顯示選項輸入失敗: {}", e)))?,
    };

    Ok((password_mode, display_password))
}

pub fn get_conversion_mode_and_password() -> io::Result<(bool, PasswordMode, bool, String, String)> {
    let is_compressed = get_conversion_mode()?;
    let layer = get_zip_layer(is_compressed)?;
    let (password_mode, display_password) = get_password_options(&layer)?;
    let encryption_method = "aes256".to_string();
    Ok((is_compressed, password_mode, display_password, layer, encryption_method))
}

pub fn get_output_path() -> io::Result<String> {
    Input::new()
        .with_prompt("輸入輸出目錄（例如：./output，預設為 output）")
        .default("output".to_string())
        .interact_text()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
}

pub fn get_file_patterns() -> io::Result<(Vec<String>, Option<Vec<String>>)> {
    let include = Input::new()
        .with_prompt("輸入包含模式（例如：.txt,.pdf，預設為 *）")
        .default("*".to_string())
        .interact_text()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("包含模式輸入失敗: {}", e)))?
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<String>>();

    let exclude = Input::new()
        .with_prompt("輸入排除模式（例如：.jpg,.png，預設為空）")
        .default("".to_string())
        .interact_text()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("排除模式輸入失敗: {}", e)))?
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<String>>();

    Ok((include, if exclude.is_empty() { None } else { Some(exclude) }))
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

pub fn get_compression_options(is_compressed: bool) -> io::Result<(bool, String)> {
    let compress = if !is_compressed {
        Confirm::new()
            .with_prompt("是否在個別模式下將檔案壓縮為 ZIP？")
            .default(true)
            .interact()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("壓縮選項輸入失敗: {}", e)))?
    } else {
        true
    };
    Ok((compress, "deflated".to_string()))
}

pub fn get_no_progress_option() -> io::Result<bool> {
    Ok(false)
}

pub fn get_max_size_option() -> io::Result<Option<f64>> {
    Ok(None)
}

pub fn get_log_level_option() -> io::Result<String> {
    Ok("error".to_string())
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

pub fn prompt_manual_password() -> io::Result<String> {
    let pwd = Password::new()
        .with_prompt("請輸入 ZIP 加密密碼")
        .interact()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("密碼輸入失敗: {}", e)))?;
    let confirm_pwd = Password::new()
        .with_prompt("請再次輸入密碼以確認")
        .interact()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("密碼確認失敗: {}", e)))?;
    if pwd != confirm_pwd {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "密碼不匹配"));
    }
    Ok(pwd)
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

pub fn process_cli_mode() -> io::Result<String> {
    let cli = Cli::parse();
    validate_cli_args(&cli)?;
    setup_logging(&cli.log_level)?;

    let (include_set, exclude_set) = create_regex_sets(&cli.include, &cli.exclude.as_deref().unwrap_or(&[]).to_vec());
    let display_password = cli.display_password.unwrap_or_else(|| cli.password_mode == "random");
    let password_mode = match cli.password_mode.as_str() {
        "random" => PasswordMode::Random,
        "manual" => PasswordMode::Manual,
        "timestamp" => PasswordMode::Timestamp,
        "none" => PasswordMode::None,
        _ => PasswordMode::Random,
    };

    let preset_password = if cli.password_mode == "manual" {
        Some(prompt_manual_password()?)
    } else {
        None
    };

    match cli.mode {
        Mode::Individual => {
            log::info!("開始個別轉換，輸入路徑：{}，輸出目錄：{}，包含模式：{:?}",
                  cli.input, cli.output, cli.include);
            crate::convert::process_individual(
                Path::new(&cli.input),
                &cli.output,
                &include_set,
                &exclude_set,
                cli.compress,
                &cli.compression_level,
                password_mode,
                display_password,
                &cli.layer,
                &cli.encryption_method,
                cli.no_progress,
                cli.max_size,
                preset_password,
            )?;
        }
        Mode::Compressed => {
            log::info!("開始壓縮轉換，輸入路徑：{}，輸出目錄：{}，包含模式：{:?}",
                  cli.input, cli.output, cli.include);
            crate::convert::process_compressed(
                Path::new(&cli.input),
                &cli.output,
                &include_set,
                &exclude_set,
                password_mode,
                display_password,
                &cli.compression_level,
                &cli.layer,
                &cli.encryption_method,
                cli.no_progress,
                cli.max_size,
                preset_password,
            )?;
        }
    }

    Ok(cli.output)
}