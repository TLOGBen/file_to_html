mod service {
    pub(crate) mod config_service;
}

mod config {
    pub(crate) mod config;
    pub(crate) mod ports;
}

mod action {
    pub(crate) mod cli;
    pub(crate) mod interactive;
}

mod utils {
    pub(crate) mod convert;
    pub(crate) mod file;
    pub(crate) mod html;
    pub(crate) mod utils;
    pub(crate) mod zip;
}

use std::io;

use crate::action::cli::process_args;

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let output_dir = process_args(args)?;
    log::info!("程式執行完成，輸出目錄：{}", output_dir);
    println!("轉換完成！輸出檔案位於：{}", output_dir);
    Ok(())
}