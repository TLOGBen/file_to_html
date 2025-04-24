use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use base64::{engine::general_purpose, Engine as _};
use clap::{Parser, ValueEnum};
use dialoguer::{Input, Password, Select, Confirm};
use pathdiff::diff_paths;
use log::{info, warn, error};
use env_logger;
use zip::write::{FileOptions, ZipWriter};
use zip::CompressionMethod;
use zip::AesMode;
use indicatif::{ProgressBar, ProgressStyle};
use regex::RegexSet;
use rand::{Rng};
use rand::distributions::Alphanumeric;
use chrono::Local;

/// 命令列參數結構，用於解析使用者輸入的參數
#[derive(Parser)]
#[command(
    name = "file_to_html",
    about = "將檔案或目錄轉換為嵌入式 HTML 格式",
    long_about = "一個將檔案或目錄轉換為 HTML 格式的工具，支援單一檔案轉換或壓縮成單一 ZIP 檔案並嵌入 HTML，內嵌單層或雙層 ZIP（可選擇加密）。\n使用 `--help` 查看詳細用法。",
    arg_required_else_help = true
)]
struct Cli {
    /// 輸入檔案或目錄路徑，必須存在
    input: String,

    /// 輸出目錄路徑，預設為 "output"
    #[arg(short, long, default_value = "output")]
    output: String,

    /// 轉換模式：individual（每個檔案產生獨立的 HTML）或 compressed（壓縮成單一 ZIP）
    #[arg(long, default_value = "individual")]
    mode: Mode,

    /// 僅處理指定副檔名的檔案，支援多次指定，例如：*.txt *.pdf，預設為 "*"
    #[arg(long, default_value = "*", value_delimiter = ',')]
    include: Vec<String>,

    /// 排除指定副檔名的檔案，支援多次指定，例如：*.jpg *.png
    #[arg(long, value_delimiter = ',')]
    exclude: Option<Vec<String>>,

    /// 是否在個別模式下將檔案內容壓縮為 ZIP，預設為 true
    #[arg(long, default_value_t = true)]
    compress: bool,

    /// 密碼模式：random（隨機生成）、manual（手動輸入）、timestamp（時間戳 yyyyMMddhhmmss）、none（無密碼），預設為 random
    #[arg(long, default_value = "random", value_parser = ["random", "manual", "timestamp", "none"])]
    password_mode: String,

    /// 是否在 HTML 中顯示密碼，random 模式下預設為 true，其他模式為 false
    #[arg(long)]
    display_password: Option<bool>,

    /// ZIP 壓縮等級：stored（無壓縮）或 deflated（壓縮），預設為 deflated
    #[arg(long, default_value = "deflated", value_parser = ["stored", "deflated"])]
    compression_level: String,

    /// ZIP 層數：none（不壓縮，僅個別模式）、single（單層）或 double（雙層），預設為 double
    #[arg(long, default_value = "double", value_parser = ["none", "single", "double"])]
    layer: String,

    /// 加密方法：aes128、aes192、aes256，預設為 aes256
    #[arg(long, default_value = "aes256", value_parser = ["aes128", "aes192", "aes256"])]
    encryption_method: String,

    /// 禁用進度條
    #[arg(long, default_value_t = false)]
    no_progress: bool,

    /// 限制檔案大小（MB），超過此大小的檔案將被跳過
    #[arg(long)]
    max_size: Option<f64>,

    /// 日誌級別：info、warn、error，預設為 info
    #[arg(long, default_value = "info", value_parser = ["info", "warn", "error"])]
    log_level: String,
}

/// 轉換模式列舉，定義個別轉換和壓縮轉換兩種模式
#[derive(Clone, ValueEnum)]
enum Mode {
    Individual,
    Compressed,
}

/// 密碼模式列舉，定義隨機、手動、時間戳和無密碼四種方式
#[derive(Clone, PartialEq, Debug)]
enum PasswordMode {
    Random,
    Manual,
    Timestamp,
    None,
}

/// HTML 模板，使用繁體中文，包含下載連結和 Base64 展示，無 JavaScript
const HTML_TEMPLATE: &str = r#"
<!DOCTYPE html>
<html lang="zh-TW">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>檔案下載</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
            text-align: center;
            background-color: #f4f7fa;
            padding: 40px;
            margin: 0;
            color: #333;
        }
        h1 {
            font-size: 28px;
            color: #2c3e50;
            margin-bottom: 20px;
        }
        p {
            font-size: 16px;
            color: #5a6a7a;
            margin: 10px 0;
        }
        .password-display {
            display: inline-block;
            background-color: #e9ecef;
            padding: 8px 12px;
            border-radius: 5px;
            font-family: 'Courier New', Courier, monospace;
            font-size: 16px;
            color: #2c3e50;
            margin: 10px 0;
        }
        .base64-data {
            background-color: #e9ecef;
            padding: 15px;
            border-radius: 5px;
            font-family: 'Courier New', Courier, monospace;
            font-size: 14px;
            max-height: 300px;
            overflow-x: auto;
            overflow-y: auto;
            text-align: left;
            white-space: pre-wrap;
            word-break: break-all;
            margin: 20px auto;
            max-width: 90%;
        }
        a {
            display: inline-block;
            padding: 12px 24px;
            background-color: #007bff;
            color: white;
            text-decoration: none;
            border-radius: 5px;
            font-size: 16px;
            margin: 20px 0;
            transition: background-color 0.3s;
        }
        a:hover {
            background-color: #0056b3;
        }
        .container {
            max-width: 800px;
            margin: 0 auto;
            background: white;
            padding: 30px;
            border-radius: 10px;
            box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
        }
        @media (max-width: 600px) {
            .container {
                padding: 20px;
                max-width: 95%;
            }
            h1 {
                font-size: 24px;
            }
            p, a {
                font-size: 14px;
            }
            .base64-data {
                font-size: 12px;
                padding: 10px;
            }
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>檔案下載</h1>
        <p>檔案名稱：{{FILE_NAME}}</p>
        <p>檔案大小：{{FILE_SIZE}}</p>
        {{INSTRUCTIONS}}
        {{PASSWORD_DISPLAY}}
        <a href="data:application/zip;base64,{{ZIP_BASE64}}" download="{{DOWNLOAD_ZIP_NAME}}">下載 ZIP 檔案</a>
        <p>或複製下方 Base64 資料並手動解碼為 ZIP 檔案：</p>
        <pre class="base64-data">{{ZIP_BASE64}}</pre>
    </div>
</body>
</html>
"#;

/// 主函數，程式入口點，負責初始化並協調執行
fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let output_dir = process_args(args)?;
    info!("程式執行完成，輸出目錄：{}", output_dir);
    println!("轉換完成！輸出檔案位於：{}", output_dir);
    Ok(())
}

/// 根據參數數量決定進入互動模式或命令列模式
fn process_args(args: Vec<String>) -> io::Result<String> {
    if args.len() == 1 {
        process_interactive_mode()
    } else {
        process_cli_mode()
    }
}

/// 互動模式處理，收集輸入並執行轉換
fn process_interactive_mode() -> io::Result<String> {
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

/// 收集輸入路徑，確保路徑存在
fn get_input_path() -> io::Result<String> {
    Input::new()
        .with_prompt("請輸入檔案或目錄路徑（例如：./myfile.txt 或 ./mydir）")
        .validate_with(|input: &String| -> Result<(), String> {
            if Path::new(input).exists() { Ok(()) } else { Err(format!("路徑 '{}' 不存在", input)) }
        })
        .interact_text()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
}

/// 收集轉換模式
fn get_conversion_mode() -> io::Result<bool> {
    let is_compressed = Select::new()
        .with_prompt("選擇轉換模式（使用方向鍵選擇，按 Enter 確認）")
        .items(&["個別 - 為每個檔案生成單獨的 HTML", "壓縮 - 壓縮成單個 ZIP 嵌入 HTML"])
        .default(0)
        .interact()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("轉換模式選擇失敗: {}", e)))? == 1;
    Ok(is_compressed)
}

/// 收集 ZIP 層數
fn get_zip_layer(is_compressed: bool) -> io::Result<String> {
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

/// 收集密碼模式和顯示選項
fn get_password_options(layer: &str) -> io::Result<(PasswordMode, bool)> {
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

/// 收集轉換模式、密碼選項、ZIP 層數和加密方法
fn get_conversion_mode_and_password() -> io::Result<(bool, PasswordMode, bool, String, String)> {
    let is_compressed = get_conversion_mode()?;
    let layer = get_zip_layer(is_compressed)?;
    let (password_mode, display_password) = get_password_options(&layer)?;
    let encryption_method = "aes256".to_string(); // 預設加密方法
    Ok((is_compressed, password_mode, display_password, layer, encryption_method))
}

/// 收集輸出路徑，預設為 "output"
fn get_output_path() -> io::Result<String> {
    Input::new()
        .with_prompt("輸入輸出目錄（例如：./output，預設為 output）")
        .default("output".to_string())
        .interact_text()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
}

/// 收集檔案過濾模式（包含和排除模式）
fn get_file_patterns() -> io::Result<(Vec<String>, Option<Vec<String>>)> {
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

/// 驗證通配符模式
fn is_valid_pattern(pattern: &str) -> bool {
    let invalid_chars = ['/', '\\', ':', '?', '"', '<', '>', '|'];
    !pattern.is_empty() && !pattern.contains(&invalid_chars[..])
}

/// 驗證檔案過濾模式
fn validate_file_patterns(include: &[String], exclude: &Option<Vec<String>>) -> io::Result<()> {
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

/// 收集壓縮選項（僅個別模式需要）
fn get_compression_options(is_compressed: bool) -> io::Result<(bool, String)> {
    let compress = if !is_compressed {
        Confirm::new()
            .with_prompt("是否在個別模式下將檔案壓縮為 ZIP？")
            .default(true)
            .interact()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("壓縮選項輸入失敗: {}", e)))?
    } else {
        true
    };
    Ok((compress, "deflated".to_string())) // 預設壓縮級別
}

/// 收集是否禁用進度條
fn get_no_progress_option() -> io::Result<bool> {
    Ok(false) // 預設不禁用進度條
}

/// 收集檔案大小限制
fn get_max_size_option() -> io::Result<Option<f64>> {
    Ok(None) // 預設無限制
}

/// 收集日誌級別
fn get_log_level_option() -> io::Result<String> {
    Ok("info".to_string()) // 預設日誌級別
}

/// 設置日誌級別
fn setup_logging(log_level: &str) -> io::Result<()> {
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

/// 驗證 CLI 參數
fn validate_cli_args(cli: &Cli) -> io::Result<()> {
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

/// 處理手動密碼輸入
fn prompt_manual_password() -> io::Result<String> {
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

/// 命令列模式處理，解析 CLI 參數並執行轉換
fn process_cli_mode() -> io::Result<String> {
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
            info!("開始個別轉換，輸入路徑：{}，輸出目錄：{}，包含模式：{:?}",
                  cli.input, cli.output, cli.include);
            process_individual(
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
            info!("開始壓縮轉換，輸入路徑：{}，輸出目錄：{}，包含模式：{:?}",
                  cli.input, cli.output, cli.include);
            process_compressed(
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

/// 執行轉換，根據模式分派處理
fn execute_conversion(
    input: &str,
    output: &str,
    is_compressed: bool,
    compress: bool,
    include: &[String],
    exclude: &[String],
    password_mode: PasswordMode,
    display_password: bool,
    compression_level: &str,
    layer: &str,
    encryption_method: &str,
    no_progress: bool,
    max_size: Option<f64>,
) -> io::Result<String> {
    let input_path = validate_input_path(input)?;
    let (include_set, exclude_set) = create_regex_sets(include, exclude);

    if is_compressed {
        info!("開始壓縮轉換，輸入路徑：{}，輸出目錄：{}", input, output);
        process_compressed(
            input_path,
            output,
            &include_set,
            &exclude_set,
            password_mode,
            display_password,
            compression_level,
            layer,
            encryption_method,
            no_progress,
            max_size,
            None,
        )?;
    } else {
        info!("開始個別轉換，輸入路徑：{}，輸出目錄：{}", input, output);
        process_individual(
            input_path,
            output,
            &include_set,
            &exclude_set,
            compress,
            compression_level,
            password_mode,
            display_password,
            layer,
            encryption_method,
            no_progress,
            max_size,
            None,
        )?;
    }

    Ok(output.to_string())
}

/// 驗證輸入路徑是否存在
fn validate_input_path(input: &str) -> io::Result<&Path> {
    let path = Path::new(input);
    if !path.exists() {
        error!("輸入路徑不存在：{}", input);
        return Err(io::Error::new(io::ErrorKind::NotFound, format!("輸入路徑 '{}' 不存在", input)));
    }
    Ok(path)
}

/// 創建正則表達式集，用於檔案過濾
fn create_regex_sets(include: &[String], exclude: &[String]) -> (RegexSet, RegexSet) {
    let include_patterns: Vec<_> = include.iter()
        .map(|p| p.replace(".", "\\.").replace("*", ".*"))
        .collect();
    let exclude_patterns: Vec<_> = exclude.iter()
        .map(|p| p.replace(".", "\\.").replace("*", ".*"))
        .collect();

    let include_set = RegexSet::new(&include_patterns)
        .unwrap_or_else(|e| {
            warn!("無效的包含模式: {}，使用空集作為回退", e);
            RegexSet::empty()
        });

    let exclude_set = RegexSet::new(&exclude_patterns)
        .unwrap_or_else(|e| {
            warn!("無效的排除模式: {}，使用空集作為回退", e);
            RegexSet::empty()
        });

    (include_set, exclude_set)
}

/// 生成隨機密碼
fn generate_random_password(length: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

/// 格式化檔案大小為 KB 或 MB
fn format_file_size(size: usize) -> String {
    if size < 1024 * 1024 {
        format!("{:.2} KB", size as f64 / 1024.0)
    } else {
        format!("{:.2} MB", size as f64 / (1024.0 * 1024.0))
    }
}

/// 創建未加密的單檔案 ZIP
fn create_zip_buffer(file_name: &str, data: &[u8], options: FileOptions<()>) -> io::Result<Vec<u8>> {
    let mut zip_buffer = Vec::new();
    let mut zip = ZipWriter::new(std::io::Cursor::new(&mut zip_buffer));
    zip.start_file(file_name, options)?;
    zip.write_all(data)?;
    zip.finish()?;
    Ok(zip_buffer)
}

/// 生成密碼
fn generate_password(password_mode: &PasswordMode, preset_password: Option<String>) -> io::Result<Option<String>> {
    match password_mode {
        PasswordMode::Random => Ok(Some(generate_random_password(16))),
        PasswordMode::Manual => {
            if let Some(pwd) = preset_password {
                Ok(Some(pwd))
            } else {
                let pwd = Password::new()
                    .with_prompt("請輸入 ZIP 加密密碼")
                    .interact()
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("密碼輸入失敗: {}", e)))?;
                let confirm_pwd = Password::new()
                    .with_prompt("請再次輸入密碼以確認")
                    .interact()
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("密碼確認失敗: {}", e)))?;
                if pwd != confirm_pwd {
                    Err(io::Error::new(io::ErrorKind::InvalidInput, "密碼不匹配"))
                } else {
                    Ok(Some(pwd))
                }
            }
        }
        PasswordMode::Timestamp => Ok(Some(Local::now().format("%Y%m%d%H%M%S").to_string())),
        PasswordMode::None => Ok(None),
    }
}

/// 生成解壓說明
fn generate_instructions(layer: &str, has_password: bool) -> String {
    match (layer, has_password) {
        ("double", true) => "<p>請使用下載連結或複製 Base64 資料手動解碼為 ZIP 檔案，然後使用密碼解壓外層和內層 ZIP（使用相同密碼）。建議使用 7-Zip 或 WinRAR。</p>".to_string(),
        ("double", false) => "<p>請使用下載連結或複製 Base64 資料手動解碼為 ZIP 檔案，然後無需密碼解壓外層和內層 ZIP。建議使用 7-Zip 或 WinRAR。</p>".to_string(),
        ("single", true) => "<p>請使用下載連結或複製 Base64 資料手動解碼為 ZIP 檔案，然後使用密碼解壓 ZIP。建議使用 7-Zip 或 WinRAR。</p>".to_string(),
        ("single", false) => "<p>請使用下載連結或複製 Base64 資料手動解碼為 ZIP 檔案，然後無需密碼解壓 ZIP。建議使用 7-Zip 或 WinRAR。</p>".to_string(),
        _ => "<p>請使用下載連結或複製 Base64 資料手動解碼為檔案，無需解壓。</p>".to_string(),
    }
}

/// 處理密碼顯示或儲存
fn handle_password_display(
    password: Option<&str>,
    display_password: bool,
    file_name: &str,
    output_dir: &str,
) -> io::Result<(String, String)> {
    if let Some(pwd) = password {
        if display_password {
            Ok(("下方密碼".to_string(), format!("<p>密碼：<span class=\"password-display\">{}</span></p>", pwd)))
        } else {
            let key_file = format!("{}.html.key", file_name);
            fs::write(Path::new(output_dir).join(&key_file), pwd)?;
            Ok((format!("{}.html.key 檔案", file_name), "".to_string()))
        }
    } else {
        Ok(("無需密碼".to_string(), "".to_string()))
    }
}

/// 生成 HTML 內容
fn generate_html_content(
    zip_base64: &str,
    file_name: &str,
    download_zip_name: &str,
    instructions: &str,
    file_size_str: &str,
    password_info: &str,
    password_display: &str,
) -> String {
    HTML_TEMPLATE
        .replace("{{ZIP_BASE64}}", zip_base64)
        .replace("{{FILE_NAME}}", file_name)
        .replace("{{DOWNLOAD_ZIP_NAME}}", download_zip_name)
        .replace("{{INSTRUCTIONS}}", instructions)
        .replace("{{FILE_SIZE}}", file_size_str)
        .replace("{{PASSWORD}}", password_info)
        .replace("{{PASSWORD_DISPLAY}}", password_display)
}

/// 讀取檔案內容
fn read_file_content(file_path: &Path) -> io::Result<(Vec<u8>, usize)> {
    let mut file = File::open(file_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    let file_size = buffer.len();
    Ok((buffer, file_size))
}

/// 壓縮檔案內容
fn compress_file_content(
    data: &[u8],
    file_name: &str,
    compression_level: &str,
    password: Option<&str>,
    aes_mode: AesMode,
) -> io::Result<Vec<u8>> {
    let mut zip_buffer = Vec::new();
    let mut zip = ZipWriter::new(std::io::Cursor::new(&mut zip_buffer));
    if let Some(pwd) = password {
        let options: FileOptions<zip::write::ExtendedFileOptions> = FileOptions::default()
            .compression_method(if compression_level == "stored" {
                CompressionMethod::Stored
            } else {
                CompressionMethod::Deflated
            })
            .with_aes_encryption(aes_mode, pwd);
        zip.start_file(file_name.to_string(), options)?;
    } else {
        let options: FileOptions<()> = FileOptions::default()
            .compression_method(if compression_level == "stored" {
                CompressionMethod::Stored
            } else {
                CompressionMethod::Deflated
            });
        zip.start_file(file_name.to_string(), options)?;
    }
    zip.write_all(data)?;
    zip.finish()?;
    Ok(zip_buffer)
}

/// 創建單層或雙層 ZIP
fn create_zip(
    data: &[u8],
    file_name: &str,
    layer: &str,
    password: Option<&str>,
    aes_mode: AesMode,
) -> io::Result<Vec<u8>> {
    if layer == "double" {
        // 直接創建外層 ZIP，包裝輸入數據
        let mut outer_zip_buffer = Vec::new();
        let mut outer_zip = ZipWriter::new(std::io::Cursor::new(&mut outer_zip_buffer));
        if let Some(pwd) = password {
            let outer_options: FileOptions<zip::write::ExtendedFileOptions> = FileOptions::default()
                .compression_method(CompressionMethod::Deflated)
                .with_aes_encryption(aes_mode, pwd);
            outer_zip.start_file(format!("{}_outer.zip", file_name), outer_options)?;
            outer_zip.write_all(data)?;
            outer_zip.finish()?;
            info!("生成外層加密 ZIP，密碼：{}，大小：{} 位元組", pwd, outer_zip_buffer.len());
        } else {
            let outer_options: FileOptions<()> = FileOptions::default()
                .compression_method(CompressionMethod::Deflated);
            outer_zip.start_file(format!("{}_outer.zip", file_name), outer_options)?;
            outer_zip.write_all(data)?;
            outer_zip.finish()?;
            info!("生成外層無密碼 ZIP，大小：{} 位元組", outer_zip_buffer.len());
        }
        Ok(outer_zip_buffer)
    } else if layer == "single" {
        let mut zip_buffer = Vec::new();
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut zip_buffer));
        if let Some(pwd) = password {
            let options: FileOptions<zip::write::ExtendedFileOptions> = FileOptions::default()
                .compression_method(CompressionMethod::Deflated)
                .with_aes_encryption(aes_mode, pwd);
            zip.start_file(format!("{}.zip", file_name), options)?;
            zip.write_all(data)?;
            zip.finish()?;
            info!("生成單層加密 ZIP，密碼：{}，大小：{} 位元組", pwd, zip_buffer.len());
        } else {
            let options: FileOptions<()> = FileOptions::default()
                .compression_method(CompressionMethod::Deflated);
            zip.start_file(format!("{}.zip", file_name), options)?;
            zip.write_all(data)?;
            zip.finish()?;
            info!("生成單層無密碼 ZIP，大小：{} 位元組", zip_buffer.len());
        }
        Ok(zip_buffer)
    } else {
        Ok(data.to_vec())
    }
}

/// 將資料轉為 Base64 並檢查大小
fn encode_to_base64(data: &[u8], file_path: &Path) -> io::Result<String> {
    let zip_base64 = general_purpose::STANDARD.encode(data);
    const MAX_BASE64_SIZE: usize = 1_000_000; // 約 1MB
    if zip_base64.len() > MAX_BASE64_SIZE {
        warn!(
            "Base64 資料過大：{} 位元組，超過建議限制 {} 位元組，可能影響顯示或下載：{}",
            zip_base64.len(), MAX_BASE64_SIZE, file_path.display()
        );
    }
    Ok(zip_base64)
}

/// 寫入 HTML 檔案
fn write_html_file(html_content: &str, output_dir: &str, file_name: &str) -> io::Result<()> {
    let output_path = Path::new(output_dir).join(format!("{}.html", file_name));
    fs::write(&output_path, html_content)?;
    Ok(())
}

/// 將單一檔案轉換為 HTML 文件
fn convert_file_to_html(
    file_path: &Path,
    output_dir: &str,
    compress: bool,
    compression_level: &str,
    password: Option<String>,
    display_password: bool,
    layer: &str,
    encryption_method: &str,
) -> io::Result<()> {
    let file_name = file_path.file_name().unwrap().to_string_lossy();
    let download_zip_name = if layer == "none" {
        file_name.to_string()
    } else if layer == "single" {
        format!("{}.zip", file_name)
    } else {
        format!("{}_outer.zip", file_name)
    };

    // 讀取檔案內容
    let (mut data, file_size) = read_file_content(file_path)?;
    info!("讀取檔案：{}，原始大小：{} 位元組", file_path.display(), file_size);

    // 選擇加密方法
    let aes_mode = match encryption_method {
        "aes128" => AesMode::Aes128,
        "aes192" => AesMode::Aes192,
        "aes256" => AesMode::Aes256,
        _ => AesMode::Aes256,
    };

    // 創建最終 ZIP
    let final_zip_buffer = if layer == "single" {
        // 單層壓縮：直接生成單層 ZIP
        let options: FileOptions<()> = if compression_level == "stored" {
            FileOptions::default().compression_method(CompressionMethod::Stored)
        } else {
            FileOptions::default().compression_method(CompressionMethod::Deflated)
        };
        if let Some(ref pwd) = password {
            let mut zip_buffer = Vec::new();
            let mut zip = ZipWriter::new(std::io::Cursor::new(&mut zip_buffer));
            let encrypt_options: FileOptions<zip::write::ExtendedFileOptions> = FileOptions::default()
                .compression_method(CompressionMethod::Deflated)
                .with_aes_encryption(aes_mode, pwd);
            zip.start_file(file_name.to_string(), encrypt_options)?;
            zip.write_all(&data)?;
            zip.finish()?;
            info!("生成單層加密 ZIP，密碼：{}，大小：{} 位元組", pwd, zip_buffer.len());
            zip_buffer
        } else {
            let zip_buffer = if compress {
                create_zip_buffer(&file_name, &data, options)?
            } else {
                data
            };
            info!("生成單層無密碼 ZIP，大小：{} 位元組", zip_buffer.len());
            zip_buffer
        }
    } else {
        // 雙層壓縮或不壓縮
        let inner_data = if compress && layer != "none" {
            let zip_buffer = compress_file_content(&data, &file_name, compression_level, password.as_deref(), aes_mode)?;
            info!("壓縮檔案至內層 ZIP：{}，壓縮後大小：{} 位元組", file_path.display(), zip_buffer.len());
            if let Some(ref pwd) = password {
                info!("內層 ZIP 使用密碼：{}", pwd);
            }
            zip_buffer
        } else {
            info!("未壓縮檔案：{}，直接使用原始資料", file_path.display());
            data
        };
        create_zip(&inner_data, &file_name, layer, password.as_deref(), aes_mode)?
    };

    // 轉為 Base64
    let zip_base64 = encode_to_base64(&final_zip_buffer, file_path)?;
    info!("生成最終資料的 Base64，總大小：{} 位元組", zip_base64.len());

    // 生成解壓說明
    let instructions = generate_instructions(layer, password.is_some());

    // 處理密碼顯示
    let (password_info, password_display) = handle_password_display(
        password.as_deref(),
        display_password,
        &file_name,
        output_dir,
    )?;
    if password.is_some() && !display_password {
        info!("密碼已儲存至：{}.html.key", file_name);
    }

    // 格式化檔案大小
    let file_size_str = format_file_size(file_size);

    // 生成 HTML 內容
    let html_content = generate_html_content(
        &zip_base64,
        &file_name,
        &download_zip_name,
        &instructions,
        &file_size_str,
        &password_info,
        &password_display,
    );

    // 寫入 HTML 檔案
    write_html_file(&html_content, output_dir, &file_name)?;
    info!("生成 HTML 文件：{}/{}.html，大小：{} 位元組", output_dir, file_name, html_content.len());

    Ok(())
}

/// 創建進度條
fn create_progress_bar(total: u64, no_progress: bool) -> ProgressBar {
    if no_progress {
        ProgressBar::hidden()
    } else {
        let pb = ProgressBar::new(total);
        pb.set_style(ProgressStyle::default_bar().template("{msg} [{bar:40}] {pos}/{len}").unwrap());
        pb
    }
}

/// 驗證檔案是否符合條件
fn is_file_valid(
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
        let file_size = fs::metadata(path)?.len() as f64 / 1_048_576.0; // 轉換為 MB
        if file_size > max {
            warn!("檔案 {} 超過大小限制（{} MB > {} MB），跳過", path.display(), file_size, max);
            return Ok(false);
        }
    }
    Ok(true)
}

/// 收集符合條件的檔案
fn collect_files(
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

/// 收集檔案並計算總大小
fn collect_and_measure_files(
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

/// 創建內層 ZIP
fn create_inner_zip(
    input_path: &Path,
    files: &[PathBuf],
    options: FileOptions<()>,
    password: Option<&str>,
    aes_mode: AesMode,
) -> io::Result<Vec<u8>> {
    let mut inner_zip_buffer = Vec::new();
    let mut inner_zip = ZipWriter::new(std::io::Cursor::new(&mut inner_zip_buffer));
    for file_path in files {
        if let Some(relative_path) = diff_paths(file_path, input_path.parent().unwrap_or(input_path)) {
            let relative_path_str = relative_path.to_string_lossy().replace("\\", "/").trim_start_matches("./").to_string();
            let (data, _) = read_file_content(file_path)?;
            if let Some(pwd) = password {
                let encrypt_options: FileOptions<zip::write::ExtendedFileOptions> = FileOptions::default()
                    .compression_method(CompressionMethod::Deflated)
                    .with_aes_encryption(aes_mode, pwd);
                inner_zip.start_file(&relative_path_str, encrypt_options)?;
            } else {
                inner_zip.start_file(&relative_path_str, options.clone())?;
            }
            inner_zip.write_all(&data)?;
        }
    }
    inner_zip.finish()?;
    Ok(inner_zip_buffer)
}

/// 壓縮模式處理，將多個檔案壓縮為單一 ZIP 並嵌入 HTML
fn process_compressed(
    input_path: &Path,
    output_dir: &str,
    include_set: &RegexSet,
    exclude_set: &RegexSet,
    password_mode: PasswordMode,
    display_password: bool,
    compression_level: &str,
    layer: &str,
    encryption_method: &str,
    no_progress: bool,
    max_size: Option<f64>,
    preset_password: Option<String>,
) -> io::Result<()> {
    fs::create_dir_all(output_dir)?;
    let options: FileOptions<()> = if compression_level == "stored" {
        FileOptions::default().compression_method(CompressionMethod::Stored)
    } else {
        FileOptions::default().compression_method(CompressionMethod::Deflated)
    };

    let (files, total_size) = collect_and_measure_files(input_path, include_set, exclude_set, max_size)?;
    let total_files = files.len();
    info!("開始壓縮 {} 個檔案（內層 ZIP）", total_files);

    let password = generate_password(&password_mode, preset_password)?;
    if let Some(ref pwd) = password {
        info!("使用密碼：{}", pwd);
    } else {
        info!("選擇無密碼模式，ZIP 不加密");
    }

    let aes_mode = match encryption_method {
        "aes128" => AesMode::Aes128,
        "aes192" => AesMode::Aes192,
        "aes256" => AesMode::Aes256,
        _ => AesMode::Aes256,
    };

    let pb = create_progress_bar(total_files as u64, no_progress);
    let inner_zip_buffer = create_inner_zip(input_path, &files, options, password.as_deref(), aes_mode)?;
    for (index, file_path) in files.iter().enumerate() {
        pb.set_message(format!("壓縮檔案 {}/{}：{}", index + 1, total_files, file_path.display()));
        if index % 10 == 0 || index == total_files - 1 {
            pb.set_position((index + 1) as u64);
        }
    }
    pb.finish_with_message("內層 ZIP 壓縮完成");
    info!("內層 ZIP 壓縮完成，共處理 {} 個檔案，總大小：{} 位元組", total_files, total_size);
    if let Some(ref pwd) = password {
        info!("內層 ZIP 使用密碼：{}", pwd);
    }

    let file_name = input_path.file_name().unwrap_or(std::ffi::OsStr::new("archive")).to_string_lossy().to_string();
    let final_zip_buffer = if layer == "double" {
        // 雙層壓縮：使用 create_zip 生成雙層 ZIP
        create_zip(&inner_zip_buffer, &file_name, layer, password.as_deref(), aes_mode)?
    } else {
        // 單層壓縮：直接使用內層 ZIP
        inner_zip_buffer
    };

    let zip_base64 = encode_to_base64(&final_zip_buffer, input_path)?;
    info!("生成最終 ZIP 的 Base64，總大小：{} 位元組", zip_base64.len());

    let instructions = generate_instructions(layer, password.is_some());
    let (password_info, password_display) = handle_password_display(
        password.as_deref(),
        display_password,
        &file_name,
        output_dir,
    )?;
    if password.is_some() && !display_password {
        info!("密碼已儲存至：{}.html.key", file_name);
    }

    let file_size_str = format_file_size(total_size);
    let html_content = generate_html_content(
        &zip_base64,
        &file_name,
        &format!("{}_outer.zip", file_name),
        &instructions,
        &file_size_str,
        &password_info,
        &password_display,
    );

    write_html_file(&html_content, output_dir, &file_name)?;
    info!("生成 HTML 文件：{}/{}.html，大小：{} 位元組", output_dir, file_name, html_content.len());

    Ok(())
}

/// 個別模式處理，將每個檔案轉為獨立的 HTML 文件
fn process_individual(
    input_path: &Path,
    output_dir: &str,
    include_set: &RegexSet,
    exclude_set: &RegexSet,
    compress: bool,
    compression_level: &str,
    password_mode: PasswordMode,
    display_password: bool,
    layer: &str,
    encryption_method: &str,
    no_progress: bool,
    max_size: Option<f64>,
    preset_password: Option<String>,
) -> io::Result<()> {
    fs::create_dir_all(output_dir)?;
    let mut files = Vec::new();
    collect_files(input_path, &mut files, include_set, exclude_set, max_size)?;
    let total_files = files.len();
    info!("正在處理 {} 個檔案", total_files);

    if total_files == 0 {
        warn!("無符合條件的檔案可處理");
        return Ok(());
    }

    let password = generate_password(&password_mode, preset_password)?;
    if let Some(ref pwd) = password {
        info!("使用密碼：{}", pwd);
    } else {
        info!("選擇無密碼模式，ZIP 不加密");
    }

    let pb = create_progress_bar(total_files as u64, no_progress);
    for (i, file_path) in files.iter().enumerate() {
        pb.set_message(format!("處理檔案 {}/{}：{}", i + 1, total_files, file_path.display()));
        if let Err(e) = convert_file_to_html(
            file_path,
            output_dir,
            compress,
            compression_level,
            password.clone(),
            display_password,
            layer,
            encryption_method,
        ) {
            error!("處理檔案 {} 失敗: {}", file_path.display(), e);
        } else {
            pb.inc(1);
        }
    }
    pb.finish_with_message("處理完成");
    Ok(())
}