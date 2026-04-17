# Goバックエンドをビルドして src-tauri/binaries に配置するスクリプト (Windows用)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$BackendDir = Join-Path $ScriptDir "backend"
$BinariesDir = Join-Path $ScriptDir "src-tauri\binaries"

$GoExe = "C:\Program Files\Go\bin\go.exe"
$RustcExe = "$env:USERPROFILE\.cargo\bin\rustc.exe"

# Rustのターゲットトリプルを取得
$Target = (& $RustcExe -vV | Select-String "host:").ToString().Split(":")[1].Trim()
Write-Host "ターゲット: $Target"

# Goバイナリをビルド
Write-Host "Goバックエンドをビルド中..."
Push-Location $BackendDir
& $GoExe build -o "$BinariesDir\backend-$Target.exe" .
Pop-Location

Write-Host "ビルド完了: $BinariesDir\backend-$Target.exe"
