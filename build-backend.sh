#!/bin/bash
# Goバックエンドをビルドしてsrc-tauri/binariesに配置するスクリプト

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BACKEND_DIR="$SCRIPT_DIR/backend"
BINARIES_DIR="$SCRIPT_DIR/src-tauri/binaries"

# Tauriの --target が指定されていればその値を使い、なければホスト向けにする。
TARGET="${TAURI_ENV_TARGET_TRIPLE:-$(rustc -vV | awk '/^host:/ { print $2 }')}"

case "$TARGET" in
  x86_64-*-darwin)  GOOS=darwin;  GOARCH=amd64 ;;
  aarch64-*-darwin) GOOS=darwin;  GOARCH=arm64 ;;
  x86_64-*-windows-*)  GOOS=windows; GOARCH=amd64 ;;
  aarch64-*-windows-*) GOOS=windows; GOARCH=arm64 ;;
  x86_64-*-linux-*)  GOOS=linux;   GOARCH=amd64 ;;
  aarch64-*-linux-*) GOOS=linux;   GOARCH=arm64 ;;
  armv7-*-linux-*)   GOOS=linux;   GOARCH=arm; GOARM=7 ;;
  *)
    echo "未対応のRustターゲットです: $TARGET" >&2
    exit 1
    ;;
esac

OUTPUT="$BINARIES_DIR/backend-$TARGET"
if [ "$GOOS" = "windows" ]; then
  OUTPUT="$OUTPUT.exe"
fi

echo "ターゲット: $TARGET"
echo "Goバックエンドをビルド中..."

mkdir -p "$BINARIES_DIR"
cd "$BACKEND_DIR"
if [ -n "${GOARM:-}" ]; then
  CGO_ENABLED=0 GOOS="$GOOS" GOARCH="$GOARCH" GOARM="$GOARM" go build -trimpath -o "$OUTPUT" .
else
  CGO_ENABLED=0 GOOS="$GOOS" GOARCH="$GOARCH" go build -trimpath -o "$OUTPUT" .
fi

echo "ビルド完了: $OUTPUT"
