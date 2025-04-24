use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use zip::write::{FileOptions, ZipWriter};
use zip::CompressionMethod;
use zip::AesMode;
use log::info;
use pathdiff::diff_paths;

pub fn create_zip_buffer(file_name: &str, data: &[u8], options: FileOptions<()>) -> io::Result<Vec<u8>> {
    let mut zip_buffer = Vec::new();
    let mut zip = ZipWriter::new(std::io::Cursor::new(&mut zip_buffer));
    zip.start_file(file_name, options)?;
    zip.write_all(data)?;
    zip.finish()?;
    Ok(zip_buffer)
}

pub fn compress_file_content(
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

pub fn create_zip(
    data: &[u8],
    file_name: &str,
    layer: &str,
    password: Option<&str>,
    aes_mode: AesMode,
) -> io::Result<Vec<u8>> {
    if layer == "double" {
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

pub fn create_inner_zip(
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
            let (data, _) = crate::file::read_file_content(file_path)?;
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