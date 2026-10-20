[CmdletBinding()]
param(
    [switch]$SkipTests
)

$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "Cargo was not found. Install Rust with the MSVC toolchain first."
}

# 빌드 대상 파일이 실행 중인 경우 종료하여 파일 락 해제
Get-Process -Name "IMECaret", "ime-caret" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 300

if (-not $SkipTests) {
    Write-Host "[1/4] Running tests"
    cargo test
    if ($LASTEXITCODE -ne 0) { throw "cargo test failed" }
}

Write-Host "[2/4] Building release binary"
cargo build --release
if ($LASTEXITCODE -ne 0) { throw "cargo build --release failed" }

Write-Host "[3/4] Preparing distribution folder"
$dist = Join-Path $PSScriptRoot "dist"
New-Item -ItemType Directory -Force -Path $dist | Out-Null

$sourceExe = Join-Path $PSScriptRoot "target\release\IMECaretPlus.exe"
$targetExe = Join-Path $dist "IMECaretPlus.exe"
Copy-Item -Force $sourceExe $targetExe
# 구버전 호환 이름 (설치 스크립트가 함께 배포)
Copy-Item -Force $sourceExe (Join-Path $dist "IMECaret.exe")
Copy-Item -Force $sourceExe (Join-Path $dist "ime-caret.exe")

foreach ($ini in @("IMECaretPlus.ini", "IMECaret.ini")) {
    $targetConfig = Join-Path $dist $ini
    if (-not (Test-Path -LiteralPath $targetConfig)) {
        Copy-Item -LiteralPath (Join-Path $PSScriptRoot "IMECaret.ini") -Destination $targetConfig
    }
}

foreach ($name in @("IMEE.wav", "IMEJ.wav", "IMEK.wav")) {
    $sound = Join-Path $PSScriptRoot ("assets\" + $name)
    if (Test-Path $sound) {
        Copy-Item -Force $sound $dist
    }
}

Write-Host "[4/4] Building Inno Setup installer"
$isccPaths = @(
    "C:\Program Files (x86)\Inno Setup 6\ISCC.exe",
    "C:\Program Files\Inno Setup 6\ISCC.exe"
)
$iscc = $isccPaths | Where-Object { Test-Path $_ } | Select-Object -First 1
if ($iscc) {
    $issFile = Join-Path $PSScriptRoot "installer\installer.iss"
    & $iscc $issFile
    if ($LASTEXITCODE -ne 0) { throw "Inno Setup compilation failed" }
    $setupExe = Join-Path $PSScriptRoot "installer\Output\IMECaretPlus-Setup.exe"
    Write-Host "Installer created: $setupExe"
} else {
    Write-Warning "Inno Setup ISCC.exe not found. Skipped installer packaging."
}

Write-Host ""
Write-Host "Build complete!"
Write-Host "  Binary:    $targetExe"
Write-Host "  Installer: $(Join-Path $PSScriptRoot 'installer\Output\IMECaretPlus-Setup.exe')"
