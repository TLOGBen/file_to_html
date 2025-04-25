# file_to_html

`file_to_html` 是一個Rust開發的檔案轉換工具，能將檔案或目錄轉換為嵌入式HTML格式。本工具支援兩種主要模式：將單一檔案轉換為獨立HTML（`individual`模式）或將多個檔案壓縮為單一ZIP後嵌入HTML（`compressed`模式）。生成的HTML檔案包含Base64編碼的檔案資料，提供便捷的下載連結與解壓說明，並支援多層ZIP壓縮及AES加密功能。

## 核心功能

- **轉換模式**：提供個別檔案轉換和批量壓縮兩種基本處理模式
- **壓縮選項**：支援無壓縮、單層或雙層ZIP壓縮，便於不同使用場景
- **資料加密**：實作AES-128/192/256加密，支援多種密碼生成方式
- **檔案篩選**：使用通配符進行檔案選擇與排除
- **進度顯示**：包含進度條和日誌輸出，方便監控處理過程
- **跨平台**：在Windows、macOS和Linux環境中運行

## 版本資訊

- **版本**：v0.1.0
- **發布日期**：2025-04-25
- **相容性需求**：Rust 1.65+

## 安裝指南

### 方法一：從源碼編譯

1. **確認Rust環境**：
   請先安裝Rust和Cargo。若尚未安裝，執行：
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **取得源碼**：
   ```bash
   git clone https://github.com/yourusername/file_to_html.git
   cd file_to_html
   ```

3. **編譯專案**：
   ```bash
   cargo build --release
   ```
   完成後，執行檔位於：`target/release/file_to_html`

4. **選擇性全域安裝**：
   ```bash
   # Linux/macOS
   cp target/release/file_to_html /usr/local/bin/
   
   # Windows (管理員PowerShell)
   Copy-Item .\target\release\file_to_html.exe -Destination "$env:USERPROFILE\.cargo\bin"
   ```

### 方法二：使用Cargo安裝

```bash
cargo install file_to_html
```

## 技術依賴

本專案使用以下Rust套件：
```toml
[dependencies]
base64 = "0.22"         # Base64編碼/解碼
clap = "4.5"            # 命令列參數解析
dialoguer = "0.11"      # 互動式使用者介面
env_logger = "0.11"     # 日誌系統
indicatif = "0.17"      # 進度顯示
log = "0.4"             # 日誌記錄
pathdiff = "0.2"        # 路徑比較工具
rand = "0.8"            # 亂數生成
regex = "1.10"          # 正則表達式
zip = "2.2"             # ZIP壓縮/解壓
chrono = "0.4"          # 時間戳處理
```

## 使用指南

### 命令列模式

#### 基本語法

```bash
file_to_html <輸入路徑> [選項參數]
```

#### 主要選項

| 參數 | 說明 | 預設值 |
|------|------|--------|
| `-o, --output <路徑>` | 指定輸出目錄 | `output` |
| `--mode <模式>` | 轉換模式：`individual`或`compressed` | `individual` |
| `--include <模式>` | 包含檔案模式（如：`*.txt,*.pdf`） | `*`（全部） |
| `--exclude <模式>` | 排除檔案模式（如：`*.jpg,*.png`） | 無 |
| `--compress` | 個別模式下是否壓縮檔案 | `true` |
| `--password-mode <模式>` | 密碼模式：`random`、`manual`、`timestamp`或`none` | `random` |
| `--display-password` | 在HTML中顯示密碼 | 依密碼模式而定 |
| `--layer <層數>` | ZIP層數：`none`、`single`或`double` | `double` |
| `--encryption-method <方法>` | 加密方法：`aes128`、`aes192`或`aes256` | `aes256` |
| `--max-size <MB>` | 處理檔案大小上限（MB） | 無限制 |
| `--log-level <級別>` | 日誌級別：`info`、`warn`或`error` | `info` |
| `--no-progress` | 不顯示進度條 | `false` |

#### 實用範例

**範例1：單檔轉換（單層加密）**
```bash
file_to_html ./report.pdf --mode individual --layer single --password-mode random
```
- 結果：生成`output/report.pdf.html`，內含單層加密ZIP

**範例2：目錄壓縮（雙層加密）**
```bash
file_to_html ./project_files --mode compressed --layer double --password-mode manual
```
- 結果：提示輸入密碼，生成`output/project_files.html`，內含雙層加密ZIP

**範例3：特定檔案類型處理**
```bash
file_to_html ./documents --include "*.docx,*.xlsx,*.pptx" --exclude "*.tmp,*_old.*"
```
- 結果：僅處理Office文件，排除暫存和舊版檔案

### 互動模式使用

不提供命令列參數時，程式會啟動互動模式，引導完成設定：

1. **啟動互動模式**：
   ```bash
   file_to_html
   ```

2. **逐步設定**：
   - 指定輸入路徑
   - 選擇轉換模式（個別/壓縮）
   - 設定ZIP層數
   - 選擇密碼方式
   - 配置其他選項

互動操作範例：
```
=== 歡迎使用互動模式 ===
請輸入檔案或目錄路徑：./project_docs
選擇轉換模式：壓縮 - 壓縮成單個ZIP嵌入HTML
選擇ZIP層數：雙層 - 生成外層和內層ZIP
選擇密碼模式：隨機生成（16位）
是否在HTML中顯示密碼？是
輸入輸出目錄：secure_output
輸入包含模式：*.pdf,*.docx
輸入排除模式：*draft*,*temp*
```

## 使用須知

- **密碼安全性**：隨機密碼適合日常使用，重要資料建議使用強密碼手動輸入
- **檔案體積**：Base64編碼會增加約33%的檔案大小，超過10MB的檔案可能影響HTML載入速度
- **解壓建議**：加密ZIP檔案建議使用7-Zip、WinRAR等專業解壓工具開啟
- **密碼管理**：未直接顯示的密碼會儲存在`*.html.key`檔案中，請妥善保存

## 進階技巧

- 使用`--layer none`搭配`--mode individual`可生成無壓縮的純Base64嵌入HTML
- 雙層加密適合重要資料，提供兩重密碼保護
- 使用`--compression-level stored`可加快處理速度，但不減少檔案大小
- 批處理大量檔案時可使用`--no-progress`減少輸出資訊
