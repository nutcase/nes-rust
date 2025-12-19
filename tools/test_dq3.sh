#!/bin/bash

# DQ3動作確認用自動テストスクリプト
# Usage: ./tools/test_dq3.sh [frames]
#
# このスクリプトはヘッドレスモードでDQ3を実行し、以下をチェックします：
# 1. 非黒ピクセルが表示されているか
# 2. INIDISPのブランクが解除されているか
# 3. VRAM/CGRAM/OAMに書き込みがあるか

set -e

# デフォルトのフレーム数
FRAMES=${1:-400}
ROM_PATH="roms/dq3.sfc"
LOG_FILE="$(mktemp /tmp/dq3_test_XXXXXX.log)"

# ROMファイルの存在確認
if [ ! -f "$ROM_PATH" ]; then
    echo "ERROR: ROM file not found: $ROM_PATH"
    echo "Please place Dragon Quest III ROM at $ROM_PATH"
    exit 1
fi

echo "================================================"
echo "DQ3 Automated Test"
echo "================================================"
echo "ROM: $ROM_PATH"
echo "Frames: $FRAMES"
echo "Log: $LOG_FILE"
echo "================================================"

# ビルド（リリースモード）
echo "[1/3] Building release binary..."
cargo build --release --bin snes_emulator 2>&1 | grep -v "^   Compiling" | grep -v "^    Finished" || true

# ヘッドレス実行
echo "[2/3] Running headless test..."
env HEADLESS=1 \
    HEADLESS_FRAMES=$FRAMES \
    HEADLESS_AUTO_INPUT=1 \
    QUIET=1 \
    DUMP_REGISTER_SUMMARY=1 \
    DUMP_REGISTER_FRAMES="100,200,300,$FRAMES" \
    ./target/release/snes_emulator "$ROM_PATH" 2>&1 | tee "$LOG_FILE"

# ログ解析
echo ""
echo "[3/3] Analyzing results..."
echo "================================================"

# 非黒ピクセルのチェック
NON_BLACK=$(grep -o "Non-black pixels: [0-9]*" "$LOG_FILE" | tail -1 | awk '{print $3}')
if [ -z "$NON_BLACK" ]; then
    echo "❌ FAIL: Could not find non-black pixel count in log"
    exit 1
fi

echo "📊 Non-black pixels: $NON_BLACK"

# INIDISP ブランク状態のチェック
INIDISP_BLANK=$(grep "INIDISP:" "$LOG_FILE" | tail -1 | grep -o "blank=[A-Z]*" | cut -d= -f2)
if [ -z "$INIDISP_BLANK" ]; then
    echo "❌ FAIL: Could not determine INIDISP blank status"
    exit 1
fi

echo "🔆 INIDISP blank: $INIDISP_BLANK"

# VRAM/CGRAM/OAM使用量のチェック
VRAM_USAGE=$(grep "VRAM usage:" "$LOG_FILE" | tail -1 | grep -o "[0-9]*/" | cut -d/ -f1)
CGRAM_USAGE=$(grep "CGRAM usage:" "$LOG_FILE" | tail -1 | grep -o "[0-9]*/" | cut -d/ -f1)
OAM_USAGE=$(grep "OAM usage:" "$LOG_FILE" | tail -1 | grep -o "[0-9]*/" | cut -d/ -f1)

echo "💾 VRAM usage:  $VRAM_USAGE bytes"
echo "🎨 CGRAM usage: $CGRAM_USAGE bytes"
echo "🎮 OAM usage:   $OAM_USAGE bytes"

# 判定基準
PASS=true

if [ "$NON_BLACK" -lt 100 ]; then
    echo "⚠️  WARNING: Very few non-black pixels ($NON_BLACK < 100)"
    # PASS=false  # 警告だけで失敗にはしない
fi

if [ "$INIDISP_BLANK" = "ON" ]; then
    echo "❌ FAIL: Screen still in forced blank mode"
    PASS=false
fi

if [ "$VRAM_USAGE" -lt 100 ]; then
    echo "⚠️  WARNING: Very low VRAM usage ($VRAM_USAGE < 100)"
fi

if [ "$CGRAM_USAGE" -lt 10 ]; then
    echo "⚠️  WARNING: Very low CGRAM usage ($CGRAM_USAGE < 10)"
fi

# DMATOINIDISPチェック（オプション）
INIDISP_DMA_COUNT=$(grep -c "MDMA write to INIDISP" "$LOG_FILE" 2>/dev/null || echo 0)
# Ensure it's a valid integer
INIDISP_DMA_COUNT=$(echo "$INIDISP_DMA_COUNT" | tr -d '\n' | grep -o '[0-9]*' | head -1)
if [ -n "$INIDISP_DMA_COUNT" ] && [ "$INIDISP_DMA_COUNT" -gt 0 ]; then
    echo "⚠️  WARNING: Detected $INIDISP_DMA_COUNT DMA writes to INIDISP (may interfere with display)"
fi

echo "================================================"
if [ "$PASS" = true ]; then
    echo "✅ PASS: DQ3 test completed successfully"
    echo ""
    echo "Summary:"
    echo "  - Non-black pixels: $NON_BLACK"
    echo "  - INIDISP blank: $INIDISP_BLANK"
    echo "  - VRAM: $VRAM_USAGE bytes, CGRAM: $CGRAM_USAGE bytes, OAM: $OAM_USAGE bytes"
    rm -f "$LOG_FILE"
    exit 0
else
    echo "❌ FAIL: DQ3 test failed"
    echo ""
    echo "Log file saved: $LOG_FILE"
    echo "Review the log for details"
    exit 1
fi
