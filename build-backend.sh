#!/bin/bash
# Goバックエンドをビルドしてsrc-tauri/binariesに配置するスクリプト

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BACKEND_DIR="$SCRIPT_DIR/backend"
BINARIES_DIR="$SCRIPT_DIR/src-tauri/binaries"

# Rustのターゲットトリプルを取得
TARGET=$(rustc -vV | grep host | awk '{print $2}')

echo "ターゲット: $TARGET"
echo "Goバックエンドをビルド中..."

cd "$BACKEND_DIR"
go build -o "$BINARIES_DIR/backend-$TARGET" .

echo "ビルド完了: $BINARIES_DIR/backend-$TARGET"
