use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use base64::{engine::general_purpose, Engine as _};
use clap::{Parser, ValueEnum};
use dialoguer::{Input, Password, Select, Confirm};
use pathdiff::diff_paths;
use env_logger;
use zip::write::{FileOptions, ZipWriter};
use zip::CompressionMethod;
use indicatif::{ProgressBar, ProgressStyle};
use regex::RegexSet;

/// 命令列介面參數結構
#[derive(Parser)]
#[command(
    name = "file_to_html",
    about = "將檔案或目錄轉換為嵌入式HTML格式"
)]
struct Cli {
    /// 輸入檔案或目錄路徑
    input: String,
    
    /// 輸出目錄
    #[arg(short, long, default_value = "output")]
    output: String,
    
    /// 轉換模式：個別檔案或壓縮模式
    #[arg(long, default_value = "individual")]
    mode: Mode,
    
    /// 僅處理指定副檔名的檔案（使用逗號分隔）
    #[arg(long, default_value = "*", value_delimiter = ',')]
    include: Vec<String>,
    
    /// 排除指定副檔名的檔案（使用逗號分隔）
    #[arg(long, value_delimiter = ',')]
    exclude: Option<Vec<String>>,
    
    /// 是否在個別模式下壓縮檔案內容為ZIP格式
    #[arg(long, default_value_t = true)]
    compress: bool,
    
    /// ZIP加密密碼（僅壓縮模式有效）
    #[arg(long)]
    password: Option<String>,
}

/// 轉換模式枚舉
#[derive(Clone, ValueEnum)]
enum Mode {
    /// 個別檔案模式 - 每個檔案生成單獨的HTML
    Individual,
    /// 壓縮模式 - 將所有檔案壓縮為單一ZIP並嵌入HTML
    Compressed,
}

/// HTML模板，用於生成下載頁面
const HTML_TEMPLATE: &str = r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>檔案下載</title>
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

/// 主函數 - 程式入口點
fn main() -> io::Result<()> {
    // 初始化環境日誌
    env_logger::init();
    
    // 獲取命令列參數
    let args: Vec<String> = std::env::args().collect();
    
    // 根據參數數量決定使用互動模式或命令列模式
    let output_dir = if args.len() == 1 {
        // 互動模式 - 無命令列參數時啟動
        let (input, output, is_compressed, password, compress) = interactive_mode()?;
        let input_path = Path::new(&input);
        
        // 檢查輸入路徑是否存在
        if !input_path.exists() {
            println!("錯誤：輸入路徑 '{}' 不存在", input);
            return Err(io::Error::new(io::ErrorKind::NotFound, "輸入路徑不存在"));
        }
        
        // 根據選擇的模式處理檔案
        if is_compressed {
            process_compressed(input_path, &output, &vec!["*".to_string()], &[], password.as_deref())?;
        } else {
            process_individual(input_path, &output, &vec!["*".to_string()], &[], compress)?;
        }
        output
    } else {
        // 命令列模式 - 解析命令列參數
        let cli = Cli::parse();
        let input_path = Path::new(&cli.input);
        
        // 檢查輸入路徑是否存在
        if !input_path.exists() {
            println!("錯誤：輸入路徑 '{}' 不存在", cli.input);
            return Err(io::Error::new(io::ErrorKind::NotFound, "輸入路徑不存在"));
        }
        
        // 處理排除項目
        let exclude = cli.exclude.as_deref().unwrap_or(&[]);
        
        // 根據指定的模式處理檔案
        match cli.mode {
            Mode::Individual => process_individual(input_path, &cli.output, &cli.include, exclude, cli.compress)?,
            Mode::Compressed => process_compressed(input_path, &cli.output, &cli.include, exclude, cli.password.as_deref())?,
        }
        cli.output
    };
    
    // 顯示完成訊息
    println!("轉換完成！輸出檔案位於：{}", output_dir);
    Ok(())
}

/// 互動模式 - 引導使用者輸入參數
fn interactive_mode() -> io::Result<(String, String, bool, Option<String>, bool)> {
    println!("=== 歡迎使用互動模式 ===");
    
    // 輸入檔案或目錄路徑
    let input = Input::new()
        .with_prompt("請輸入檔案或目錄路徑（例如：./myfile.txt 或 ./mydir）")
        .validate_with(|input: &String| -> Result<(), String> {
            if Path::new(input).exists() { Ok(()) } else { Err(format!("路徑 '{}' 不存在", input)) }
        })
        .interact_text()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    // 選擇轉換模式
    let is_compressed = Select::new()
        .with_prompt("請選擇轉換模式")
        .items(&["個別模式 - 為每個檔案生成單獨的HTML", "壓縮模式 - 將所有檔案壓縮為單一ZIP並嵌入HTML"])
        .default(0)
        .interact()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))? == 1;

    // 如果選擇壓縮模式，可以設定密碼
    let password = if is_compressed {
        let pwd = Password::new()
            .with_prompt("請輸入ZIP加密密碼（直接按Enter跳過）")
            .allow_empty_password(true)
            .interact()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        if pwd.is_empty() { None } else { Some(pwd) }
    } else {
        None
    };

    // 輸入輸出目錄
    let output = Input::new()
        .with_prompt("請輸入輸出目錄（預設為output）")
        .default("output".to_string())
        .interact_text()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    // 如果是個別模式，詢問是否壓縮檔案內容
    let compress = if !is_compressed {
        Confirm::new()
            .with_prompt("是否在個別模式下將檔案內容壓縮為ZIP格式？")
            .default(true)
            .interact()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
    } else {
        true
    };

    Ok((input, output, is_compressed, password, compress))
}

/// 處理個別檔案模式 - 為每個檔案生成單獨的HTML
fn process_individual(input_path: &Path, output_dir: &str, include: &[String], exclude: &[String], compress: bool) -> io::Result<()> {
    // 創建輸出目錄
    fs::create_dir_all(output_dir)?;
    
    // 處理單個檔案
    if input_path.is_file() && matches_extension(input_path, include, exclude) {
        convert_file_to_html(input_path, output_dir, compress)?;
    } 
    // 處理目錄
    else if input_path.is_dir() {
        for entry in fs::read_dir(input_path)? {
            let path = entry?.path();
            if path.is_file() && matches_extension(&path, include, exclude) {
                convert_file_to_html(&path, output_dir, compress)?;
            } else if path.is_dir() {
                // 遞迴處理子目錄
                process_individual(&path, output_dir, include, exclude, compress)?;
            }
        }
    }
    Ok(())
}

/// 將單個檔案轉換為HTML格式
fn convert_file_to_html(file_path: &Path, output_dir: &str, compress: bool) -> io::Result<()> {
    // 讀取檔案內容
    let mut file = File::open(file_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    
    // 獲取檔案名稱
    let file_name = file_path.file_name().unwrap().to_string_lossy();
    let download_file_name = if compress { format!("{}.zip", file_name) } else { file_name.to_string() };
    let compression_note = if compress { "（ZIP壓縮格式，請解壓後使用）" } else { "" };

    // 將檔案內容轉換為base64
    let base64_data = if compress {
        // 如果需要壓縮，先將檔案內容壓縮為ZIP
        let mut zip_buffer = Vec::new();
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut zip_buffer));
        
        // 定義ZIP檔案選項，明確指定類型參數
        let options = FileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(0o755) as FileOptions<'_, ()>;

        // 添加檔案到ZIP
        zip.start_file(file_name.as_ref(), options)?;
        zip.write_all(&buffer)?;
        zip.finish()?;
        
        // 將ZIP轉換為base64
        general_purpose::STANDARD.encode(&zip_buffer)
    } else {
        // 直接將檔案內容轉換為base64
        general_purpose::STANDARD.encode(&buffer)
    };

    // 生成HTML內容
    let html_content = HTML_TEMPLATE
        .replace("{{BASE64_DATA}}", &base64_data)
        .replace("{{FILE_NAME}}", &file_name)
        .replace("{{DOWNLOAD_FILE_NAME}}", &download_file_name)
        .replace("{{COMPRESSION_NOTE}}", compression_note);

    // 寫入HTML檔案
    let output_path = Path::new(output_dir).join(format!("{}.html", file_name));
    fs::write(&output_path, &html_content)?;
    Ok(())
}

/// 處理壓縮模式 - 將所有檔案壓縮為單一ZIP並嵌入HTML
fn process_compressed(input_path: &Path, output_dir: &str, include: &[String], exclude: &[String], password: Option<&str>) -> io::Result<()> {
    // 創建輸出目錄
    fs::create_dir_all(output_dir)?;
    
    // 準備ZIP緩衝區
    let mut zip_buffer = Vec::new();
    let mut zip = ZipWriter::new(std::io::Cursor::new(&mut zip_buffer));
    
    // 定義ZIP檔案選項，明確指定類型參數
    let options: FileOptions<'static, ()> = FileOptions::default()
        .compression_method(CompressionMethod::Deflated);

    // 收集所有符合條件的檔案
    let mut files = Vec::new();
    collect_files(input_path, &mut files, include, exclude)?;
    
    // 檢查是否有檔案符合條件
    if files.is_empty() {
        println!("錯誤：沒有檔案符合包含/排除條件");
        return Err(io::Error::new(io::ErrorKind::Other, "沒有有效的檔案可以壓縮"));
    }

    // 創建進度條
    let pb = ProgressBar::new(files.len() as u64);
    pb.set_style(ProgressStyle::default_bar().template("{msg} [{bar:40}] {pos}/{len}").unwrap());

    // 處理每個檔案
    for (index, file_path) in files.iter().enumerate() {
        if let Some(relative_path) = diff_paths(file_path, input_path.parent().unwrap_or(input_path)) {
            let relative_path_str = relative_path.to_string_lossy().replace("\\", "/").trim_start_matches("./").to_string();
            
            // 讀取檔案內容
            let mut file = File::open(file_path)?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;
            
            // 更新進度條
            pb.set_message(format!("正在壓縮檔案 {}/{}: {}", index + 1, files.len(), relative_path_str));
            pb.inc(1);
            
            // 添加檔案到ZIP
            zip.start_file(&relative_path_str, options.clone())?;
            zip.write_all(&buffer)?;
        }
    }

    // 完成進度條
    pb.finish_with_message("壓縮完成");
    
    // 完成ZIP
    zip.finish()?;
    
    // 將ZIP轉換為base64
    let base64_data = general_purpose::STANDARD.encode(&zip_buffer);
    
    // 獲取輸出檔案名稱
    let file_name = input_path.file_name().unwrap_or(std::ffi::OsStr::new("archive")).to_string_lossy().to_string() + ".zip";
    
    // 生成HTML內容
    let html_content = HTML_TEMPLATE
        .replace("{{BASE64_DATA}}", &base64_data)
        .replace("{{FILE_NAME}}", &file_name)
        .replace("{{DOWNLOAD_FILE_NAME}}", &file_name)
        .replace("{{COMPRESSION_NOTE}}", "（ZIP壓縮格式，請解壓後使用）");
    
    // 寫入HTML檔案
    let output_path = Path::new(output_dir).join(format!("{}.html", file_name));
    fs::write(&output_path, &html_content)?;
    Ok(())
}

/// 收集所有符合條件的檔案
fn collect_files(path: &Path, files: &mut Vec<PathBuf>, include: &[String], exclude: &[String]) -> io::Result<()> {
    if path.is_file() && matches_extension(path, include, exclude) {
        // 如果是符合條件的檔案，添加到集合中
        files.push(path.to_path_buf());
    } else if path.is_dir() {
        // 如果是目錄，遞迴處理
        for entry in fs::read_dir(path)? {
            collect_files(&entry?.path(), files, include, exclude)?;
        }
    }
    Ok(())
}

/// 檢查檔案是否符合副檔名條件
fn matches_extension(path: &Path, include: &[String], exclude: &[String]) -> bool {
    let path_str = path.to_string_lossy().to_lowercase();
    
    // 先檢查是否被排除
    if is_excluded(&path_str, path, exclude) {
        return false;
    }
    
    // 再檢查是否被包含
    is_included(&path_str, path, include)
}

/// 檢查檔案是否被排除
fn is_excluded(path_str: &str, path: &Path, exclude: &[String]) -> bool {
    // 將排除模式轉換為正則表達式
    let exclude_patterns: Vec<_> = exclude.iter().map(|p| p.replace(".", "\\.").replace("*", ".*")).collect();
    
    // 嘗試使用正則表達式集合匹配
    if let Ok(exclude_set) = RegexSet::new(&exclude_patterns) {
        if exclude_set.is_match(path_str) {
            return true;
        }
    } else {
        // 如果正則表達式無效，嘗試簡單的副檔名匹配
        for pattern in exclude {
            if try_simple_extension_match(path, pattern) {
                return true;
            }
        }
    }
    false
}

/// 檢查檔案是否被包含
fn is_included(path_str: &str, path: &Path, include: &[String]) -> bool {
    // 如果包含 "*"，表示包含所有檔案
    if include.contains(&"*".to_string()) {
        return true;
    }
    
    // 將包含模式轉換為正則表達式
    let include_patterns: Vec<_> = include.iter().map(|p| p.replace(".", "\\.").replace("*", ".*")).collect();
    
    // 嘗試使用正則表達式集合匹配
    if let Ok(include_set) = RegexSet::new(&include_patterns) {
        if include_set.is_match(path_str) {
            return true;
        }
    } else {
        // 如果正則表達式無效，嘗試簡單的副檔名匹配
        for pattern in include {
            if try_simple_extension_match(path, pattern) {
                return true;
            }
        }
    }
    false
}

/// 簡單的副檔名匹配
fn try_simple_extension_match(path: &Path, pattern: &str) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case(pattern))
        .unwrap_or(false)
}