# file_to_html

`file_to_html` 是一個 Rust 開發的檔案轉換工具，能將檔案或目錄轉換為嵌入式 HTML 格式。

## 安裝指南

### 方法一：從源碼編譯

1. **確認 Rust 環境**：
   請先安裝 Rust 和 Cargo。若尚未安裝，執行：

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

### 方法二：使用 Cargo 安裝

```bash
cargo install file_to_html
```

## 使用指南

### 快速開始

最簡單的使用方式是啟用預設配置：

```bash
file_to_html <輸入路徑> -o <輸出目錄>
```

這將使用預設設定（壓縮模式、單層壓縮、隨機密碼等）進行處理。

### 命令列模式

#### 基本語法

```bash
file_to_html <輸入路徑> [選項參數]
```

#### 主要選項

| 參數                         | 說明                                              | 預設值         |
| ---------------------------- | ------------------------------------------------- | -------------- |
| `-o, --output <路徑>`        | 指定輸出目錄                                      | `output`       |
| `--mode <模式>`              | 轉換模式：`individual`或`compressed`              | `compressed`   |
| `--include <模式>`           | 包含檔案模式（如：`*.txt,*.pdf`）                 | `*`（全部）    |
| `--exclude <模式>`           | 排除檔案模式（如：`*.jpg,*.png`）                 | 無             |
| `--compress`                 | 個別模式下是否壓縮檔案                            | `true`         |
| `--password-mode <模式>`     | 密碼模式：`random`、`manual`、`timestamp`或`none` | `random`       |
| `--display-password`         | 在 HTML 中顯示密碼                                | 依密碼模式而定 |
| `--layer <層數>`             | ZIP 層數：`none`、`single`或`double`              | `single`       |
| `--encryption-method <方法>` | 加密方法：`aes128`、`aes192`或`aes256`            | `aes256`       |
| `--max-size <MB>`            | 處理檔案大小上限（MB）                            | 無限制         |
| `--log-level <級別>`         | 日誌級別：`info`、`warn`或`error`                 | `info`         |
| `--no-progress`              | 不顯示進度條                                      | `false`        |
| `--show-config`              | 顯示實際使用的配置                                | `false`        |

#### 實用範例

**範例 1：單檔轉換（單層加密）**

```bash
file_to_html ./report.pdf --mode individual --layer single --password-mode random
```

- 結果：生成`output/report.pdf.html`，內含單層加密 ZIP

**範例 2：目錄壓縮（雙層加密）**

```bash
file_to_html ./project_files --mode compressed --layer double --password-mode manual
```

- 結果：提示輸入密碼，生成`output/project_files.html`，內含雙層加密 ZIP

**範例 3：特定檔案類型處理**

```bash
file_to_html ./documents --include "*.docx,*.xlsx,*.pptx" --exclude "*.tmp,*_old.*"
```

- 結果：僅處理 Office 文件，排除暫存和舊版檔案

**範例 4：預設配置（簡化指令）**

```bash
file_to_html ./mydata --show-config
```

- 結果：使用預設設定處理目錄，並顯示實際使用的配置

**範例 5：使用時間戳密碼**

```bash
file_to_html ./mydata --password-mode timestamp --display-password
```

- 結果：使用時間戳格式（yyyyMMddhhmmss）作為密碼，並在 HTML 中顯示

### 互動模式使用

不提供命令列參數時，程式會啟動互動模式，引導完成設定：

1. **啟動互動模式**：

   ```bash
   file_to_html
   ```

2. **選擇預設配置**：
   首先決定是否使用預設配置，這會跳過大部分設定步驟

3. **逐步設定**（如不使用預設配置）：
   - 指定輸入路徑
   - 選擇轉換模式（個別/壓縮）
   - 設定 ZIP 層數
   - 選擇密碼方式
   - 配置其他選項

互動操作範例：

```
=== 歡迎使用互動模式 ===
是否使用預設配置？（壓縮模式、單層壓縮、隨機密碼等，僅需指定輸入和輸出路徑） [Y/n]: n
請輸入檔案或目錄路徑（例如：./myfile.txt 或 ./mydir）: ./project_docs
選擇轉換模式（使用方向鍵選擇，按 Enter 確認）:
> 個別 - 為每個檔案生成單獨的 HTML
  壓縮 - 壓縮成單個 ZIP 嵌入 HTML
選擇 ZIP 層數（使用方向鍵選擇，按 Enter 確認）:
  不壓縮
> 單層 - 僅生成一層 ZIP
  雙層 - 生成外層和內層 ZIP（預設）
選擇密碼模式（使用方向鍵選擇，按 Enter 確認）:
> 隨機生成（16 位，預設）
  手動輸入
  時間戳（yyyyMMddhhmmss）
  無密碼
是否在 HTML 中顯示隨機生成的密碼？（預設為是） [Y/n]: y
輸入輸出目錄（例如：./output，預設為 output）: secure_output
輸入包含模式（例如：.txt,.pdf，預設為 *）: *.pdf,*.docx
輸入排除模式（例如：.jpg,.png，預設為空）: *draft*,*temp*
```

## 使用須知

- **預設配置說明**：預設配置使用壓縮模式、單層壓縮和隨機密碼
- **密碼安全性**：隨機密碼適合日常使用，重要資料建議使用強密碼手動輸入
- **檔案體積**：Base64 編碼會增加約 33%的檔案大小，超過 10MB 的檔案可能影響 HTML 載入速度
- **解壓建議**：加密 ZIP 檔案建議使用 7-Zip、WinRAR 等專業解壓工具開啟
- **密碼管理**：未直接顯示的密碼會儲存在`*.html.key`檔案中，請妥善保存
