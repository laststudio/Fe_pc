# =====================================================================
# Themida 64-bit 工具链自动化装配与发现脚本
# 适用于本地研发环境与 GitHub Actions CI (windows-latest)
# =====================================================================

param (
    [switch]$Strict = $false,
    [string]$DestinationDir = "tools\Themida"
)

$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

Write-Host "=====================================================================" -ForegroundColor Cyan
Write-Host " 正在装配与检测 Themida 64-bit 商业加壳防护引擎..." -ForegroundColor Cyan
Write-Host "=====================================================================" -ForegroundColor Cyan

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Resolve-Path (Join-Path $ScriptDir "..\..")

function Write-GithubOutput ($Key, $Val) {
    if ($env:GITHUB_OUTPUT -and (Test-Path $env:GITHUB_OUTPUT)) {
        Add-Content -Path $env:GITHUB_OUTPUT -Value "${Key}=${Val}" -Encoding utf8
    }
}

function Write-GithubEnv ($Key, $Val) {
    if ($env:GITHUB_ENV -and (Test-Path $env:GITHUB_ENV)) {
        Add-Content -Path $env:GITHUB_ENV -Value "${Key}=${Val}" -Encoding utf8
    }
}

# 1. 查找现有安装候选路径
$Candidates = @(
    $env:THEMIDA_PATH,
    $(if ($env:THEMIDA_DIR) { Join-Path $env:THEMIDA_DIR "Themida64.exe" }),
    (Join-Path $RepoRoot $DestinationDir "Themida64.exe"),
    (Join-Path $RepoRoot "Themida_x32_x64_v3.2.4.52_Retail_Licensed\Themida64.exe"),
    (Join-Path $RepoRoot "..\Themida_x32_x64_v3.2.4.52_Retail_Licensed\Themida64.exe"),
    (Join-Path $RepoRoot "..\Themida\Themida64.exe"),
    "C:\Program Files\Themida\Themida64.exe",
    "C:\Program Files (x86)\Themida\Themida64.exe"
)

$FoundPath = $null
foreach ($cand in $Candidates) {
    if ($cand -and (Test-Path $cand)) {
        $FoundPath = (Resolve-Path $cand).Path
        break
    }
}

# 2. 如果未找到，尝试从 GitHub Actions Secrets 下载并还原
if (-not $FoundPath) {
    $TempWorkDir = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [System.IO.Path]::GetTempPath() }
    $DestFull = Join-Path $RepoRoot $DestinationDir

    # 方案 A: 通过 THEMIDA_ARCHIVE_URL 直链下载
    if ($env:THEMIDA_ARCHIVE_URL) {
        Write-Host "[*] 检测到 THEMIDA_ARCHIVE_URL，正在从私有存储下载加壳引擎..." -ForegroundColor Yellow
        $DownloadTarget = Join-Path $TempWorkDir "themida_download.bin"
        Invoke-WebRequest -Uri $env:THEMIDA_ARCHIVE_URL -OutFile $DownloadTarget -UseBasicParsing

        $ExtractSource = $DownloadTarget
        if ($env:THEMIDA_DECRYPT_KEY) {
            Write-Host "[*] 检测到 THEMIDA_DECRYPT_KEY，正在执行 OpenSSL AES-256 解密..." -ForegroundColor Yellow
            $DecryptedZip = Join-Path $TempWorkDir "themida_decrypted.zip"
            & openssl enc -d -aes-256-cbc -pbkdf2 -k "$env:THEMIDA_DECRYPT_KEY" -in "$DownloadTarget" -out "$DecryptedZip"
            if ($LASTEXITCODE -ne 0 -or -not (Test-Path $DecryptedZip)) {
                throw "OpenSSL 解密 Themida 压缩包失败，请检查 THEMIDA_DECRYPT_KEY！"
            }
            $ExtractSource = $DecryptedZip
        }

        Write-Host "[*] 正在解压至: $DestFull" -ForegroundColor Gray
        if (-not (Test-Path $DestFull)) { New-Item -ItemType Directory -Path $DestFull -Force | Out-Null }
        Expand-Archive -Path $ExtractSource -DestinationPath $DestFull -Force
        
        $CheckExe = Join-Path $DestFull "Themida64.exe"
        if (-not (Test-Path $CheckExe)) {
            # 兼容二级子目录
            $SubExe = Get-ChildItem -Path $DestFull -Filter "Themida64.exe" -Recurse | Select-Object -First 1
            if ($SubExe) { $CheckExe = $SubExe.FullName }
        }
        if (Test-Path $CheckExe) { $FoundPath = $CheckExe }
    }

    # 方案 B: 通过私有 GitHub Release 下载 (THEMIDA_REPO)
    if (-not $FoundPath -and $env:THEMIDA_REPO) {
        $Token = if ($env:THEMIDA_TOKEN) { $env:THEMIDA_TOKEN } elseif ($env:PUBLIC_RELEASE_TOKEN) { $env:PUBLIC_RELEASE_TOKEN } else { $env:GH_TOKEN }
        if ($Token) {
            Write-Host "[*] 检测到 THEMIDA_REPO: $($env:THEMIDA_REPO)，正在通过 gh 下载 Release 资产..." -ForegroundColor Yellow
            $GhDir = Join-Path $TempWorkDir "gh_themida"
            if (-not (Test-Path $GhDir)) { New-Item -ItemType Directory -Path $GhDir -Force | Out-Null }
            $env:GH_TOKEN = $Token
            & gh release download --repo $env:THEMIDA_REPO --pattern "*themida*" --dir $GhDir --clobber
            
            $ZipAsset = Get-ChildItem -Path $GhDir -Filter "*.zip" | Select-Object -First 1
            if ($ZipAsset) {
                if (-not (Test-Path $DestFull)) { New-Item -ItemType Directory -Path $DestFull -Force | Out-Null }
                Expand-Archive -Path $ZipAsset.FullName -DestinationPath $DestFull -Force
                $FoundPath = Join-Path $DestFull "Themida64.exe"
            }
        }
    }
}

# 3. 结果判断与环境导出
if ($FoundPath -and (Test-Path $FoundPath)) {
    Write-Host "[+] 成功定位 Themida 64-bit 引擎: $FoundPath" -ForegroundColor Green
    $VersionInfo = (Get-Item $FoundPath).VersionInfo
    Write-Host "    产品版本: $($VersionInfo.ProductVersion)" -ForegroundColor Gray
    Write-Host "    文件体积: $([math]::Round((Get-Item $FoundPath).Length / 1MB, 2)) MB" -ForegroundColor Gray

    Write-GithubOutput "themida_path" "$FoundPath"
    Write-GithubOutput "themida_available" "true"
    Write-GithubEnv "THEMIDA_PATH" "$FoundPath"
    exit 0
} else {
    Write-GithubOutput "themida_available" "false"
    if ($Strict) {
        Write-Host "[-] 错误: 未能在环境中找到 Themida64.exe 引擎！" -ForegroundColor Red
        Write-Host "    在正式发布 (Release) 流水线中，加壳引擎是强制必需的。" -ForegroundColor Yellow
        Write-Host "    请在仓库 Settings -> Secrets -> Actions 中配置以下任一项:" -ForegroundColor Yellow
        Write-Host "      - THEMIDA_ARCHIVE_URL (+ 可选 THEMIDA_DECRYPT_KEY)" -ForegroundColor Yellow
        Write-Host "      - THEMIDA_REPO (配合 PUBLIC_RELEASE_TOKEN)" -ForegroundColor Yellow
        exit 1
    } else {
        Write-Host "[!] 提示: 当前未检测到 Themida 引擎，非严格模式下已安全跳过加壳阶段。" -ForegroundColor Yellow
        exit 0
    }
}
