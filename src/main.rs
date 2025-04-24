use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use base64::{engine::general_purpose, Engine as _};
use clap::{Parser, ValueEnum};
use dialoguer::{Input, Password, Select, Confirm};
use pathdiff::diff_paths;
use log::{info, warn, error, debug};
use env_logger;
use zip::write::{FileOptions, ZipWriter};
use zip::CompressionMethod;
use zip::AesMode;
use indicatif::{ProgressBar, ProgressStyle};
use regex::RegexSet;
use rand::{distributions::Alphanumeric, Rng};

/// 命令列參數結構，用於解析使用者輸入的參數
#[derive(Parser)]
#[command(
    name = "file_to_html",
    about = "將檔案或目錄轉換為嵌入式 HTML 格式",
    long_about = "一個將檔案或目錄轉換為 HTML 格式的工具，支援單一檔案轉換或壓縮成單一 ZIP 檔案並嵌入 HTML。\n使用 `--help` 查看詳細用法。",
    arg_required_else_help = true
)]
struct Cli {
    /// 輸入檔案或目錄路徑
    input: String,

    /// 輸出目錄路徑，預設為 "output"
    #[arg(short, long, default_value = "output")]
    output: String,

    /// 轉換模式：individual（每個檔案產生獨立的 HTML）或 compressed（壓縮成單一 ZIP）
    #[arg(long, default_value = "individual")]
    mode: Mode,

    /// 僅處理指定副檔名的檔案，支援正規表達式，例如：*.txt|*.pdf 或 txt,pdf
    #[arg(long, default_value = "*", value_delimiter = ',')]
    include: Vec<String>,

    /// 排除指定副檔名的檔案，支援正規表達式，例如：*.jpg|*.png
    #[arg(long, value_delimiter = ',')]
    exclude: Option<Vec<String>>,

    /// 是否在個別模式下將檔案內容壓縮為 ZIP，預設為 true
    #[arg(long, default_value_t = true)]
    compress: bool,

    /// ZIP 加密密碼（僅在壓縮模式下有效，若未提供則隨機生成）
    #[arg(long)]
    password: Option<String>,

    /// ZIP 壓縮等級（stored 或 deflated），預設為 "deflated"
    #[arg(long, default_value = "deflated", value_parser = ["stored", "deflated"])]
    compression_level: String,
}

/// 轉換模式列舉，定義兩種模式：個別轉換和壓縮轉換
#[derive(Clone, ValueEnum)]
enum Mode {
    Individual,
    Compressed,
}

/// HTML 模板，包含繁體中文文本，顯示隨機密碼、檔案大小並提供 ZIP 檔案下載，無 JavaScript
const HTML_TEMPLATE: &str = r#"
<!DOCTYPE html>
<html lang="zh-TW">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>檔案下載</title>
    <style>
        body {
            font-family: Arial, sans-serif;
            text-align: center;
            background-color: #f0f0f0;
            padding: 20px;
        }
        h1 {
            color: #333;
        }
        p {
            color: #666;
        }
        a {
            padding: 10px 20px;
            background-color: #007bff;
            color: white;
            text-decoration: none;
            border-radius: 5px;
            display: inline-block;
        }
        a:hover {
            background-color: #0056b3;
        }
        .password-display {
            padding: 8px;
            margin: 10px;
            background-color: #e9ecef;
            display: inline-block;
            font-family: monospace;
            font-size: 16px;
        }
    </style>
</head>
<body>
    <h1>檔案下載</h1>
    <p>檔案名稱: {{FILE_NAME}}</p>
    <p>檔案大小: {{FILE_SIZE}}</p>
    <p>請下載 ZIP 檔案並使用下方密碼解壓{{COMPRESSION_NOTE}}。</p>
    <p>密碼: <span class="password-display">{{PASSWORD}}</span></p>
    <a href="data:application/zip;base64,{{ZIP_BASE64}}" download="{{DOWNLOAD_ZIP_NAME}}">下載 ZIP 檔案</a>
    <p>使用說明：下載後使用 7-Zip、WinRAR 或其他支援 AES-256 加密的工具解壓縮，輸入上方密碼以取得原始檔案。</p>
</body>
</html>
"#;

/// 主函數，程式進入點
fn main() -> io::Result<()> {
    env_logger::init(); // 初始化日誌系統
    let args: Vec<String> = std::env::args().collect(); // 收集命令列參數
    let output_dir = process_args(args)?; // 處理參數並獲取輸出目錄
    println!("轉換完成！輸出檔案位於：{}", output_dir); // 輸出完成訊息
    info!("程式執行完成，輸出目錄：{}", output_dir); // 記錄完成日誌
    Ok(())
}

/// 處理命令列參數，根據參數數量決定模式
fn process_args(args: Vec<String>) -> io::Result<String> {
    if args.len() == 1 {
        process_interactive_mode() // 無參數時進入互動模式
    } else {
        process_cli_mode() // 有參數時進入命令列模式
    }
}

/// 互動模式處理，收集使用者輸入並執行轉換
fn process_interactive_mode() -> io::Result<String> {
    println!("=== 歡迎使用互動模式 ===");
    println!("請按提示輸入參數（按 Enter 使用預設值）。");

    let input = get_input_path()?;
    let (is_compressed, password) = get_conversion_mode()?;
    let output = get_output_path()?;
    let (include, exclude) = get_file_patterns()?;
    let (compress, compression_level) = get_compression_options(is_compressed)?;

    info!("已收集互動模式參數: 輸入={}, 輸出={}, 模式={}, 壓縮={}", 
          input, output, if is_compressed { "壓縮" } else { "個別" }, compress);

    execute_conversion(input, output, is_compressed, compress, &include, &exclude, password, &compression_level)
}

fn get_input_path() -> io::Result<String> {
    Input::new()
        .with_prompt("請輸入檔案或目錄路徑（例如：./someFile.txt 或 ./anyDir）")
        .validate_with(|input: &String| -> Result<(), String> {
            if Path::new(input).exists() {
                Ok(())
            } else {
                Err(format!("路徑 '{}' 不存在", input))
            }
        })
        .interact_text()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
}

fn get_conversion_mode() -> io::Result<(bool, Option<String>)> {
    let is_compressed = Select::new()
        .with_prompt("選擇轉換模式（使用方向鍵選擇，按 Enter 確認）")
        .items(&["個別 - 為每個檔案生成單獨的 HTML", "壓縮 - 壓縮成單個 ZIP 嵌入 HTML"])
        .default(0)
        .interact()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))? == 1;

    let password = if is_compressed {
        get_password()?
    } else {
        None
    };

    Ok((is_compressed, password))
}

fn get_password() -> io::Result<Option<String>> {
    let pwd = Password::new()
        .with_prompt("輸入 ZIP 加密密碼（按 Enter 使用隨機生成密碼）")
        .allow_empty_password(true)
        .interact()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    if pwd.is_empty() {
        info!("未提供密碼，將隨機生成");
        Ok(None)
    } else {
        info!("已設置 ZIP 加密密碼");
        Ok(Some(pwd))
    }
}

fn get_output_path() -> io::Result<String> {
    Input::new()
        .with_prompt("輸入輸出目錄（例如：./output，預設為 output）")
        .default("output".to_string())
        .interact_text()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
}

fn get_file_patterns() -> io::Result<(Vec<String>, Vec<String>)> {
    let include = Input::new()
        .with_prompt("輸入包含模式（例如：*.txt,*.pdf，預設為 *）")
        .default("*".to_string())
        .interact_text()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    let exclude = Input::new()
        .with_prompt("輸入排除模式（例如：*.jpg,*.png，預設為空）")
        .default("".to_string())
        .interact_text()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    Ok((include, exclude))
}

fn get_compression_options(is_compressed: bool) -> io::Result<(bool, String)> {
    let compress = if !is_compressed {
        Confirm::new()
            .with_prompt("是否在個別模式下將檔案內容壓縮為 ZIP？")
            .default(true)
            .interact()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?
    } else {
        true
    };

    let compression_level = Select::new()
        .with_prompt("選擇壓縮級別")
        .items(&["stored", "deflated"])
        .default(1)
        .interact()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
        .map(|i| if i == 0 { "stored" } else { "deflated" })?
        .to_string();

    Ok((compress, compression_level))
}

fn execute_conversion(
    input: String,
    output: String,
    is_compressed: bool,
    compress: bool,
    include: &[String],
    exclude: &[String],
    password: Option<String>,
    compression_level: &str,
) -> io::Result<String> {
    let input_path = validate_input_path(&input)?;
    let (include_set, exclude_set) = create_regex_sets(include, exclude);

    if is_compressed {
        info!("開始壓縮轉換，輸入路徑：{}，輸出目錄：{}", input, output);
        process_compressed(input_path, &output, &include_set, &exclude_set, password.as_deref(), compression_level)?;
    } else {
        info!("開始個別轉換，輸入路徑：{}，輸出目錄：{}", input, output);
        process_individual(input_path, &output, &include_set, &exclude_set, compress, compression_level)?;
    }

    Ok(output)
}

/// 命令列模式處理，解析 CLI 參數並執行轉換
fn process_cli_mode() -> io::Result<String> {
    let cli = Cli::parse(); // 解析命令列參數
    let input_path = validate_input_path(&cli.input)?; // 驗證輸入路徑
    let (include_set, exclude_set) = create_regex_sets(
        &cli.include,
        &cli.exclude.as_deref().unwrap_or(&[]).to_vec()
    ); // 創建正則表達式集

    match cli.mode {
        Mode::Individual => {
            info!("開始個別轉換，輸入路徑：{}，輸出目錄：{}，包含模式：{:?}", 
                  cli.input, cli.output, cli.include);
            process_individual(
                input_path,
                &cli.output,
                &include_set,
                &exclude_set,
                cli.compress,
                &cli.compression_level
            )?;
        }
        Mode::Compressed => {
            info!("開始壓縮轉換，輸入路徑：{}，輸出目錄：{}，包含模式：{:?}", 
                  cli.input, cli.output, cli.include);
            process_compressed(
                input_path,
                &cli.output,
                &include_set,
                &exclude_set,
                cli.password.as_deref(),
                &cli.compression_level
            )?;
        }
    }

    Ok(cli.output)
}

/// 驗證輸入路徑是否存在
fn validate_input_path(input: &str) -> io::Result<&Path> {
    let input_path = Path::new(input);
    if !input_path.exists() {
        error!("輸入路徑不存在：{}", input);
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("輸入路徑 '{}' 不存在", input)
        ));
    }
    Ok(input_path)
}

/// 工具函數：創建正則表達式集，用於檔案過濾
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

/// 格式化檔案大小（以 KB 或 MB 顯示）
fn format_file_size(size: usize) -> String {
    if size < 1024 * 1024 {
        format!("{:.2} KB", size as f64 / 1024.0)
    } else {
        format!("{:.2} MB", size as f64 / (1024.0 * 1024.0))
    }
}

/// 創建 ZIP 緩衝區，將檔案內容壓縮（無加密）
fn create_zip_buffer(file_name: &str, data: &[u8], options: FileOptions<()>) -> io::Result<Vec<u8>> {
    let mut zip_buffer = Vec::new();
    let mut zip = ZipWriter::new(io::Cursor::new(&mut zip_buffer));
    zip.start_file(file_name, options)?;
    zip.write_all(data)?;
    zip.finish()?;
    Ok(zip_buffer)
}

/// 將資料壓縮為加密的 ZIP
fn create_encrypted_zip_buffer(data: &[u8], file_name: &str, password: &str) -> io::Result<Vec<u8>> {
    let mut zip_buffer = Vec::new();
    let mut zip = ZipWriter::new(io::Cursor::new(&mut zip_buffer));
    let options: FileOptions<zip::write::ExtendedFileOptions> = FileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .with_aes_encryption(AesMode::Aes256, password); // 使用 AES-256 加密
    zip.start_file(file_name, options)?;
    zip.write_all(data)?;
    zip.finish()?;
    info!("生成加密 ZIP，密碼：{}，大小：{} 位元組", password, zip_buffer.len());
    Ok(zip_buffer)
}

/// 個別轉換處理，將每個檔案轉為獨立的 HTML
fn process_individual(
    input_path: &Path,
    output_dir: &str,
    include_set: &RegexSet,
    exclude_set: &RegexSet,
    compress: bool,
    compression_level: &str
) -> io::Result<()> {
    fs::create_dir_all(output_dir)?; // 確保輸出目錄存在
    let mut files = Vec::new();
    collect_files(input_path, &mut files, include_set, exclude_set)?; // 收集符合條件的檔案
    let total_files = files.len();
    info!("正在處理 {} 個檔案", total_files);

    let pb = ProgressBar::new(total_files as u64); // 初始化進度條
    pb.set_style(ProgressStyle::default_bar()
        .template("{msg} [{bar:40}] {pos}/{len}")
        .unwrap());

    for (index, file_path) in files.iter().enumerate() {
        pb.set_message(format!("正在處理第 {}/{} 個檔案：{}",
                               index + 1, total_files, file_path.display()));
        convert_file_to_html(file_path, output_dir, compress, compression_level)?; // 轉換檔案為 HTML

        if index % 10 == 0 || index == total_files - 1 {
            pb.set_position((index + 1) as u64); // 更新進度條
        }
    }

    pb.finish_with_message("處理完成");
    Ok(())
}

/// 將單一檔案轉換為加密 ZIP 的 HTML 文件
fn convert_file_to_html(file_path: &Path, output_dir: &str, compress: bool, compression_level: &str) -> io::Result<()> {
    let mut file = File::open(file_path)?; // 開啟檔案
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?; // 讀取檔案內容
    let file_size = buffer.len(); // 記錄原始檔案大小
    info!("讀取檔案：{}，原始大小：{} 位元組", file_path.display(), file_size);

    let file_name = file_path.file_name().unwrap().to_string_lossy();
    if compress { format!("{}.zip", file_name) } else { file_name.to_string() };
    let download_zip_name = format!("{}_protected.zip", file_name); // 下載的 ZIP 檔案名稱
    let compression_note = if compress { "（內含 ZIP 檔案，請解壓後使用）" } else { "" }; // 繁體中文提示

    // 處理原始或壓縮後的資料
    let data_to_zip = if compress {
        let options: FileOptions<()> = if compression_level == "stored" {
            FileOptions::default().compression_method(CompressionMethod::Stored)
        } else {
            FileOptions::default().compression_method(CompressionMethod::Deflated)
        };
        let zip_buffer = create_zip_buffer(&file_name, &buffer, options)?; // 壓縮檔案
        info!("壓縮檔案至 ZIP：{}，壓縮後大小：{} 位元組", file_path.display(), zip_buffer.len());
        zip_buffer
    } else {
        info!("未壓縮檔案：{}，直接使用原始資料", file_path.display());
        buffer
    };

    // 生成隨機密碼（16 字元）
    let password = generate_random_password(16);
    info!("生成隨機密碼：{}", password);

    // 將資料壓縮為加密 ZIP
    let zip_buffer = create_encrypted_zip_buffer(&data_to_zip, &file_name, &password)?;
    let zip_base64 = general_purpose::STANDARD.encode(&zip_buffer);
    info!("生成加密 ZIP 的 Base64，總大小：{} 位元組", zip_base64.len());

    // 檢查 Base64 資料大小（建議 <1MB）
    const MAX_BASE64_SIZE: usize = 1_000_000; // 約 1MB
    if zip_base64.len() > MAX_BASE64_SIZE {
        warn!(
            "Base64 資料過大：{} 位元組，超過建議限制 {} 位元組，可能無法在某些瀏覽器下載：{}",
            zip_base64.len(), MAX_BASE64_SIZE, file_path.display()
        );
    }

    // 格式化檔案大小
    let file_size_str = format_file_size(file_size);

    // 生成 HTML 內容
    let html_content = HTML_TEMPLATE
        .replace("{{ZIP_BASE64}}", &zip_base64)
        .replace("{{FILE_NAME}}", &file_name)
        .replace("{{DOWNLOAD_ZIP_NAME}}", &download_zip_name)
        .replace("{{COMPRESSION_NOTE}}", compression_note)
        .replace("{{PASSWORD}}", &password)
        .replace("{{FILE_SIZE}}", &file_size_str);

    let output_path = Path::new(output_dir).join(format!("{}.html", file_name));
    fs::write(&output_path, &html_content)?; // 寫入 HTML 文件
    info!("生成 HTML 文件：{}，大小：{} 位元組", output_path.display(), html_content.len());
    Ok(())
}

/// 壓縮轉換處理，將多個檔案壓縮為單一 ZIP 並嵌入加密的 HTML
fn process_compressed(
    input_path: &Path,
    output_dir: &str,
    include_set: &RegexSet,
    exclude_set: &RegexSet,
    password: Option<&str>,
    compression_level: &str
) -> io::Result<()> {
    fs::create_dir_all(output_dir)?; // 確保輸出目錄存在
    let mut zip_buffer = Vec::new();
    let mut zip = ZipWriter::new(io::Cursor::new(&mut zip_buffer));
    let mut options: FileOptions<zip::write::ExtendedFileOptions> = if compression_level == "stored" {
        FileOptions::default().compression_method(CompressionMethod::Stored)
    } else {
        FileOptions::default().compression_method(CompressionMethod::Deflated)
    };

    // 將密碼提升到外部作用域，確保生命週期足夠長
    let pwd_owned: String;
    let zip_password = if let Some(pwd) = password {
        pwd.to_string()
    } else {
        pwd_owned = generate_random_password(16); // 隨機生成密碼
        pwd_owned
    };
    options = options.with_aes_encryption(AesMode::Aes256, &*zip_password);
    info!("使用 AES-256 加密 ZIP，密碼：{}", zip_password);

    let mut files = Vec::new();
    collect_files(input_path, &mut files, include_set, exclude_set)?; // 收集符合條件的檔案
    if files.is_empty() {
        error!("無有效檔案可壓縮");
        return Err(io::Error::new(io::ErrorKind::Other, "無有效檔案可壓縮"));
    }

    let total_files = files.len();
    let mut total_size = 0;
    info!("開始壓縮 {} 個檔案", total_files);

    let pb = ProgressBar::new(total_files as u64); // 初始化進度條
    pb.set_style(ProgressStyle::default_bar().template("{msg} [{bar:40}] {pos}/{len}").unwrap());
    for (index, file_path) in files.iter().enumerate() {
        if let Some(relative_path) = diff_paths(file_path, input_path.parent().unwrap_or(input_path)) {
            let relative_path_str = relative_path.to_string_lossy().replace("\\", "/").trim_start_matches("./").to_string();
            let mut file = File::open(file_path)?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;
            total_size += buffer.len(); // 累計檔案大小
            pb.set_message(format!("壓縮檔案 {}/{}：{}", index + 1, total_files, relative_path_str));
            zip.start_file(&relative_path_str, options.clone())?;
            zip.write_all(&buffer)?; // 寫入檔案內容
            if index % 10 == 0 || index == total_files - 1 {
                pb.set_position((index + 1) as u64); // 更新進度條
            }
        } else {
            warn!("跳過檔案：{}，無法計算相對路徑", file_path.display());
        }
    }
    pb.finish_with_message("壓縮完成");
    info!("壓縮完成，共處理 {} 個檔案，總大小：{} 位元組", total_files, total_size);
    zip.finish()?;

    // 生成隨機密碼（16 字元）用於額外 ZIP 層
    let extra_zip_password = generate_random_password(16);
    info!("生成額外 ZIP 隨機密碼：{}", extra_zip_password);

    // 將 ZIP 資料壓縮為加密 ZIP
    let file_name = input_path.file_name().unwrap_or(std::ffi::OsStr::new("archive")).to_string_lossy().to_string();
    let zip_buffer_extra = create_encrypted_zip_buffer(&zip_buffer, &format!("{}.zip", file_name), &extra_zip_password)?;
    let zip_base64 = general_purpose::STANDARD.encode(&zip_buffer_extra);
    info!("生成加密 ZIP 的 Base64，總大小：{} 位元組", zip_base64.len());

    // 檢查 Base64 資料大小
    const MAX_BASE64_SIZE: usize = 1_000_000; // 約 1MB
    if zip_base64.len() > MAX_BASE64_SIZE {
        warn!(
            "Base64 資料過大：{} 位元組，超過建議限制 {} 位元組，可能無法在某些瀏覽器下載：{}",
            zip_base64.len(), MAX_BASE64_SIZE, input_path.display()
        );
    }

    // 格式化檔案大小
    let file_size_str = format_file_size(total_size);

    // 生成 HTML 內容
    let file_name = format!("{}.zip", file_name);
    let download_zip_name = format!("{}_protected.zip", file_name);
    let html_content = HTML_TEMPLATE
        .replace("{{ZIP_BASE64}}", &zip_base64)
        .replace("{{FILE_NAME}}", &file_name)
        .replace("{{DOWNLOAD_ZIP_NAME}}", &download_zip_name)
        .replace("{{COMPRESSION_NOTE}}", "（ZIP 壓縮格式，請解壓後使用）")
        .replace("{{PASSWORD}}", &extra_zip_password)
        .replace("{{FILE_SIZE}}", &file_size_str);
    let output_path = Path::new(output_dir).join(format!("{}.html", file_name));
    fs::write(&output_path, &html_content)?; // 寫入 HTML 文件
    info!("生成 ZIP HTML 文件：{}，大小：{} 位元組", output_path.display(), html_content.len());
    Ok(())
}

/// 收集符合條件的檔案路徑
fn collect_files(path: &Path, files: &mut Vec<PathBuf>, include_set: &RegexSet, exclude_set: &RegexSet) -> io::Result<()> {
    let path_str = path.to_string_lossy().to_lowercase();
    if exclude_set.is_match(&path_str) {
        debug!("跳過檔案：{}，符合排除模式", path.display());
        return Ok(());
    }
    if include_set.is_match(&path_str) || include_set.is_match("*") {
        if path.is_file() {
            files.push(path.to_path_buf()); // 添加檔案路徑
        } else if path.is_dir() {
            for entry in fs::read_dir(path)? {
                collect_files(&entry?.path(), files, include_set, exclude_set)?; // 遞迴處理目錄
            }
        }
    }
    Ok(())
}