use std::io;
use crate::models::file::{FileCollectInput, FileCollectOutput};
use crate::models::zip::{ZipCompressInput, ZipCompressOutput};
use crate::models::html::{HtmlGenerateInput, HtmlGenerateOutput};

// File 服務接口，負責檔案收集
pub trait FileServiceTrait: Send + Sync {
    /// 收集符合條件的檔案
    /// # 參數
    /// - input: 檔案收集的輸入參數
    /// # 回傳
    /// - 成功時返回收集的檔案列表和總大小，失敗時返回 IO 錯誤
    fn collect_files(&self, input: FileCollectInput) -> io::Result<FileCollectOutput>;
}

// Zip 服務接口，負責檔案壓縮
pub trait ZipServiceTrait: Send + Sync {
    /// 壓縮檔案生成 ZIP
    /// # 參數
    /// - input: 壓縮所需的輸入參數
    /// # 回傳
    /// - 成功時返回壓縮後的 ZIP 數據和總大小，失敗時返回 IO 錯誤
    fn compress_files(&self, input: ZipCompressInput) -> io::Result<ZipCompressOutput>;
}

// HTML 服務接口，負責生成 HTML 檔案
pub trait HtmlServiceTrait: Send + Sync {
    /// 根據輸入生成 HTML 檔案
    /// # 參數
    /// - input: HTML 生成的輸入參數
    /// # 回傳
    /// - 成功時返回生成的 HTML 檔案路徑，失敗時返回 IO 錯誤
    fn generate_html(&self, input: HtmlGenerateInput) -> io::Result<HtmlGenerateOutput>;
}