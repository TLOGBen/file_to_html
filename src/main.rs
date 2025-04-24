mod cli;
mod file;
mod zip;
mod html;
mod convert;
mod utils;

use std::io;

use crate::cli::process_args;

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let output_dir = process_args(args)?;
    log::info!("程式執行完成，輸出目錄：{}", output_dir);
    println!("轉換完成！輸出檔案位於：{}", output_dir);
    Ok(())
}