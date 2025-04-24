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

    /// ZIP 加密密碼（僅在壓縮模式下有效）
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

/// HTML 模板，包含繁體中文文本和優化的樣式
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
        button {
            padding: 10px 20px;
            background-color: #007bff;
            color: white;
            border: none;
            border-radius: 5px;
            cursor: pointer;
        }
        button:hover {
            background-color: #0056b3;
        }
    </style>
</head>
<body>
    <h1>檔案下載</h1>
    <p>檔案名稱: {{FILE_NAME}}</p>
    <p>點擊下方按鈕下載檔案{{COMPRESSION_NOTE}}。</p>
    <button onclick="downloadFile()">下載檔案</button>
    <script>
        function downloadFile() {
            const base64Data = "{{BASE64_DATA}}";
            const fileName = "{{DOWNLOAD_FILE_NAME}}";
            const byteCharacters = atob(base64Data);
            const byteNumbers = new Array(byteCharacters.length);
            for (let i = 0; i < byteCharacters.length; i++) {
                byteNumbers[i] = byteCharacters.charCodeAt(i);
            }
            const byteArray = new Uint8Array(byteNumbers);
            const blob = new Blob([byteArray], { type: 'application/octet-stream' });
            const url = window.URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = fileName;
            document.body.appendChild(a);
            a.click();
            window.URL.revokeObjectURL(url);
            document.body.removeChild(a);
        }
    </script>
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
    let (input, output, is_compressed, password, compress, include, exclude, compression_level) = interactive_mode()?;
    let input_path = validate_input_path(&input)?; // 驗證輸入路徑
    let (include_set, exclude_set) = create_regex_sets(&include, &exclude); // 創建正則表達式集

    if is_compressed {
        info!("開始壓縮轉換，輸入路徑：{}，輸出目錄：{}", input, output);
        process_compressed(input_path, &output, &include_set, &exclude_set, password.as_deref(), &compression_level)?;
    } else {
        info!("開始個別轉換，輸入路徑：{}，輸出目錄：{}", input, output);
        process_individual(input_path, &output, &include_set, &exclude_set, compress, &compression_level)?;
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
    // 將包含模式的通配符轉換為正則表達式
    let include_patterns: Vec<_> = include.iter()
        .map(|p| p.replace(".", "\\.").replace("*", ".*"))
        .collect();
    // 將排除模式的通配符轉換為正則表達式
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

/// 互動模式配置收集，提示使用者輸入參數
fn interactive_mode() -> io::Result<(String, String, bool, Option<String>, bool, Vec<String>, Vec<String>, String)> {
    println!("=== 歡迎使用互動模式 ===");
    println!("請按提示輸入參數（按 Enter 使用預設值）。");

    // 收集輸入路徑
    let input = Input::new()
        .with_prompt("請輸入檔案或目錄路徑（例如：./myfile.txt 或 ./mydir）")
        .validate_with(|input: &String| -> Result<(), String> {
            if Path::new(input).exists() {
                Ok(())
            } else {
                Err(format!("路徑 '{}' 不存在", input))
            }
        })
        .interact_text()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    // 選擇轉換模式
    let is_compressed = Select::new()
        .with_prompt("選擇轉換模式（使用方向鍵選擇，按 Enter 確認）")
        .items(&["個別 - 為每個檔案生成單獨的 HTML", "壓縮 - 壓縮成單個 ZIP 嵌入 HTML"])
        .default(0)
        .interact()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))? == 1;

    // 如果是壓縮模式，收集密碼
    let password = if is_compressed {
        let pwd = Password::new()
            .with_prompt("輸入 ZIP 加密密碼（按 Enter 跳過不加密）")
            .allow_empty_password(true)
            .interact()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        if pwd.is_empty() {
            info!("未提供密碼，ZIP 將不會加密");
            None
        } else {
            info!("已設置 ZIP 加密密碼");
            Some(pwd)
        }
    } else {
        None
    };

    // 收集輸出路徑
    let output = Input::new()
        .with_prompt("輸入輸出目錄（例如：./output，預設為 output）")
        .default("output".to_string())
        .interact_text()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    // 收集包含模式
    let include = Input::new()
        .with_prompt("輸入包含模式（例如：*.txt,*.pdf，預設為 *）")
        .default("*".to_string())
        .interact_text()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?
        .split(',')
        .map(|s| s.trim().to_string())
        .collect::<Vec<String>>();

    // 收集排除模式
    let exclude = Input::new()
        .with_prompt("輸入排除模式（例如：*.jpg,*.png，預設為空）")
        .default("".to_string())
        .interact_text()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<String>>();

    // 選擇壓縮選項（僅對個別模式有效）
    let compress = if !is_compressed {
        Confirm::new()
            .with_prompt("是否在個別模式下將檔案內容壓縮為 ZIP？")
            .default(true)
            .interact()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?
    } else {
        true
    };

    // 選擇壓縮級別
    let compression_level = Select::new()
        .with_prompt("選擇壓縮級別")
        .items(&["stored", "deflated"])
        .default(1)
        .interact()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
        .map(|i| if i == 0 { "stored" } else { "deflated" })?;

    info!("已收集互動模式參數: 輸入={}, 輸出={}, 模式={}, 壓縮={}", 
          input, output, if is_compressed { "壓縮" } else { "個別" }, compress);

    Ok((input, output, is_compressed, password, compress, include, exclude, compression_level.to_string()))
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

/// 將單一檔案轉換為 HTML 文件
fn convert_file_to_html(file_path: &Path, output_dir: &str, compress: bool, compression_level: &str) -> io::Result<()> {
    let mut file = File::open(file_path)?; // 開啟檔案
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?; // 讀取檔案內容
    info!("讀取檔案：{}，原始大小：{} 位元組", file_path.display(), buffer.len());

    let file_name = file_path.file_name().unwrap().to_string_lossy();
    let download_file_name = if compress { format!("{}.zip", file_name) } else { file_name.to_string() };
    let compression_note = if compress { "（ZIP 壓縮格式，請解壓後使用）" } else { "" }; // 繁體中文提示

    let base64_data = if compress {
        let options: FileOptions<()> = if compression_level == "stored" {
            FileOptions::default().compression_method(CompressionMethod::Stored)
        } else {
            FileOptions::default().compression_method(CompressionMethod::Deflated)
        };
        let zip_buffer = create_zip_buffer(&file_name, &buffer, options)?; // 壓縮檔案
        info!("壓縮檔案至 ZIP：{}，壓縮後大小：{} 位元組", file_path.display(), zip_buffer.len());
        general_purpose::STANDARD.encode(&zip_buffer)
    } else {
        info!("未壓縮檔案：{}，直接嵌入原始資料", file_path.display());
        general_purpose::STANDARD.encode(&buffer)
    };

    let html_content = HTML_TEMPLATE
        .replace("{{BASE64_DATA}}", &base64_data)
        .replace("{{FILE_NAME}}", &file_name)
        .replace("{{DOWNLOAD_FILE_NAME}}", &download_file_name)
        .replace("{{COMPRESSION_NOTE}}", compression_note);

    let output_path = Path::new(output_dir).join(format!("{}.html", file_name));
    fs::write(&output_path, &html_content)?; // 寫入 HTML 文件
    info!("生成 HTML 文件：{}，大小：{} 位元組", output_path.display(), html_content.len());
    Ok(())
}

/// 創建 ZIP 緩衝區，將檔案內容壓縮
fn create_zip_buffer(file_name: &str, data: &[u8], options: FileOptions<()>) -> io::Result<Vec<u8>> {
    let mut zip_buffer = Vec::new();
    let mut zip = ZipWriter::new(std::io::Cursor::new(&mut zip_buffer));
    zip.start_file(file_name, options)?; // 開始寫入檔案
    zip.write_all(data)?; // 寫入數據
    zip.finish()?; // 完成壓縮
    Ok(zip_buffer)
}

/// 壓縮轉換處理，將多個檔案壓縮為單一 ZIP 並嵌入 HTML
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
    let mut zip = ZipWriter::new(std::io::Cursor::new(&mut zip_buffer));
    let mut options: FileOptions<()> = if compression_level == "stored" {
        FileOptions::default().compression_method(CompressionMethod::Stored)
    } else {
        FileOptions::default().compression_method(CompressionMethod::Deflated)
    };
    if let Some(pwd) = password {
        options = options.with_aes_encryption(AesMode::Aes256, pwd); // 設定 AES-256 加密
        info!("使用 AES-256 加密 ZIP，已設定密碼");
    } else {
        info!("未設定密碼，ZIP 不會加密");
    }

    let mut files = Vec::new();
    collect_files(input_path, &mut files, include_set, exclude_set)?; // 收集符合條件的檔案
    if files.is_empty() {
        error!("無有效檔案可壓縮");
        return Err(io::Error::new(io::ErrorKind::Other, "無有效檔案可壓縮"));
    }

    let total_files = files.len();
    info!("開始壓縮 {} 個檔案", total_files);

    let pb = ProgressBar::new(total_files as u64); // 初始化進度條
    pb.set_style(ProgressStyle::default_bar().template("{msg} [{bar:40}] {pos}/{len}").unwrap());
    for (index, file_path) in files.iter().enumerate() {
        if let Some(relative_path) = diff_paths(file_path, input_path.parent().unwrap_or(input_path)) {
            let relative_path_str = relative_path.to_string_lossy().replace("\\", "/").trim_start_matches("./").to_string();
            let mut file = File::open(file_path)?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;
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
    info!("壓縮完成，共處理 {} 個檔案", total_files);
    zip.finish()?;

    let base64_data = general_purpose::STANDARD.encode(&zip_buffer);
    let file_name = input_path.file_name().unwrap_or(std::ffi::OsStr::new("archive")).to_string_lossy().to_string() + ".zip";
    let html_content = HTML_TEMPLATE
        .replace("{{BASE64_DATA}}", &base64_data)
        .replace("{{FILE_NAME}}", &file_name)
        .replace("{{DOWNLOAD_FILE_NAME}}", &file_name)
        .replace("{{COMPRESSION_NOTE}}", "（ZIP 壓縮格式，請解壓後使用）"); // 繁體中文提示
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