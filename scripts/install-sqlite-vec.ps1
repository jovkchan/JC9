# sqlite-vec 扩展安装脚本 (Windows x64)
# 下载预编译的 vec0.dll 并放置到 src-tauri 目录

param(
    [string]$Version = "v0.1.10-alpha.4"
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$TargetDir = Join-Path (Join-Path $ScriptDir "..") "src-tauri"

$ReleaseUrl = "https://github.com/asg017/sqlite-vec/releases/download/$Version/sqlite-vec-$($Version.TrimStart('v'))-loadable-windows-x86_64.tar.gz"
$TempTar = Join-Path $env:TEMP "sqlite-vec.tar.gz"
$TempExtract = Join-Path $env:TEMP "sqlite-vec-extract"

Write-Host "=== JC9 sqlite-vec 扩展安装 ===" -ForegroundColor Cyan
Write-Host "版本: $Version"
Write-Host "下载地址: $ReleaseUrl"
Write-Host ""

# 下载
try {
    Write-Host "正在下载 sqlite-vec..." -ForegroundColor Yellow
    Invoke-WebRequest -Uri $ReleaseUrl -OutFile $TempTar -UseBasicParsing
    Write-Host "✅ 下载完成" -ForegroundColor Green
} catch {
    Write-Host "❌ 下载失败: $_" -ForegroundColor Red
    Write-Host ""
    Write-Host "手动下载步骤:" -ForegroundColor Yellow
    Write-Host "1. 访问 https://github.com/asg017/sqlite-vec/releases/tag/$Version"
    Write-Host "2. 下载 loadable-windows-x86_64.tar.gz"
    Write-Host "3. 解压后将 vec0.dll 放到: $TargetDir"
    exit 1
}

# 解压 (tar.gz)
try {
    Write-Host "正在解压..." -ForegroundColor Yellow
    if (Test-Path $TempExtract) { Remove-Item -Recurse -Force $TempExtract }
    New-Item -ItemType Directory -Force -Path $TempExtract | Out-Null
    tar -xzf $TempTar -C $TempExtract
    if ($LASTEXITCODE -ne 0) {
        throw "tar 解压失败，退出码: $LASTEXITCODE"
    }
} catch {
    Write-Host "❌ 解压失败: $_" -ForegroundColor Red
    exit 1
}

# 复制 DLL
$DllPath = Get-ChildItem -Path $TempExtract -Recurse -Filter "vec0.dll" | Select-Object -First 1
if (-not $DllPath) {
    $DllPath = Get-ChildItem -Path $TempExtract -Recurse -Filter "*.dll" | Select-Object -First 1
}

if ($DllPath) {
    $DestPath = Join-Path $TargetDir "vec0.dll"
    Copy-Item -Path $DllPath.FullName -Destination $DestPath -Force
    Write-Host "✅ vec0.dll 已安装到: $DestPath" -ForegroundColor Green
} else {
    Write-Host "❌ 在解压包中未找到 vec0.dll" -ForegroundColor Red
    Write-Host "请手动从 $TempExtract 中找到 DLL 并复制到 $TargetDir"
    exit 1
}

# 清理
Remove-Item -Force $TempTar -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force $TempExtract -ErrorAction SilentlyContinue

Write-Host ""
Write-Host "=== 安装完成 ===" -ForegroundColor Cyan
Write-Host "重启 JC9 后，向量语义检索将自动启用 sqlite-vec 加速。" -ForegroundColor Green
Write-Host "无需 OpenAI API Key 时，将使用哈希向量作为降级方案。" -ForegroundColor Gray
