# file_to_html

`file_to_html` 是一個 Rust 編寫的命令列工具，用於將檔案或目錄轉換為嵌入式 HTML 格式。支援將檔案轉換為單獨的 HTML 文件（`individual` 模式）或壓縮為單一 ZIP 檔案並嵌入 HTML（`compressed` 模式）。生成的 HTML 文件包含 Base64 編碼的檔案數據，並提供下載連結和解壓說明，支援單層或雙層 ZIP 壓縮，以及 AES 加密選項。

## 功能特色

- **靈活的轉換模式**：支援 `individual`（每個檔案生成獨立 HTML）和 `compressed`（壓縮為單一 ZIP 嵌入 HTML）模式。
- **ZIP 層數選擇**：可選擇無壓縮（僅 `individual` 模式）、單層壓縮或雙層壓縮。
- **加密支援**：支援 AES-128、AES-192、AES-256 加密，密碼模式包括隨機生成、手動輸入、時間戳或無密碼。
- **檔案過濾**：透過包含和排除模式（通配符）過濾處理的檔案。
- **進度條與日誌**：提供進度條顯示轉換進度，以及詳細的日誌輸出。
- **跨平台**：支援 Windows、Linux 和 macOS。

## 版本資訊

- **當前版本**：v0.1.0
- **發佈日期**：2025-04-25
- **相容性**：Rust 1.65 或以上

## 安裝

1. **安裝 Rust**：
   請確保已安裝 Rust 和 Cargo。若尚未安裝，請執行以下命令：
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **克隆專案**：
   ```bash
   git clone <your-repository-url>
   cd file_to_html
   ```

3. **編譯與安裝**：
   ```bash
   cargo build --release
   ```
   可執行檔案位於 `target/release/file_to_html`。

4. **（可選）全局安裝**：
   將可執行檔案複製到系統路徑，例如：
   ```bash
   cp target/release/file_to_html /usr/local/bin/
   ```

## 依賴

專案依賴以下 Rust 庫（已包含在 `Cargo.toml` 中）：
```toml
[dependencies]
base64 = "0.22"
clap = { version = "4.5", features = ["derive"] }
dialoguer = "0.11"
env_logger = "0.11"
indicatif = "0.17"
log = "0.4"
pathdiff = "0.2"
rand = "0.8"
regex = "1.10"
zip = { version = "2.2", features = ["aes-crypto"] }
chrono = "0.4"
```

## CLI 使用方式

### 基本語法

```bash
file_to_html <INPUT> [OPTIONS]
```

- `<INPUT>`：輸入檔案或目錄的路徑（必須存在）。
- `[OPTIONS]`：可選參數，見下表。

### 常用選項

| 選項 | 描述 | 預設值 |
|------|------|--------|
| `-o, --output <PATH>` | 輸出目錄路徑 | `output` |
| `--mode <MODE>` | 轉換模式：`individual` 或 `compressed` | `individual` |
| `--include <PATTERNS>` | 包含模式（通配符，例如 `*.txt,*.pdf`） | `*` |
| `--exclude <PATTERNS>` | 排除模式（通配符，例如 `*.jpg,*.png`） | 無 |
| `--compress` | 在 `individual` 模式下壓縮檔案為 ZIP | `true` |
| `--password-mode <MODE>` | 密碼模式：`random`, `manual`, `timestamp`, `none` | `random` |
| `--display-password` | 在 HTML 中顯示密碼（`random` 模式預設 `true`） | 依模式而定 |
| `--compression-level <LEVEL>` | 壓縮等級：`stored`（無壓縮）或 `deflated` | `deflated` |
| `--layer <LAYER>` | ZIP 層數：`none`, `single`, `double` | `double` |
| `--encryption-method <METHOD>` | 加密方法：`aes128`, `aes192`, `aes256` | `aes256` |
| `--no-progress` | 禁用進度條 | `false` |
| `--max-size <MB>` | 檔案大小限制（MB） | 無限制 |
| `--log-level <LEVEL>` | 日誌級別：`info`, `warn`, `error` | `info` |

### 示例

1. **將單一檔案轉換為 HTML（單層壓縮，隨機密碼）**：
   ```bash
   file_to_html ./myfile.txt --output ./output --mode individual --layer single --password-mode random
   ```
   - 生成 `output/myfile.txt.html`，包含單層加密 ZIP。

2. **將目錄壓縮為單一 ZIP（雙層壓縮，手動密碼）**：
   ```bash
   file_to_html ./mydir --output ./output --mode compressed --layer double --password-mode manual
   ```
   - 提示輸入密碼，生成 `output/mydir.html`，包含雙層加密 ZIP。

3. **過濾特定檔案類型（僅處理 `.txt` 和 `.pdf`）**：
   ```bash
   file_to_html ./mydir --output ./output --include "*.txt,*.pdf" --exclude "*.jpg,*.png"
   ```
   - 僅處理 `.txt` 和 `.pdf` 檔案，排除 `.jpg` 和 `.png`。

## 互動介面使用方式

若不提供命令列參數，程式將進入互動模式，逐步提示使用者輸入所需參數：

1. **啟動互動模式**：
   ```bash
   cargo run
   ```

2. **輸入提示**：
   - **輸入路徑**：輸入檔案或目錄路徑（例如 `./myfile.txt` 或 `./mydir`）。
   - **轉換模式**：選擇 `個別`（每個檔案生成 HTML）或 `壓縮`（壓縮為單一 ZIP）。
   - **ZIP 層數**：
     - `壓縮` 模式：選擇 `單層` 或 `雙層`。
     - `個別` 模式：選擇 `不壓縮`、`單層` 或 `雙層`。
   - **密碼模式**：選擇 `隨機生成`（16 位）、`手動輸入`、`時間戳`（yyyyMMddhhmmss）或 `無密碼`。
   - **顯示密碼**：選擇是否在 HTML 中顯示密碼（隨機模式預設 `是`，其他模式預設 `否`）。
   - **輸出目錄**：輸入輸出目錄（預設 `output`）。
   - **包含模式**：輸入通配符（例如 `.txt,.pdf`，預設 `*`）。
   - **排除模式**：輸入通配符（例如 `.jpg,.png`，預設空）。
   - **壓縮選項**（僅 `個別` 模式）：選擇是否壓縮檔案為 ZIP（預設 `是`）。

3. **示例輸出**：
   ```
   === 歡迎使用互動模式 ===
   請輸入檔案或目錄路徑（例如：./myfile.txt 或 ./mydir）: ./mydir
   選擇轉換模式（使用方向鍵選擇，按 Enter 確認）: 壓縮 - 壓縮成單個 ZIP 嵌入 HTML
   選擇 ZIP 層數（使用方向鍵選擇，按 Enter 確認）: 雙層 - 生成外層和內層 ZIP（預設）
   選擇密碼模式（使用方向鍵選擇，按 Enter 確認）: 隨機生成（16 位，預設）
   是否在 HTML 中顯示隨機生成的密碼？（預設為是） yes
   輸入輸出目錄（例如：./output，預設為 output）: output
   輸入包含模式（例如：.txt,.pdf，預設為 *）: *
   輸入排除模式（例如：.jpg,.png，預設為空）:
   ```

4. **結果**：
   - 生成 `output/mydir.html`，包含雙層加密 ZIP，密碼顯示在 HTML 中。

## 注意事項

- **密碼安全**：隨機密碼和時間戳密碼適合快速測試，手動輸入密碼時請確保安全性。
- **檔案大小限制**：若 Base64 數據超過 1MB，程式會發出警告，可能影響 HTML 顯示或下載。
- **解壓工具**：建議使用 7-Zip 或 WinRAR 解壓加密 ZIP 檔案。
- **日誌**：日誌輸出到控制台，包含詳細的轉換資訊，位於 `output` 目錄的 `.html.key` 檔案儲存未顯示的密碼。
