use std::io;
use clap::Parser;
use crate::config::config::{Cli, Mode, PasswordMode, validate_input_path, validate_file_patterns};
use crate::action::interactive::process_interactive_mode;
use crate::utils::utils::setup_logging;
use crate::config::ports::{AppConfig, ConfigPort, ConversionPort};
use crate::service::config_service::{ConfigService, DefaultConfigAdapter};
use crate::utils::convert::ConversionAdapter;

pub fn process_args(args: Vec<String>) -> io::Result<String> {
    if args.len() == 1 {
        process_interactive_mode()
    } else {
        process_cli_mode()
    }
}

pub fn process_cli_mode() -> io::Result<String> {
    let cli = Cli::parse();
    setup_logging(&cli.log_level)?;

    // 選擇配置適配器
    let config_port: Box<dyn ConfigPort> = if cli.use_default_config {
        Box::new(DefaultConfigAdapter::new(cli.input.clone(), cli.output.clone()))
    } else {
        Box::new(CliConfigAdapter::new(cli.clone())) // 使用 clone
    };

    let config_service = ConfigService::new(config_port);
    let config = config_service.get_config()?;

    let conversion_port: Box<dyn ConversionPort> = Box::new(ConversionAdapter);
    let copy_config = config.clone(); // 克隆配置以傳遞給轉換器
    let output = conversion_port.execute(config)?; // 傳遞借用

    // 若啟用 --show-config，在轉換後顯示配置
    if cli.show_config {
        println!("實際使用的配置：{:#?}", copy_config);
    }

    Ok(output)
}

// CLI 配置適配器
pub struct CliConfigAdapter {
    cli: Cli,
}

impl CliConfigAdapter {
    pub fn new(cli: Cli) -> Self {
        CliConfigAdapter { cli }
    }
}

impl ConfigPort for CliConfigAdapter {
    fn get_config(&self) -> io::Result<AppConfig> {
        // 驗證輸入路徑
        validate_input_path(&self.cli.input)?;
        // 驗證檔案模式
        validate_file_patterns(&self.cli.include, &self.cli.exclude)?;
        // 驗證壓縮模式下的層數
        if matches!(self.cli.mode, Mode::Compressed) && self.cli.layer == "none" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "壓縮模式下不支援 'none' 層數，請選擇 'single' 或 'double'"
            ));
        }

        let password_mode = match self.cli.password_mode.as_str() {
            "random" => PasswordMode::Random,
            "manual" => PasswordMode::Manual,
            "timestamp" => PasswordMode::Timestamp,
            "none" => PasswordMode::None,
            _ => PasswordMode::Random,
        };

        if self.cli.password_mode == "manual" {
            Some(crate::action::interactive::prompt_manual_password()?)
        } else {
            None
        };

        // 檢查是否忽略了自訂參數
        if self.cli.use_default_config && (
            self.cli.mode != Mode::Individual ||
                self.cli.layer != "double" ||
                self.cli.password_mode != "random" ||
                self.cli.compress != true ||
                self.cli.encryption_method != "aes256" ||
                self.cli.no_progress != false ||
                self.cli.max_size.is_some() ||
                self.cli.include != vec!["*"] ||
                self.cli.exclude.is_some() ||
                self.cli.display_password != Some(true)
        ) {
            log::warn!("使用預設配置，忽略指定的其他參數：mode={:?}, layer={}, password_mode={}, compress={}, encryption_method={}, no_progress={}, max_size={:?}, include={:?}, exclude={:?}, display_password={:?}",
                self.cli.mode, self.cli.layer, self.cli.password_mode, self.cli.compress,
                self.cli.encryption_method, self.cli.no_progress, self.cli.max_size,
                self.cli.include, self.cli.exclude, self.cli.display_password);
        }

        Ok(AppConfig {
            input: self.cli.input.clone(),
            output: self.cli.output.clone(),
            is_compressed: matches!(self.cli.mode, Mode::Compressed),
            compress: self.cli.compress,
            include: self.cli.include.clone(),
            exclude: self.cli.exclude.clone(),
            password_mode,
            display_password: self.cli.display_password.unwrap_or_else(|| self.cli.password_mode == "random"),
            layer: self.cli.layer.clone(),
            encryption_method: self.cli.encryption_method.clone(),
            no_progress: self.cli.no_progress,
            max_size: self.cli.max_size,
        })
    }
}