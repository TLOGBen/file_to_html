use std::fs;
use std::io::{self, BufWriter, Write};
use std::path::Path;
use base64::{engine::general_purpose, write::EncoderWriter};
use log::{info, warn};
use crate::models::html::{HtmlGenerateInput, HtmlGenerateOutput};
use crate::service::traits::i_service::HtmlServiceTrait;
use crate::utils::utils::{format_file_size, get_file_name};

const HTML_TEMPLATE: &str = include_str!("../../assets/template/html_template.html");

/// HTML 服務，負責生成 HTML 檔案並實現 HtmlServiceTrait
pub struct HtmlService;

impl HtmlService {
    /// 創建新的 HtmlService 實例
    pub fn new() -> Self {
        HtmlService
    }
}

impl HtmlServiceTrait for HtmlService {
    /// 根據輸入生成 HTML 檔案
    /// # 參數
    /// - input: HTML 生成的輸入參數，包含 ZIP 數據、路徑、密碼等
    /// # 回傳
    /// - 成功時返回生成的 HTML 檔案路徑，失敗時返回 IO 錯誤
    fn generate_html(&self, input: HtmlGenerateInput) -> io::Result<HtmlGenerateOutput> {
        // 取得檔案名稱與下載名稱
        let (file_name, download_zip_name) = get_file_name(&input.input_path, &input.layer);

        // 將 ZIP 數據編碼為 Base64
        let zip_base64 = encode_to_base64(&input.zip_buffer, &input.input_path)?;
        info!("生成 Base64 數據，總大小：{} 位元組", zip_base64.len());

        // 生成使用說明
        let instructions = generate_instructions(&input.layer, input.password.is_some());

        // 處理密碼顯示邏輯
        let (password_info, password_display) = handle_password_display(
            input.password.as_deref(),
            input.display_password,
            &file_name,
            &input.output_dir,
        )?;

        // 格式化檔案大小
        let file_size_str = format_file_size(input.total_size);

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
        write_html_file(&html_content, &input.output_dir, &file_name)?;
        info!(
            "生成 HTML 檔案：{}/{}.html，大小：{} 位元組",
            input.output_dir,
            file_name,
            html_content.len()
        );

        Ok(HtmlGenerateOutput {
            html_file_path: format!("{}/{}.html", input.output_dir, file_name),
        })
    }
}

// 以下是原有的 HTML 生成相關函數，保持不變

/// 生成 HTML 內容，替換模板中的佔位符
pub fn generate_html_content(
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

/// 生成使用說明，根據層數和是否有密碼
pub fn generate_instructions(layer: &str, has_password: bool) -> String {
    match (layer, has_password) {
        ("double", true) => "<p>請使用下載連結或複製 Base64 資料手動解碼為 ZIP 檔案，然後使用密碼解壓外層和內層 ZIP（使用相同密碼）。建議使用 7-Zip 或 WinRAR。</p>".to_string(),
        ("double", false) => "<p>請使用下載連結或複製 Base64 資料手動解碼為 ZIP 檔案，然後無需密碼解壓外層和內層 ZIP。建議使用 7-Zip 或 WinRAR。</p>".to_string(),
        ("single", true) => "<p>請使用下載連結或複製 Base64 資料手動解碼為 ZIP 檔案，然後使用密碼解壓 ZIP。建議使用 7-Zip 或 WinRAR。</p>".to_string(),
        ("single", false) => "<p>請使用下載連結或複製 Base64 資料手動解碼為 ZIP 檔案，然後無需密碼解壓 ZIP。建議使用 7-Zip 或 WinRAR。</p>".to_string(),
        _ => "<p>請使用下載連結或複製 Base64 資料手動解碼為檔案，無需解壓。</p>".to_string(),
    }
}

/// 處理密碼顯示邏輯，決定是否將密碼嵌入 HTML 或儲存到檔案
pub fn handle_password_display(
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
            let path = Path::new(output_dir).join(&key_file);
            let mut file = BufWriter::new(fs::File::create(&path)?);
            file.write_all(pwd.as_bytes())?;
            file.flush()?;
            info!("密碼已儲存至：{}", key_file);
            Ok((format!("{}.html.key 檔案", file_name), "".to_string()))
        }
    } else {
        Ok(("無需密碼".to_string(), "".to_string()))
    }
}

/// 將數據編碼為 Base64 格式
pub fn encode_to_base64(data: &[u8], file_path: &Path) -> io::Result<String> {
    let mut base64_buffer = Vec::new();
    {
        let mut encoder = EncoderWriter::new(&mut base64_buffer, &general_purpose::STANDARD);
        encoder.write_all(data)?;
        encoder.flush()?;
    }
    let zip_base64 = String::from_utf8(base64_buffer)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    const MAX_BASE64_SIZE: usize = 1_000_000;
    if zip_base64.len() > MAX_BASE64_SIZE {
        warn!(
            "Base64 資料過大：{} 位元組，超過建議限制 {} 位元組，可能影響顯示或下載：{}",
            zip_base64.len(), MAX_BASE64_SIZE, file_path.display()
        );
    }
    Ok(zip_base64)
}

/// 將 HTML 內容寫入檔案
pub fn write_html_file(html_content: &str, output_dir: &str, file_name: &str) -> io::Result<()> {
    let output_path = Path::new(output_dir).join(format!("{}.html", file_name));
    let file = fs::File::create(&output_path)?;
    let mut writer = BufWriter::new(file);
    writer.write_all(html_content.as_bytes())?;
    writer.flush()?;
    Ok(())
}