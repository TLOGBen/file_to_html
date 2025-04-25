use std::fs;
use std::io;
use std::io::Write;
use std::path::Path;
use regex::RegexSet;
use log::{info, error, warn};
use zip::write::{SimpleFileOptions, ZipWriter};
use zip::{CompressionMethod, AesMode};
use crate::config::PasswordMode;
use crate::config::validate_input_path;
use crate::file::{collect_files, collect_and_measure_files, read_file_content};
use crate::zip::{create_zip, create_inner_zip, create_zip_buffer, compress_file_content};
use crate::html::{generate_html_content, generate_instructions, handle_password_display, encode_to_base64, write_html_file};
use crate::utils::{create_progress_bar, format_file_size, manage_progress, get_file_name};

pub fn execute_conversion(
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
    let (include_set, exclude_set) = crate::utils::create_regex_sets(include, exclude);

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

fn compress_single_file(
    data: &[u8],
    file_name: &str,
    compress: bool,
    compression_level: &str,
    password: Option<&str>,
    aes_mode: AesMode,
    layer: &str,
) -> io::Result<Vec<u8>> {
    if layer == "single" {
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored);
        if let Some(pwd) = password {
            let mut zip_buffer = Vec::new();
            let mut zip = ZipWriter::new(std::io::Cursor::new(&mut zip_buffer));
            let encrypt_options = SimpleFileOptions::default()
                .compression_method(CompressionMethod::Stored)
                .with_aes_encryption(aes_mode, pwd);
            zip.start_file(file_name.to_string(), encrypt_options)?;
            zip.write_all(data)?;
            zip.finish()?;
            info!("生成單層加密 ZIP，密碼：{}，大小：{} 位元組", pwd, zip_buffer.len());
            Ok(zip_buffer)
        } else {
            let zip_buffer = if compress {
                create_zip_buffer(file_name, data, options)?
            } else {
                data.to_vec()
            };
            info!("生成單層無密碼 ZIP，大小：{} 位元組", zip_buffer.len());
            Ok(zip_buffer)
        }
    } else {
        let inner_data = if compress && layer != "none" {
            let zip_buffer = compress_file_content(data, file_name, compression_level, password, aes_mode)?;
            info!("壓縮檔案至內層 ZIP：{}，壓縮後大小：{} 位元組", file_name, zip_buffer.len());
            if let Some(pwd) = password {
                info!("內層 ZIP 使用密碼：{}", pwd);
            }
            zip_buffer
        } else {
            info!("未壓縮檔案：{}，直接使用原始資料", file_name);
            data.to_vec()
        };
        Ok(create_zip(&inner_data, file_name, layer, password, aes_mode)?)
    }
}

pub fn convert_file_to_html(
    file_path: &Path,
    output_dir: &str,
    compress: bool,
    compression_level: &str,
    password: Option<String>,
    display_password: bool,
    layer: &str,
    encryption_method: &str,
) -> io::Result<()> {
    let (file_name, download_zip_name) = get_file_name(file_path, layer);
    let (data, file_size) = read_file_content(file_path)?;
    info!("讀取檔案：{}，原始大小：{} 位元組", file_path.display(), file_size);

    let aes_mode = match encryption_method {
        "aes128" => AesMode::Aes128,
        "aes192" => AesMode::Aes192,
        "aes256" => AesMode::Aes256,
        _ => AesMode::Aes256,
    };

    let final_zip_buffer = compress_single_file(
        &data,
        &file_name,
        compress,
        compression_level,
        password.as_deref(),
        aes_mode,
        layer,
    )?;

    let zip_base64 = encode_to_base64(&final_zip_buffer, file_path)?;
    info!("生成最終資料的 Base64，總大小：{} 位元組", zip_base64.len());

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

    let file_size_str = format_file_size(file_size);
    let html_content = generate_html_content(
        &zip_base64,
        &file_name,
        &download_zip_name,
        &instructions,
        &file_size_str,
        &password_info,
        &password_display,
    );

    write_html_file(&html_content, output_dir, &file_name)?;
    info!("生成 HTML 文件：{}/{}.html，大小：{} 位元組", output_dir, file_name, html_content.len());
    Ok(())
}

pub fn process_individual(
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
    collect_files(input_path, &mut files, include_set, exclude_set, max_size, no_progress)?;
    let total_files = files.len();
    info!("正在處理 {} 個檔案", total_files);

    if total_files == 0 {
        warn!("無符合條件的檔案可處理");
        return Ok(());
    }

    let password = crate::utils::generate_password(&password_mode, preset_password)?;
    if let Some(ref pwd) = password {
        info!("使用密碼：{}", pwd);
    } else {
        info!("選擇無密碼模式，ZIP 不加密");
    }

    let pm = create_progress_bar(total_files as u64, no_progress);
    let start = std::time::Instant::now();
    for (i, file_path) in files.iter().enumerate() {
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
            if (i + 1) % 1000 == 0 {
                manage_progress(&pm, (i + 1) as u64, None, start, no_progress, "處理檔案");
            }
        }
    }
    if total_files % 1000 != 0 {
        manage_progress(&pm, total_files as u64, None, start, no_progress, "處理檔案");
    }
    pm.finish(total_files as u64, None, 0);
    Ok(())
}

fn finalize_compression(
    input_path: &Path,
    output_dir: &str,
    zip_buffer: Vec<u8>,
    layer: &str,
    password: Option<&str>,
    display_password: bool,
    total_size: usize,
    aes_mode: AesMode,
) -> io::Result<()> {
    let (file_name, download_zip_name) = get_file_name(input_path, layer);
    let final_zip_buffer = if layer == "double" {
        create_zip(&zip_buffer, &file_name, layer, password, aes_mode)?
    } else {
        zip_buffer
    };

    let zip_base64 = encode_to_base64(&final_zip_buffer, input_path)?;
    info!("生成最終 ZIP 的 Base64，總大小：{} 位元組", zip_base64.len());

    let instructions = generate_instructions(layer, password.is_some());
    let (password_info, password_display) = handle_password_display(
        password,
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
        &download_zip_name,
        &instructions,
        &file_size_str,
        &password_info,
        &password_display,
    );

    write_html_file(&html_content, output_dir, &file_name)?;
    info!("生成 HTML 文件：{}/{}.html，大小：{} 位元組", output_dir, file_name, html_content.len());
    Ok(())
}

pub fn process_compressed(
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
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored);

    let (files, total_size) = collect_and_measure_files(input_path, include_set, exclude_set, max_size, no_progress)?;
    let total_files = files.len();
    info!("開始壓縮 {} 個檔案（內層 ZIP）", total_files);

    let password = crate::utils::generate_password(&password_mode, preset_password)?;
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

    let pm = create_progress_bar(total_files as u64, no_progress);
    let start = std::time::Instant::now();
    let inner_zip_buffer = create_inner_zip(input_path, &files, options, password.as_deref(), aes_mode, no_progress)?;
    if total_files % 1000 != 0 {
        manage_progress(&pm, total_files as u64, Some(total_size), start, no_progress, "內層壓縮檔案");
    }
    pm.finish(total_files as u64, Some(total_size), 0);
    info!("內層 ZIP 壓縮完成，共處理 {} 個檔案，總大小：{} 位元組", total_files, total_size);
    if let Some(ref pwd) = password {
        info!("內層 ZIP 使用密碼：{}", pwd);
    }

    finalize_compression(
        input_path,
        output_dir,
        inner_zip_buffer,
        layer,
        password.as_deref(),
        display_password,
        total_size,
        aes_mode,
    )?;

    Ok(())
}