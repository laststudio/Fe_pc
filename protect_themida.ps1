# =====================================================================
# Themida 3.2.4.52 PowerShell 自动化加壳脚本 - Fe PC
# =====================================================================

param (
    [string]$ThemidaPath = "",
    [string]$InputExe = "",
    [string]$OutputExe = "",
    [string]$ProjectFile = ""
)

$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

Write-Host "=====================================================================" -ForegroundColor Cyan
Write-Host " Themida 3.2.4.52 一键自动化加壳保护流水线 (PowerShell) - Fe PC" -ForegroundColor Cyan
Write-Host "=====================================================================" -ForegroundColor Cyan
Write-Host ""

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path

# 1. 查找 Themida64.exe 路径
if (-not $ThemidaPath) {
    if ($env:THEMIDA_PATH -and (Test-Path $env:THEMIDA_PATH)) {
        $ThemidaPath = $env:THEMIDA_PATH
    } else {
        $Candidates = @(
            (Join-Path $ScriptDir "tools\Themida\Themida64.exe"),
            (Join-Path $ScriptDir "Themida_x32_x64_v3.2.4.52_Retail_Licensed\Themida64.exe"),
            (Join-Path $ScriptDir "..\Themida_x32_x64_v3.2.4.52_Retail_Licensed\Themida64.exe"),
            (Join-Path $ScriptDir "..\Themida\Themida64.exe"),
            "C:\Program Files\Themida\Themida64.exe",
            "C:\Program Files (x86)\Themida\Themida64.exe"
        )
        foreach ($c in $Candidates) {
            if (Test-Path $c) {
                $ThemidaPath = (Resolve-Path $c).Path
                break
            }
        }
    }
}

if (-not $ThemidaPath -or -not (Test-Path $ThemidaPath)) {
    $FromEnv = Get-Command "Themida64.exe" -ErrorAction SilentlyContinue
    if ($FromEnv) {
        $ThemidaPath = $FromEnv.Source
    }
}

if (-not $ThemidaPath -or -not (Test-Path $ThemidaPath)) {
    Write-Host "[-] 错误: 未检测到 Themida64.exe，请指定 -ThemidaPath 参数或设置 THEMIDA_PATH 环境变量" -ForegroundColor Red
    exit 1
}

# 2. 确定工程配置文件 (fepc.tmd)
if (-not $ProjectFile) {
    $TmdCandidates = @(
        (Join-Path $ScriptDir "fepc.tmd"),
        (Join-Path $ScriptDir "..\fepc.tmd")
    )
    foreach ($t in $TmdCandidates) {
        if (Test-Path $t) {
            $ProjectFile = (Resolve-Path $t).Path
            break
        }
    }
}

if (-not $ProjectFile -or -not (Test-Path $ProjectFile)) {
    Write-Host "[-] 错误: 未检测到加壳配置文件 (fepc.tmd)" -ForegroundColor Red
    exit 1
}

# 3. 确定输入二进制 (fepc.exe)
if (-not $InputExe) {
    $ExeCandidates = @(
        (Join-Path $ScriptDir "target\release\fepc.exe"),
        (Join-Path $ScriptDir "fepc.exe"),
        (Join-Path $ScriptDir "..\fepc.exe"),
        (Join-Path $ScriptDir "fepc\target\release\fepc.exe")
    )
    foreach ($e in $ExeCandidates) {
        if (Test-Path $e) {
            $InputExe = (Resolve-Path $e).Path
            break
        }
    }
}

if (-not $InputExe -or -not (Test-Path $InputExe)) {
    Write-Host "[-] 错误: 待加壳源文件不存在: $InputExe" -ForegroundColor Red
    Write-Host "    请先执行 cargo build --release 生成原生 fepc.exe" -ForegroundColor Yellow
    exit 2
}

# 4. 确定输出二进制
if (-not $OutputExe) {
    $OutputExe = Join-Path $ScriptDir "fepc_protected.exe"
}

# 终止残留进程以防文件写锁
Get-Process -Name fepc, fepc_protected -ErrorAction SilentlyContinue | Stop-Process -Force

Write-Host "[*] 加载 Themida 引擎: $ThemidaPath" -ForegroundColor Gray
Write-Host "[*] 输入二进制文件:   $InputExe" -ForegroundColor Gray
Write-Host "[*] 加壳配置文件:     $ProjectFile" -ForegroundColor Gray
Write-Host "[*] 目标输出文件:     $OutputExe" -ForegroundColor Gray
Write-Host ""
Write-Host "[*] 开始执行高级虚拟机多态编译与混淆装甲注入..." -ForegroundColor Cyan

# 5. 启动加壳进程
$Process = Start-Process -FilePath $ThemidaPath `
    -ArgumentList @("/protect", "`"$ProjectFile`"", "/inputfile", "`"$InputExe`"", "/outputfile", "`"$OutputExe`"", "/shareconsole") `
    -NoNewWindow -PassThru -Wait

$ExitCode = $Process.ExitCode

Write-Host "---------------------------------------------------------------------" -ForegroundColor Gray

if ($ExitCode -eq 0 -and (Test-Path $OutputExe)) {
    Write-Host "[+] 成功！Themida 保护装甲注入与虚拟机编译已圆满完成！" -ForegroundColor Green
    Write-Host "[+] 已生成二进制: $OutputExe" -ForegroundColor Green
    $FileLen = (Get-Item $OutputExe).Length
    $SizeMb = [math]::Round(($FileLen / 1MB), 2)
    Write-Host "    文件体积: $SizeMb MB ($FileLen bytes)" -ForegroundColor Gray
    if ($FileLen -le 25 * 1024 * 1024) {
        Write-Host "    [OK] 文件体积符合发布规格限制。" -ForegroundColor Green
    } else {
        Write-Host "    [WARN] 文件体积超过了预期规格！" -ForegroundColor Yellow
    }
} else {
    Write-Host "[-] Themida 加壳失败，退出代码: $ExitCode" -ForegroundColor Red
    if ($ExitCode -ne 0) { exit $ExitCode } else { exit 1 }
}

exit 0
