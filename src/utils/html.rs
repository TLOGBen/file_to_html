use std::fs;
use std::io::{self, BufWriter, Write};
use std::path::Path;
use base64::{engine::general_purpose, write::EncoderWriter};
use log::{info, warn};

const HTML_TEMPLATE: &str = include_str!("../../assets/template/html_template.html");

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

pub fn generate_instructions(layer: &str, has_password: bool) -> String {
    match (layer, has_password) {
        ("double", true) => "<p>請使用下載連結或複製 Base64 資料手動解碼為 ZIP 檔案，然後使用密碼解壓外層和內層 ZIP（使用相同密碼）。建議使用 7-Zip 或 WinRAR。</p>".to_string(),
        ("double", false) => "<p>請使用下載連結或複製 Base64 資料手動解碼為 ZIP 檔案，然後無需密碼解壓外層和內層 ZIP。建議使用 7-Zip 或 WinRAR。</p>".to_string(),
        ("single", true) => "<p>請使用下載連結或複製 Base64 資料手動解碼為 ZIP 檔案，然後使用密碼解壓 ZIP。建議使用 7-Zip 或 WinRAR。</p>".to_string(),
        ("single", false) => "<p>請使用下載連結或複製 Base64 資料手動解碼為 ZIP 檔案，然後無需密碼解壓 ZIP。建議使用 7-Zip 或 WinRAR。</p>".to_string(),
        _ => "<p>請使用下載連結或複製 Base64 資料手動解碼為檔案，無需解壓。</p>".to_string(),
    }
}

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

pub fn write_html_file(html_content: &str, output_dir: &str, file_name: &str) -> io::Result<()> {
    let output_path = Path::new(output_dir).join(format!("{}.html", file_name));
    let file = fs::File::create(&output_path)?;
    let mut writer = BufWriter::new(file);
    writer.write_all(html_content.as_bytes())?;
    writer.flush()?;
    Ok(())
}