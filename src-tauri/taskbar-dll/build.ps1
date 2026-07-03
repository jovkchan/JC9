# jc9_taskbar.dll build script
$ErrorActionPreference = "Stop"

$vsPaths = @(
    "${env:ProgramFiles(x86)}\Microsoft Visual Studio\18\BuildTools",
    "${env:ProgramFiles(x86)}\Microsoft Visual Studio\2022\BuildTools"
)

$vcvars = $null
foreach ($p in $vsPaths) {
    if (Test-Path "$p\VC\Auxiliary\Build\vcvars64.bat") {
        $vcvars = "$p\VC\Auxiliary\Build\vcvars64.bat"
        Write-Host "Found: $p"
        break
    }
}
if (-not $vcvars) { Write-Host "ERROR: vcvars64.bat not found"; exit 1 }

cmd /c "call `"$vcvars`" x64 > nul && set" 2>&1 | ForEach-Object {
    if ($_ -match "^(.*?)=(.*)$") {
        [Environment]::SetEnvironmentVariable($matches[1], $matches[2], "Process")
    }
}

$srcDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$outDir = "$srcDir\..\target\debug"
if (-not (Test-Path $outDir)) { New-Item -ItemType Directory -Force -Path $outDir | Out-Null }

Push-Location $srcDir
try {
    Write-Host "Compiling jc9_taskbar.dll (x64) ..."
    & cl.exe /nologo /utf-8 /O2 /EHsc /LD /Fe:"$outDir\jc9_taskbar.dll" `
        taskbar_dll.cpp `
        ole32.lib shell32.lib user32.lib gdi32.lib `
        /link /DLL /MACHINE:X64 2>&1
    if ($LASTEXITCODE -ne 0) { Write-Host "ERROR: failed (exit $LASTEXITCODE)"; exit 1 }
    $dll = Get-Item "$outDir\jc9_taskbar.dll"
    Write-Host "SUCCESS: $($dll.FullName) ($($dll.Length) bytes)"
} finally { Pop-Location }
