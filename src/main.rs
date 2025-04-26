mod service {
    pub(crate) mod file;
    pub(crate) mod html;
    pub(crate) mod zip;
    pub(crate) mod config_service;
    pub(crate) mod traits {
        pub(crate) mod i_service;
    }
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
    pub(crate) mod utils;
}

mod facade {
    pub(crate) mod conversion_facade;
    pub(crate) mod ports {
        pub(crate) mod facade_ports;
    }
    pub(crate) mod traits {
        pub(crate) mod i_conversion;
    }
}

mod models {
    pub(crate) mod conversion;
    pub(crate) mod file;
    pub(crate) mod zip;
    pub(crate) mod html;
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