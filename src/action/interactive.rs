use dialoguer::{Input, Password, Select, Confirm};
use std::io;
use std::path::Path;

use crate::config::config::{PasswordMode};
use crate::utils::utils::setup_logging;
use crate::config::ports::{AppConfig, ConfigPort};
use crate::facade::conversion_facade::ConversionFacade;
use crate::facade::traits::i_conversion::ConversionFacadeTrait;
use crate::models::conversion::ConversionInput;
use crate::service::config_service::{DefaultConfigAdapter};
use crate::service::file::FileService;
use crate::service::html::HtmlService;
use crate::service::zip::ZipService;

pub fn process_interactive_mode() -> io::Result<String> {
    println!("=== 歡迎使用互動模式 ===");
    let use_default_config = get_default_config_option()?;
    let input = get_input_path()?;
    let output = get_output_path()?;

    let config_port: Box<dyn ConfigPort> = if use_default_config {
        println!("使用預設配置：壓縮模式，單層壓縮，隨機密碼，AES256 加密");
        Box::new(DefaultConfigAdapter::new(input.clone(), output.clone()))
    } else {
        Box::new(InteractiveConfigAdapter::new(input.clone(), output.clone()))
    };

    let facade: Box<dyn ConversionFacadeTrait> = Box::new(ConversionFacade::new(
        config_port,
        Box::new(FileService::new()),
        Box::new(ZipService::new()),
        Box::new(HtmlService::new()),
    ));

    let conversion_input = ConversionInput {
        input_path: Path::new(&input).to_path_buf(),
        output_dir: output.clone(),
        is_compressed: true,
        compress: true,
        include: vec!["*".to_string()],
        exclude: None,
        password_mode: crate::config::config::PasswordMode::Random,
        display_password: true,
        layer: "single".to_string(),
        encryption_method: "aes256".to_string(),
        no_progress: false,
        max_size: None,
    };

    let output = facade.execute_conversion(conversion_input)?;
    println!("實際使用的配置：{:#?}", output);
    Ok(output.output_path)
}

pub fn get_default_config_option() -> io::Result<bool> {
    Confirm::new()
        .with_prompt("是否使用預設配置？（壓縮模式、單層壓縮、隨機密碼等，僅需指定輸入和輸出路徑）")
        .default(true)
        .interact()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("預設配置選擇失敗: {}", e)))
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

pub fn get_output_path() -> io::Result<String> {
    Input::new()
        .with_prompt("輸入輸出目錄（例如：./output，預設為 output）")
        .default("output".to_string())
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

pub fn get_compression_options(is_compressed: bool) -> io::Result<bool> {
    let compress = if !is_compressed {
        Confirm::new()
            .with_prompt("是否在個別模式下將檔案壓縮為 ZIP？")
            .default(true)
            .interact()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("壓縮選項輸入失敗: {}", e)))?
    } else {
        true
    };
    Ok(compress)
}

pub fn get_no_progress_option() -> io::Result<bool> {
    Ok(false)
}

pub fn get_max_size_option() -> io::Result<Option<f64>> {
    Ok(None)
}

pub fn get_log_level_option() -> io::Result<String> {
    Ok("info".to_string())
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

// 交互配置適配器
pub struct InteractiveConfigAdapter {
    input: String,
    output: String,
}

impl InteractiveConfigAdapter {
    pub fn new(input: String, output: String) -> Self {
        InteractiveConfigAdapter { input, output }
    }
}

impl ConfigPort for InteractiveConfigAdapter {
    fn get_config(&self) -> io::Result<AppConfig> {
        let (is_compressed, password_mode, display_password, layer, encryption_method) = get_conversion_mode_and_password()?;
        let (include, exclude) = get_file_patterns()?;
        let compress = get_compression_options(is_compressed)?;
        let no_progress = get_no_progress_option()?;
        let max_size = get_max_size_option()?;
        let log_level = get_log_level_option()?;

        setup_logging(&log_level)?;

        Ok(AppConfig {
            input: self.input.clone(),
            output: self.output.clone(),
            is_compressed,
            compress,
            include,
            exclude,
            password_mode,
            display_password,
            layer,
            encryption_method,
            no_progress,
            max_size,
        })
    }
}