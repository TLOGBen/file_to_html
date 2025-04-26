use std::io;
use crate::config::ports::{AppConfig, ConfigPort};
use crate::config::config::PasswordMode;

// 配置服務，負責選擇適當的配置適配器
pub struct ConfigService {
    config_port: Box<dyn ConfigPort>,
}

impl ConfigService {
    pub fn new(config_port: Box<dyn ConfigPort>) -> Self {
        ConfigService { config_port }
    }

    pub fn get_config(&self) -> io::Result<AppConfig> {
        self.config_port.get_config()
    }
}

// 預設配置適配器
pub struct DefaultConfigAdapter {
    input: String,
    output: String,
}

impl DefaultConfigAdapter {
    pub fn new(input: String, output: String) -> Self {
        DefaultConfigAdapter { input, output }
    }
}

impl ConfigPort for DefaultConfigAdapter {
    fn get_config(&self) -> io::Result<AppConfig> {
        Ok(AppConfig {
            input: self.input.clone(),
            output: self.output.clone(),
            is_compressed: true, // 壓縮模式
            compress: true,
            include: vec!["*".to_string()],
            exclude: None,
            password_mode: PasswordMode::Random,
            display_password: true,
            layer: "single".to_string(), // 單層壓縮
            encryption_method: "aes256".to_string(),
            no_progress: false,
            max_size: None,
        })
    }
}