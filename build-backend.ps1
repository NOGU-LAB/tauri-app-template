# Goバックエンドをビルドして src-tauri/binaries に配置するスクリプト (Windows用)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$BackendDir = Join-Path $ScriptDir "backend"
$BinariesDir = Join-Path $ScriptDir "src-tauri\binaries"

$GoExe = (Get-Command go -ErrorAction Stop).Source
$RustcExe = (Get-Command rustc -ErrorAction Stop).Source

# Tauriの --target が指定されていればその値を使い、なければホスト向けにする。
$Target = $env:TAURI_ENV_TARGET_TRIPLE
if (-not $Target) {
    $Target = (& $RustcExe -vV | Select-String "host:").ToString().Split(":")[1].Trim()
}

switch -Wildcard ($Target) {
    "x86_64-*-windows-*"  { $GoArch = "amd64"; break }
    "aarch64-*-windows-*" { $GoArch = "arm64"; break }
    default { throw "未対応のWindows Rustターゲットです: $Target" }
}
Write-Host "ターゲット: $Target"

# Goバイナリをビルド
Write-Host "Goバックエンドをビルド中..."
New-Item -ItemType Directory -Force -Path $BinariesDir | Out-Null
$env:CGO_ENABLED = "0"
$env:GOOS = "windows"
$env:GOARCH = $GoArch
$Output = Join-Path $BinariesDir "backend-$Target.exe"
Push-Location $BackendDir
try {
    & $GoExe build -trimpath -o $Output .
} finally {
    Pop-Location
}

Write-Host "ビルド完了: $Output"
