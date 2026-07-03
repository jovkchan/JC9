# jc9_taskbar.dll build (ReBar) - avoids command-line-too-long
$ErrorActionPreference = "Stop"

$vsRoot = ""
foreach ($p in @(
    "${env:ProgramFiles(x86)}\Microsoft Visual Studio\18\BuildTools",
    "${env:ProgramFiles(x86)}\Microsoft Visual Studio\2022\BuildTools"
)) {
    if (Test-Path "$p\VC\Auxiliary\Build\vcvars64.bat") { $vsRoot = $p; Write-Host "Found: $p"; break }
}
if (-not $vsRoot) { Write-Host "ERROR: vcvars64.bat not found"; exit 1 }

$srcDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$outDir = "$srcDir\..\target\debug"
if (-not (Test-Path $outDir)) { New-Item -ItemType Directory -Force -Path $outDir | Out-Null }

# Find MSVC bin path
$msvcBin = (Resolve-Path "$vsRoot\VC\Tools\MSVC\*\bin\Hostx64\x64").Path
$env:PATH = "$msvcBin;$env:PATH"
$env:INCLUDE = (Resolve-Path "$vsRoot\VC\Tools\MSVC\*\include").Path + ";" + 
               (Resolve-Path "$vsRoot\..\..\..\Windows Kits\10\Include\*\ucrt").Path + ";" +
               (Resolve-Path "$vsRoot\..\..\..\Windows Kits\10\Include\*\um").Path
$env:LIB = (Resolve-Path "$vsRoot\VC\Tools\MSVC\*\lib\x64").Path + ";" +
           (Resolve-Path "$vsRoot\..\..\..\Windows Kits\10\Lib\*\um\x64").Path + ";" +
           (Resolve-Path "$vsRoot\..\..\..\Windows Kits\10\Lib\*\ucrt\x64").Path

Write-Host "Compiling jc9_taskbar.dll (ReBar) ..."
Push-Location $srcDir
try {
    & cl.exe /nologo /utf-8 /O2 /EHsc /LD /Fe:"$outDir\jc9_taskbar.dll" `
        taskbar_dll.cpp `
        ole32.lib shell32.lib user32.lib gdi32.lib dwmapi.lib `
        /link /DLL /MACHINE:X64 2>&1
    if ($LASTEXITCODE -ne 0) { Write-Host "ERROR: failed (exit $LASTEXITCODE)"; exit 1 }
    $dll = Get-Item "$outDir\jc9_taskbar.dll"
    Write-Host "SUCCESS: $($dll.FullName) ($($dll.Length) bytes)"
} finally { Pop-Location }
