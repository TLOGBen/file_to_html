use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use zip::write::{SimpleFileOptions, ZipWriter};
use zip::CompressionMethod;
use zip::AesMode;
use log::info;
use pathdiff::diff_paths;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

// 定義壓縮器 trait
pub trait Compressor {
    fn compress_files(&mut self, files: &[PathBuf], input_path: &Path) -> io::Result<Vec<u8>>;
}

pub struct ZipCompressor {
    options: SimpleFileOptions,
    password: Option<String>,
    aes_mode: AesMode,
    pm: Arc<crate::utils::ProgressManager>,
    no_progress: bool,
}

impl ZipCompressor {
    pub fn new(options: SimpleFileOptions, password: Option<&str>, aes_mode: AesMode, no_progress: bool) -> Self {
        let pm = Arc::new(crate::utils::create_progress_bar(0, no_progress));
        ZipCompressor { options, password: password.map(String::from), aes_mode, pm, no_progress }
    }
}

impl Compressor for ZipCompressor {
    fn compress_files(&mut self, files: &[PathBuf], input_path: &Path) -> io::Result<Vec<u8>> {
        let total_files = files.len() as u64;
        self.pm = Arc::new(crate::utils::create_progress_bar(total_files, self.no_progress));
        let mut zip_buffer = Vec::new();
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut zip_buffer));
        let mut total_size = 0;
        let mut processed_files = 0;

        let rt = tokio::runtime::Runtime::new()?;
        for (file_path, relative_path) in files.iter().filter_map(|file_path| {
            diff_paths(file_path, input_path.parent().unwrap_or(input_path))
                .map(|rp| (file_path.clone(), rp.to_string_lossy().replace("\\", "/").trim_start_matches("./").to_string()))
        }) {
            let mut file = rt.block_on(File::open(&file_path))?;
            let mut data = Vec::new();
            rt.block_on(file.read_to_end(&mut data))?;

            if let Some(pwd) = &self.password {
                let encrypt_options = SimpleFileOptions::default()
                    .compression_method(CompressionMethod::Stored)
                    .with_aes_encryption(self.aes_mode, pwd);
                zip.start_file(&relative_path, encrypt_options)?;
            } else {
                zip.start_file(&relative_path, self.options)?;
            }
            zip.write_all(&data)?;
            total_size += data.len();
            processed_files += 1;

            // 每 100 個檔案更新進度條
            if !self.no_progress{
                self.pm.update(processed_files as u64, Some(total_size), "壓縮檔案");
            }
        }

        if !self.no_progress {
            self.pm.update(processed_files as u64, Some(total_size), "壓縮檔案");
        }
        self.pm.finish(processed_files as u64, Some(total_size), 0);
        info!("內層 ZIP 壓縮完成，大小：{} 位元組", total_size);
        zip.finish()?;
        Ok(zip_buffer)
    }
}

pub fn create_inner_zip(input_path: &Path, files: &[PathBuf], options: SimpleFileOptions, password: Option<&str>, aes_mode: AesMode, no_progress: bool) -> io::Result<Vec<u8>> {
    let mut compressor = ZipCompressor::new(options, password, aes_mode, no_progress);
    compressor.compress_files(files, input_path)
}

// 更新其他函數以使用 Stored 模式
pub fn create_zip_buffer(file_name: &str, data: &[u8], options: SimpleFileOptions) -> io::Result<Vec<u8>> {
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
    password: Option<&str>,
    aes_mode: AesMode,
) -> io::Result<Vec<u8>> {
    let mut zip_buffer = Vec::new();
    let mut zip = ZipWriter::new(std::io::Cursor::new(&mut zip_buffer));
    if let Some(pwd) = password {
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .with_aes_encryption(aes_mode, pwd);
        zip.start_file(file_name.to_string(), options)?;
    } else {
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored);
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
            let outer_options = SimpleFileOptions::default()
                .compression_method(CompressionMethod::Stored)
                .with_aes_encryption(aes_mode, pwd);
            outer_zip.start_file(format!("{}_outer.zip", file_name), outer_options)?;
            outer_zip.write_all(data)?;
            outer_zip.finish()?;
            info!("生成外層加密 ZIP，密碼：{}，大小：{} 位元組", pwd, outer_zip_buffer.len());
        } else {
            let outer_options = SimpleFileOptions::default()
                .compression_method(CompressionMethod::Stored);
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
            let options = SimpleFileOptions::default()
                .compression_method(CompressionMethod::Stored)
                .with_aes_encryption(aes_mode, pwd);
            zip.start_file(format!("{}.zip", file_name), options)?;
            zip.write_all(data)?;
            zip.finish()?;
            info!("生成單層加密 ZIP，密碼：{}，大小：{} 位元組", pwd, zip_buffer.len());
        } else {
            let options = SimpleFileOptions::default()
                .compression_method(CompressionMethod::Stored);
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