# UTFsearch.exe 詳細操作手冊

**適用對象**: AI/自動化系統、腳本集成、批量文件查詢

---

## 📋 目錄

1. [快速開始](#快速開始)
2. [核心概念](#核心概念)
3. [命令參考](#命令參考)
4. [搜索語法](#搜索語法)
5. [輸出格式](#輸出格式)
6. [AI 集成指南](#ai-集成指南)
7. [實際應用例](#實際應用例)
8. [性能優化](#性能優化)
9. [故障排除](#故障排除)

---

## 快速開始

### 第一次使用（3 步）

```batch
:: 1. 建立目錄索引（第一次）
utfsearch index --root D:\Share

:: 2. 搜索文件
utfsearch search invoice

:: 3. 定期更新索引
utfsearch refresh
```

### 基本命令結構

```
utfsearch [全局選項] <命令> [命令選項]
```

**全局選項** (所有命令都支持):
- `--catalog <路徑>` - 指定目錄文件位置（默認: exe 同目錄的 `catalog.uts`）
- `--config <路徑>` - 指定配置文件
- `--format [text|json]` - 輸出格式（默認: text）
- `--quiet` - 禁用進度消息

---

## 核心概念

### 什麼是 UTFsearch？

- **不是** 全文搜索引擎（不搜索文件內容）
- **不是** SQL 數據庫（不支持複雜 join）
- **是** 文件元數據目錄：名稱、路徑、大小、修改時間、所有者

### 工作流程

```
[建立索引]
目錄掃描 (一次性) → catalog.uts (二進制)
              ↓
         [搜索] 
    (快速、無需重掃)
```

### 為什麼使用它？

| 場景 | 優勢 |
|------|------|
| 搜索 5M+ 文件 | 毫秒級響應（無需每次重掃） |
| 混合文字名稱 | 支持繁簡中文、日文、emoji 等 |
| AI/自動化 | JSON 輸出，絕對路徑，適合集成 |
| 網絡共享 | 跳過系統目錄，減少掃描時間 |

---

## 命令參考

### 1. `index` - 首次建立索引

**用途**: 掃描磁盤並建立 catalog 文件。只需運行一次。

**語法**:
```
utfsearch index --root <路徑> [--root <路徑2>] [選項]
```

**必需參數**:
- `--root <路徑>` - 要掃描的文件夾（可指定多個）

**可選參數**:
- `--exclude <名稱>` - 跳過指定的文件夾名稱（可重複）
- `--include-system` - 包含 Windows 系統目錄（默認跳過）

**示例**:
```batch
:: 單個根目錄
utfsearch index --root D:\Share

:: 多個根目錄
utfsearch index --root D:\Share --root E:\Projects --root F:\Archives

:: 排除特定文件夾
utfsearch index --root D:\Share --exclude node_modules --exclude .git --exclude __pycache__

:: 包含系統目錄（不推薦）
utfsearch index --root C:\ --include-system

:: 自定義 catalog 位置
utfsearch --catalog C:\Data\my-catalog.uts index --root D:\Share

:: JSON 格式輸出統計
utfsearch --format json index --root D:\Share
```

**輸出示例（text）**:
```
Indexing 5,234,891 entries from 2 roots
  D:\Share (4,192,105 files, 32.2 GB)
  E:\Projects (1,042,786 files, 8.5 GB)
Written to catalog.uts (125.3 MB) in 45.2s
```

**輸出示例（JSON）**:
```json
{
  "roots": [
    {
      "path": "D:\\Share",
      "count": 4192105,
      "size": 34627419955
    }
  ],
  "total_entries": 5234891,
  "total_size": 39764839275,
  "catalog_path": "catalog.uts",
  "catalog_size": 131396608,
  "duration_secs": 45.2
}
```

**關鍵參數說明**:
- 第一次 `index` 必須提供 `--root`
- 建立 catalog 非常耗時（5M 文件約 45 秒）
- 結果存儲在內存映射文件 (catalog.uts)，所有查詢都從中讀取

---

### 2. `refresh` - 增量更新索引

**用途**: 掃描磁盤的新增/刪除/修改，更新 catalog。速度比 index 快 3-10 倍。

**語法**:
```
utfsearch refresh [--root <路徑>] [選項]
```

**可選參數**:
- `--root <路徑>` - 覆蓋原有根目錄（不提供時使用 catalog 內保存的根）
- `--exclude <名稱>` - 跳過指定文件夾
- `--include-system` - 包含系統目錄

**示例**:
```batch
:: 增量更新（推薦每日運行）
utfsearch refresh

:: 覆蓋根目錄
utfsearch refresh --root D:\Share --root E:\Projects

:: 排除新的文件夾
utfsearch refresh --exclude node_modules --exclude temp

:: 使用計時診斷
set UTFSEARCH_TIMING=1
utfsearch refresh
```

**輸出示例（text）**:
```
Refreshing 5,234,891 entries from 2 roots
  D:\Share added 1,234 files, removed 456 files, modified 3,210 files
  E:\Projects added 89 files, removed 12 files, modified 234 files
Written to catalog.uts (125.8 MB) in 8.3s
```

**何時使用**:
- 定期（每天/每周）更新索引
- 監控文件夾變化
- 與自動化任務配合

---

### 3. `search` - 查詢文件

**用途**: 在 catalog 中搜索文件。最常用的命令。

**語法**:
```
utfsearch search [文本] [選項]
```

**基本參數**:
- 第一個位置參數是文件名片段（可選）

**過濾選項**:
- `--name <文本>` - 文件名完全匹配或包含（可重複）
- `--path <文本>` - 相對路徑包含
- `--ext <擴展名>` - 文件擴展名（無需 `.`）
- `--owner <用戶>` - 文件所有者
- `--root <名稱>` - 限制到特定根目錄
- `--after <時間>` - 修改時間晚於（例: `2024-01-01`）
- `--before <時間>` - 修改時間早於
- `--min-size <大小>` - 最小文件大小（例: `1MB`, `512KB`）
- `--max-size <大小>` - 最大文件大小
- `--limit <數字>` - 返回結果數量（默認: 200，最大: 5000）
- `--cursor <字符串>` - 分頁標記（來自前一頁的 `next_cursor`）

**視圖選項**:
- `--full` - 返回完整字段（默認只返回核心字段）

**示例**:

```batch
:: 按文件名搜索
utfsearch search invoice
utfsearch search "2024"

:: 完全匹配
utfsearch search --name "invoice.xlsx"

:: 多個名稱條件（OR）
utfsearch search --name invoice --name report

:: 按路徑搜索
utfsearch search --path finance/2024
utfsearch search --path "finance/2024" --ext xlsx

:: 按擴展名
utfsearch search --ext pdf
utfsearch search --ext "docx"

:: 按時間範圍
utfsearch search --after 2024-01-01 --before 2024-12-31
utfsearch search --after "2024-01-15 10:30:00"

:: 按大小
utfsearch search --ext mp4 --min-size 100MB
utfsearch search --max-size 10KB

:: 按所有者
utfsearch search --owner "domain\\user"
utfsearch search --owner "SYSTEM"

:: 複合查詢
utfsearch search --name invoice --ext xlsx --after 2024-06-01 --limit 500

:: 特定根目錄
utfsearch search --name backup --root D:\Share

:: 分頁
utfsearch search --limit 100
utfsearch search --limit 100 --cursor "eyJpZCI6NDIzLCJtdGltZSI6MTcyMDAwMDAwMH0="

:: 完整視圖
utfsearch search invoice --full

:: JSON 格式
utfsearch --format json search --name invoice --limit 50

:: 組合示例（AI 使用）
utfsearch --format json --quiet search --path docs --ext pdf --min-size 1MB --limit 1000
```

**輸出格式（text）**:
```
Found 42 hits, showing 1-42

  1. INV-2024-001.xlsx
     📁 finance/2024/invoices/
     📍 \\fileserver\share\finance\2024\invoices\INV-2024-001.xlsx
     📊 487 KB | Modified: 2024-06-15 10:30:45 | Owner: DOMAIN\john

  2. INV-2024-002.xlsx
     📁 finance/2024/invoices/
     📍 \\fileserver\share\finance\2024\invoices\INV-2024-002.xlsx
     📊 512 KB | Modified: 2024-06-20 14:22:10 | Owner: DOMAIN\jane

No more results
```

**輸出格式（JSON）**:
```json
{
  "total": 42,
  "limit": 200,
  "hits": [
    {
      "rel": "finance/2024/invoices/INV-2024-001.xlsx",
      "root": "\\\\fileserver\\share",
      "path": "\\\\fileserver\\share\\finance\\2024\\invoices\\INV-2024-001.xlsx",
      "kind": "file",
      "ext": "xlsx",
      "size": 487000,
      "mtime": "2024-06-15T10:30:45Z",
      "owner": "DOMAIN\\john"
    },
    {
      "rel": "finance/2024/invoices/INV-2024-002.xlsx",
      "root": "\\\\fileserver\\share",
      "path": "\\\\fileserver\\share\\finance\\2024\\invoices\\INV-2024-002.xlsx",
      "kind": "file",
      "ext": "xlsx",
      "size": 512000,
      "mtime": "2024-06-20T14:22:10Z",
      "owner": "DOMAIN\\jane"
    }
  ],
  "next_cursor": "eyJpZCI6NDIzLCJtdGltZSI6MTcyMDAwMDAwMH0="
}
```

---

### 4. `tree` - 瀏覽目錄樹

**用途**: 列出特定路徑下的所有文件/文件夾。

**語法**:
```
utfsearch tree <路徑> [--root <名稱>] [--full]
```

**參數**:
- `<路徑>` - 相對或絕對路徑
- `--root` - 指定根目錄名稱
- `--full` - 返回完整字段

**示例**:
```batch
:: 列出目錄樹
utfsearch tree finance/2024

:: 列出特定根目錄
utfsearch tree "D:\Share\finance\2024"

:: 完整視圖
utfsearch tree finance/2024 --full

:: JSON 格式
utfsearch --format json tree finance/2024
```

**輸出示例（text）**:
```
Directory: finance/2024/
Total: 145 items

📁 invoices/ (89 items)
📁 reports/ (34 items)
📁 receipts/ (22 items)
```

---

### 5. `status` - 查看目錄信息

**用途**: 顯示當前 catalog 的統計信息。

**語法**:
```
utfsearch status
```

**示例**:
```batch
utfsearch status
utfsearch --format json status
```

**輸出示例（text）**:
```
Catalog: catalog.uts (125.3 MB)
Created: 2024-08-19 10:15:30
Last updated: 2024-08-19 14:45:10

Roots:
  1. D:\Share (4,192,105 files, 32.2 GB)
  2. E:\Projects (1,042,786 files, 8.5 GB)

Total entries: 5,234,891
Total size: 40.5 GB
```

**輸出示例（JSON）**:
```json
{
  "catalog_path": "catalog.uts",
  "catalog_size": 131396608,
  "created": "2024-08-19T10:15:30Z",
  "updated": "2024-08-19T14:45:10Z",
  "roots": [
    {
      "name": "D:\\Share",
      "count": 4192105,
      "size": 34627419955
    }
  ],
  "total_entries": 5234891,
  "total_size": 39764839275
}
```

---

### 6. `mcp` - MCP 服務器

**用途**: 啟動 MCP 服務器，供 Claude/LLM 調用。

**語法**:
```
utfsearch mcp [--http <端口>] [--token <令牌>]
```

**參數**:
- `--http <端口>` - 啟動 HTTP 服務器（例: `8080`）
- `--token <令牌>` - 認證令牌（可選）

**示例**:
```batch
:: 啟動 stdio MCP（默認）
utfsearch mcp

:: 啟動 HTTP MCP
utfsearch mcp --http 8080

:: 帶認證
utfsearch mcp --http 8080 --token secret123
```

---

## 搜索語法

### 時間格式

支持多種時間格式：

```
絕對日期:
  2024-01-15
  2024-01-15 10:30:45
  2024-01-15T10:30:45Z

相對時間:
  1d    (1 天前)
  7d    (7 天前)
  1w    (1 周前)
  1m    (1 月前)
  1y    (1 年前)
```

**示例**:
```batch
:: 最近 7 天
utfsearch search --after 7d

:: 特定日期範圍
utfsearch search --after 2024-01-01 --before 2024-12-31

:: 確切時刻
utfsearch search --after "2024-08-19 10:00:00"
```

### 大小格式

支持多種大小單位：

```
b/B      字節
kb/KB    千字節
mb/MB    兆字節
gb/GB    吉字節
tb/TB    太字節
```

**示例**:
```batch
:: 1 MB 到 100 MB
utfsearch search --min-size 1MB --max-size 100MB

:: 512 KB
utfsearch search --max-size 512KB

:: 大於 1 GB
utfsearch search --ext iso --min-size 1GB
```

### 名稱匹配

- **片段匹配**: `utfsearch search invoice` 匹配任何包含 "invoice" 的文件名
- **完全匹配**: `utfsearch search --name "invoice.xlsx"` 需要完全相同
- **多條件 OR**: `utfsearch search --name invoice --name report` 匹配任一條件

### 路徑匹配

- **相對路徑**: 相對於根目錄
- **片段搜索**: 搜索路徑中的任何部分

```batch
utfsearch search --path "finance/2024"    # 匹配包含該路徑的任何文件
utfsearch search --path "reports"         # 匹配 reports 目錄下的所有文件
```

---

## 輸出格式

### Text 格式（默認）

適合人類閱讀，包含視覺格式。

```
Found 42 hits, showing 1-42

  1. INV-2024-001.xlsx
     📁 finance/2024/invoices/
     📍 \\fileserver\share\finance\2024\invoices\INV-2024-001.xlsx
     📊 487 KB | Modified: 2024-06-15 10:30:45 | Owner: DOMAIN\john
```

### JSON 格式

適合 AI 和自動化處理。

**對象結構**:

```json
{
  "total": 整數,              // 總結果數
  "limit": 整數,              // 請求的限制
  "hits": [                    // 搜索結果數組
    {
      "rel": "相對路徑",       // 相對於根的路徑
      "root": "根路徑",        // 根目錄絕對路徑
      "path": "絕對路徑",      // 完整絕對路徑（可直接打開）
      "kind": "file|dir",      // 類型
      "ext": "擴展名",         // 文件擴展名（file only）
      "size": 字節數,          // 文件大小（file only）
      "mtime": "ISO時間",      // 修改時間
      "owner": "用戶名"        // 所有者
    }
  ],
  "next_cursor": "字符串"      // 下一頁標記（如果有更多結果）
}
```

### 分頁

當結果超過 `--limit` 時，使用 `next_cursor` 獲取下一頁：

```batch
:: 第一頁（100 個結果）
utfsearch --format json search --limit 100

:: 提取 next_cursor 並用於第二頁
utfsearch --format json search --limit 100 --cursor "<之前的 next_cursor>"
```

**示例（PowerShell）**:
```powershell
$cursor = $null
$page = 1

while ($true) {
    $args = @('search', '--limit', '100')
    if ($cursor) { $args += @('--cursor', $cursor) }
    
    $result = utfsearch --format json @args | ConvertFrom-Json
    
    Write-Host "Page $page: $($result.hits.Count) hits"
    
    if (-not $result.next_cursor) { break }
    
    $cursor = $result.next_cursor
    $page++
}
```

---

## AI 集成指南

### 為 AI 設計的最佳實踐

#### 1. 始終使用 JSON 格式

```batch
:: ✅ 推薦
utfsearch --format json --quiet search --path docs --ext pdf --limit 100

:: ❌ 不推薦
utfsearch search --path docs --ext pdf
```

#### 2. 使用 `--quiet` 隱藏進度消息

```batch
utfsearch --format json --quiet search invoice --limit 50
```

#### 3. 使用絕對路徑

JSON 輸出中的 `path` 字段已經是絕對路徑，可直接傳遞給其他程序：

```json
{
  "path": "\\\\fileserver\\share\\finance\\2024\\invoices\\INV-2024-001.xlsx"
}
```

#### 4. 處理分頁

對於大型查詢，總是檢查 `next_cursor`：

```python
# Python 示例
import json
import subprocess

def search_all(query_args, limit=100):
    """獲取所有搜索結果，自動分頁"""
    results = []
    cursor = None
    
    while True:
        cmd = ['utfsearch', '--format', 'json', '--quiet', 'search'] + query_args
        if cursor:
            cmd.extend(['--cursor', cursor])
        cmd.extend(['--limit', str(limit)])
        
        output = subprocess.run(cmd, capture_output=True, text=True).stdout
        page = json.loads(output)
        
        results.extend(page['hits'])
        
        if not page.get('next_cursor'):
            break
        
        cursor = page['next_cursor']
    
    return results

# 使用
hits = search_all(['--name', 'invoice', '--ext', 'xlsx'])
for hit in hits:
    print(hit['path'])
```

#### 5. 檢查錯誤代碼

UTFsearch 使用特定的退出代碼：

| 代碼 | 含義 |
|------|------|
| 0 | 成功 |
| 1 | 通用錯誤 |
| 2 | 查詢/根目錄錯誤 |
| 3 | Catalog 損壞或版本不符 |
| 4 | I/O 錯誤 |
| 5 | 查詢語法錯誤 |

```python
import subprocess

result = subprocess.run(['utfsearch', 'search', 'invoice'], 
                       capture_output=True)
if result.returncode == 0:
    print("成功")
elif result.returncode == 5:
    print("查詢語法錯誤")
else:
    print(f"錯誤: {result.stderr.decode()}")
```

#### 6. 環境變數

**UTFSEARCH_TIMING**: 顯示性能時序（用於調試）

```batch
set UTFSEARCH_TIMING=1
utfsearch search invoice
```

輸出會包含：
```
[timing] open: 12.3ms
[timing] search: 45.6ms (42 hits)
```

#### 7. Catalog 位置

自動檢測順序：
1. `--catalog` 參數
2. `--config` 指定的配置文件
3. 記住的最後位置（存儲在配置目錄）
4. `catalog.uts` 在 exe 同目錄

```batch
:: 明確指定
utfsearch --catalog D:\Data\my-catalog.uts search invoice

:: 使用配置文件
utfsearch --config D:\config.json search invoice
```

---

## 實際應用例

### 例 1: 自動文件分類

AI 掃描財務目錄，按年份和類型分類發票：

```python
import subprocess
import json
from pathlib import Path

def classify_invoices():
    cmd = [
        'utfsearch', '--format', 'json', '--quiet',
        'search',
        '--name', 'INV',
        '--ext', 'xlsx',
        '--path', 'finance',
        '--limit', '5000'
    ]
    
    result = subprocess.run(cmd, capture_output=True, text=True)
    data = json.loads(result.stdout)
    
    by_year = {}
    for hit in data['hits']:
        # 從相對路徑提取年份
        rel_path = hit['rel']
        year = rel_path.split('/')[1] if len(rel_path.split('/')) > 1 else 'unknown'
        
        if year not in by_year:
            by_year[year] = []
        by_year[year].append(hit['path'])
    
    return by_year

classifications = classify_invoices()
for year, files in classifications.items():
    print(f"{year}: {len(files)} 個文件")
```

### 例 2: 文件變化監控

定期運行 `refresh`，比較新增/刪除文件：

```bash
#!/bin/bash

CATALOG="catalog.uts"
LAST_STATE="last_state.json"

# 獲取當前狀態
utfsearch --format json --quiet status > current_state.json

# 比較大小
OLD_SIZE=$(jq '.total_entries' $LAST_STATE 2>/dev/null || echo 0)
NEW_SIZE=$(jq '.total_entries' current_state.json)
ADDED=$((NEW_SIZE - OLD_SIZE))

if [ $ADDED -gt 0 ]; then
    echo "新增 $ADDED 個文件"
    mail -s "文件夾有新增文件" admin@company.com
fi

# 保存當前狀態
mv current_state.json $LAST_STATE
```

### 例 3: 按大小查找大文件

查找並刪除超過 1 GB 的文件（需謹慎！）：

```powershell
$result = utfsearch --format json --quiet search `
    --ext iso --ext bin --ext img --min-size 1GB |
    ConvertFrom-Json

foreach ($file in $result.hits) {
    $size_gb = [math]::Round($file.size / 1GB, 2)
    Write-Host "$size_gb GB: $($file.path)"
    
    # 可選：刪除
    # Remove-Item $file.path
}
```

### 例 4: MCP 集成（Claude）

在 Claude 中搜索文件（需要配置 MCP）：

```json
{
  "name": "utfsearch",
  "command": "utfsearch mcp",
  "env": {
    "CATALOG": "D:\\Data\\catalog.uts"
  }
}
```

然後在 Claude 中：

```
我需要查找所有 2024 年的發票文件。
```

Claude 將使用 MCP 調用 utfsearch 並返回結果。

### 例 5: 定期索引任務（Windows 計時器）

```xml
<!-- 每天 2:00 AM 運行 refresh -->
<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Date>2024-08-19T00:00:00</Date>
    <Author>admin</Author>
    <Description>每日更新 UTFsearch Catalog</Description>
  </RegistrationInfo>
  <Triggers>
    <TimeTrigger>
      <StartBoundary>2024-08-20T02:00:00</StartBoundary>
      <Repetition>
        <Interval>P1D</Interval>
        <StopAtDurationEnd>false</StopAtDurationEnd>
      </Repetition>
      <Enabled>true</Enabled>
    </TimeTrigger>
  </Triggers>
  <Principals>
    <Principal id="LocalSystem">
      <UserId>S-1-5-18</UserId>
      <RunLevel>HighestAvailable</RunLevel>
    </Principal>
  </Principals>
  <Actions Context="LocalSystem">
    <Exec>
      <Command>C:\Tools\utfsearch.exe</Command>
      <Arguments>--catalog D:\Data\catalog.uts refresh</Arguments>
    </Exec>
  </Actions>
</Task>
```

---

## 性能優化

### 1. 索引優化

#### 時間複雜度

| 操作 | 時間 | 條件 |
|------|------|------|
| `index` | ~9s/M entries | 首次掃描整個目錄樹 |
| `refresh` | ~1.5s/M entries | 增量更新（變化少） |
| `search` | <100ms | 在內存映射的 catalog 中 |

#### 減少索引時間

```batch
:: 排除大型系統目錄
utfsearch index --root C:\ --exclude Windows --exclude "Program Files" --exclude ProgramData

:: 使用 --include-system=false（默認）
utfsearch index --root D:\Share

:: 只索引特定文件夾
utfsearch index --root "D:\Share\Projects" --root "D:\Share\Data"
```

### 2. 搜索優化

#### 利用過濾器減少結果集

```batch
:: ❌ 慢：掃描所有 100M 文件
utfsearch search "2024" --limit 1000

:: ✅ 快：先過濾到 1K 候選，再搜索
utfsearch search "2024" --path "finance/2024" --ext xlsx --limit 1000
```

#### 使用選擇性過濾

按這個順序應用過濾器（最快的優先）：

1. `--ext` (通常最選擇性)
2. `--path` (路徑過濾)
3. `--owner` (所有者過濾)
4. `--after/--before` (時間過濾)
5. `--min-size/--max-size` (大小過濾)
6. `--name` (名稱過濾，最後)

### 3. Catalog 檔案

#### 大小預估

```
5M entries ≈ 100-150 MB catalog
10M entries ≈ 200-300 MB catalog
```

#### 存儲位置建議

- **高速磁盤** (SSD): 最快
- **網絡位置**: 如果搜索頻繁且磁盤 I/O 是瓶頸
- **避免**: USB、網絡慢速位置

#### 壓縮（可選）

UTFsearch 使用二進制格式，已經相對緊湊。如果需要：

```batch
:: 壓縮
7z a catalog.uts.7z catalog.uts

:: 使用前解壓
7z x catalog.uts.7z
```

### 4. 批量搜索優化

#### 並行搜索（多個查詢）

```powershell
# PowerShell
$queries = @(
    @('--name', 'invoice', '--ext', 'xlsx'),
    @('--name', 'report', '--ext', 'pdf'),
    @('--name', 'backup', '--min-size', '100MB')
)

$queries | ForEach-Object -Parallel {
    utfsearch --format json --quiet search @_
} -ThrottleLimit 4
```

#### 流式處理（大型結果集）

```python
import subprocess
import json

# 使用 --limit 減少內存占用
for i in range(0, 100000, 1000):
    cmd = ['utfsearch', '--format', 'json', '--quiet',
           'search', '--limit', '1000', '--cursor', str(i)]
    
    result = subprocess.run(cmd, capture_output=True, text=True)
    page = json.loads(result.stdout)
    
    for hit in page['hits']:
        process(hit)  # 逐個處理，不保存整個結果集
    
    if not page.get('next_cursor'):
        break
```

---

## 故障排除

### 常見問題

#### 1. "catalog not found"

```
Error: catalog not found
```

**原因**: Catalog 文件不存在或位置不對

**解決**:
```batch
:: 確保已運行 index
utfsearch --catalog D:\my-catalog.uts index --root D:\Share

:: 驗證位置
dir catalog.uts
dir D:\my-catalog.uts

:: 指定明確位置
utfsearch --catalog D:\my-catalog.uts status
```

#### 2. "first run needs --root"

```
Error: first run needs --root <folder>
```

**原因**: 首次運行必須指定 `--root`

**解決**:
```batch
utfsearch index --root D:\Share
```

#### 3. 搜索結果為空

```
Found 0 hits
```

**原因**: 查詢條件太嚴格或文件不在索引中

**解決**:
```batch
:: 檢查 Catalog 內容
utfsearch status

:: 寬鬆搜索條件測試
utfsearch search "test"

:: 檢查路徑
utfsearch search --path "finance"

:: 確保 catalog 已更新
utfsearch refresh
```

#### 4. Catalog 損壞或版本錯誤

```
Error: Catalog version mismatch or corrupted
```

**原因**: Catalog 文件被損壞或由不兼容版本創建

**解決**:
```batch
:: 刪除舊 catalog
del catalog.uts

:: 重新索引
utfsearch index --root D:\Share
```

#### 5. "jail escape" 安全錯誤

```
Error: path escapes root jail
```

**原因**: 搜索結果超出允許的根目錄範圍（安全檢查）

**解決**:
```batch
:: 使用 --root 限制
utfsearch search --root "D:\Share"

:: 檢查路徑參數
utfsearch search --path "finance/2024"  # ✅
:: 不要使用: utfsearch search --path "..\..\Windows"
```

#### 6. 性能差（搜索緩慢）

```batch
:: 檢查 Catalog 打開時間
set UTFSEARCH_TIMING=1
utfsearch search invoice

:: 如果 open 時間 > 1s，catalog 可能太大
:: 解決: 拆分成多個較小的 catalog，或使用 SSD
```

#### 7. Windows 權限問題

**原因**: 索引期間無法訪問某些文件/文件夾

**解決**:
```batch
:: 以管理員身份運行
runas /user:Administrator "utfsearch index --root C:\"

:: 或在批處理中設置
@echo off
cd /d %~dp0
net session >nul 2>&1
if %errorlevel% neq 0 (
    powershell -Command "Start-Process cmd -ArgumentList '/c,%~s0' -Verb RunAs"
    exit /b
)
utfsearch index --root D:\Share
```

### 調試技巧

#### 啟用時序診斷

```batch
set UTFSEARCH_TIMING=1
utfsearch search invoice --limit 100
```

輸出：
```
[timing] open: 12.3ms
[timing] search: 45.6ms (42 hits)
```

#### 詳細 JSON 輸出

```batch
utfsearch --format json search invoice | jq .
```

#### 驗證 Catalog 完整性

```batch
utfsearch --quiet status
```

如果輸出為空或錯誤，catalog 可能已損壞。

---

## 快速參考卡

### 最常用命令

```batch
:: 初始化
utfsearch index --root D:\Share

:: 更新
utfsearch refresh

:: 搜索（簡單）
utfsearch search invoice

:: 搜索（複雜）
utfsearch --format json search --name invoice --ext xlsx --after 2024-01-01 --limit 100

:: 查看狀態
utfsearch status

:: 瀏覽目錄
utfsearch tree finance/2024
```

### 一行命令模板

```batch
:: AI 查詢
utfsearch --format json --quiet search --path %PATH% --ext %EXT% --min-size %SIZE% --limit %LIMIT%

:: 分頁
utfsearch --format json --quiet search --limit 100 --cursor %CURSOR%

:: 時間範圍
utfsearch search --after %DATE1% --before %DATE2% --limit 500

:: 複合過濾
utfsearch search --name %NAME% --ext %EXT% --owner %USER% --min-size %SIZE%
```

### 環境變數

```batch
:: 指定 Catalog 位置
set UTFSEARCH_CATALOG=D:\Data\catalog.uts

:: 啟用性能時序
set UTFSEARCH_TIMING=1

:: 在腳本中使用
utfsearch search invoice
```

---

## 相關資源

- **主倉庫**: https://github.com/TunaPig1228/UTFsearch
- **安全政策**: 見 SECURITY.md
- **許可證**: MIT
- **架構文檔**: 見 docs/adr/

---

## 版本歷史

- **v1.0** (2024-08-19): 首次發布
  - 核心搜索功能
  - JSON/Text 輸出
  - WizTree 集成（Windows）
  - 懶加載優化
  - LRU 查詢快取

---

**最後更新**: 2024-08-19  
**適用版本**: utfsearch.exe 1.0+
