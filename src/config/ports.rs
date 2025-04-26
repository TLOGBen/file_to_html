use std::io;
use crate::config::config::PasswordMode;

// 應用配置結構體，封裝所有參數
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub input: String,
    pub output: String,
    pub is_compressed: bool,
    pub compress: bool,
    pub include: Vec<String>,
    pub exclude: Option<Vec<String>>,
    pub password_mode: PasswordMode,
    pub display_password: bool,
    pub layer: String,
    pub encryption_method: String,
    pub no_progress: bool,
    pub max_size: Option<f64>,
}

// 配置來源的 Port
pub trait ConfigPort {
    fn get_config(&self) -> io::Result<AppConfig>;
}

// 轉換執行的 Port
pub trait ConversionPort {
    fn execute(&self, config: AppConfig) -> io::Result<String>;
}