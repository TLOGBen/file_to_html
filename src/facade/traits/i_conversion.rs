use std::io;
use crate::models::conversion::{ConversionInput, ConversionOutput};

// Facade 接口，負責協調檔案轉換流程
pub trait ConversionFacadeTrait: Send + Sync {
    /// 執行檔案轉換，根據輸入配置生成輸出
    /// # 參數
    /// - input: 轉換所需的輸入參數
    /// # 回傳
    /// - 成功時返回轉換結果，失敗時返回 IO 錯誤
    fn execute_conversion(&self, input: ConversionInput) -> io::Result<ConversionOutput>;
}