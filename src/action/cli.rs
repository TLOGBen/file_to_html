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
    setup_logging(&cli.log_level.clone().unwrap_or("info".to_string()))?;

    // 檢查是否僅提供 input 和 output（預設配置）
    let is_default_config = cli.mode.is_none()
        && cli.include.is_none()
        && cli.exclude.is_none()
        && cli.compress.is_none()
        && cli.password_mode.is_none()
        && cli.display_password.is_none()
        && cli.layer.is_none()
        && cli.encryption_method.is_none()
        && cli.no_progress.is_none()
        && cli.max_size.is_none()
        && cli.log_level.is_none();

    // 選擇配置適配器
    let config_port: Box<dyn ConfigPort> = if is_default_config {
        log::info!("未提供選項參數，使用預設配置：壓縮模式，單層壓縮，隨機密碼");
        Box::new(DefaultConfigAdapter::new(cli.input.clone(), cli.output.clone()))
    } else {
        Box::new(CliConfigAdapter::new(cli.clone()))
    };

    let config_service = ConfigService::new(config_port);
    let config = config_service.get_config()?;

    let conversion_port: Box<dyn ConversionPort> = Box::new(ConversionAdapter);
    let output = conversion_port.execute(config.clone())?;

    // 若啟用 --show-config，在轉換後顯示配置
    if cli.show_config {
        println!("實際使用的配置：{:#?}", config);
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
        if self.cli.mode == Some(Mode::Compressed) && self.cli.layer.as_deref() == Some("none") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "壓縮模式下不支援 'none' 層數，請選擇 'single' 或 'double'"
            ));
        }

        let password_mode = match self.cli.password_mode.as_deref() {
            Some("random") => PasswordMode::Random,
            Some("manual") => PasswordMode::Manual,
            Some("timestamp") => PasswordMode::Timestamp,
            Some("none") => PasswordMode::None,
            _ => PasswordMode::Random, // 預設隨機密碼
        };

        if self.cli.password_mode.as_deref() == Some("manual") {
            Some(crate::action::interactive::prompt_manual_password()?)
        } else {
            None
        };

        // 檢查是否忽略了自訂參數
        if self.cli.mode != Some(Mode::Individual) ||
            self.cli.layer.as_deref() != Some("double") ||
            self.cli.password_mode.as_deref() != Some("random") ||
            self.cli.compress != Some(true) ||
            self.cli.encryption_method.as_deref() != Some("aes256") ||
            self.cli.no_progress != Some(false) ||
            self.cli.max_size.is_some() ||
            self.cli.include != Some(vec!["*".to_string()]) ||
            self.cli.exclude.is_some() ||
            self.cli.display_password != Some(true) {
            log::warn!("使用自訂配置，實際使用的參數：mode={:?}, layer={:?}, password_mode={:?}, compress={:?}, encryption_method={:?}, no_progress={:?}, max_size={:?}, include={:?}, exclude={:?}, display_password={:?}",
                self.cli.mode, self.cli.layer, self.cli.password_mode, self.cli.compress,
                self.cli.encryption_method, self.cli.no_progress, self.cli.max_size,
                self.cli.include, self.cli.exclude, self.cli.display_password);
        }

        Ok(AppConfig {
            input: self.cli.input.clone(),
            output: self.cli.output.clone(),
            is_compressed: self.cli.mode == Some(Mode::Compressed),
            compress: self.cli.compress.unwrap_or(true),
            include: self.cli.include.clone().unwrap_or(vec!["*".to_string()]),
            exclude: self.cli.exclude.clone(),
            password_mode,
            display_password: self.cli.display_password.unwrap_or(self.cli.password_mode.as_deref() == Some("random")),
            layer: self.cli.layer.clone().unwrap_or("double".to_string()),
            encryption_method: self.cli.encryption_method.clone().unwrap_or("aes256".to_string()),
            no_progress: self.cli.no_progress.unwrap_or(false),
            max_size: self.cli.max_size,
        })
    }
}