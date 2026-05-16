#!/usr/bin/env bash
# build.sh - Build script with pre-build test verification
# Exits with non-zero code if any step fails
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$PROJECT_DIR"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info()  { echo -e "${GREEN}[INFO]${NC}  $1"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC}  $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# --- Step 1: Run all unit tests ---
log_info "=== Step 1: Running all unit tests ==="
if cargo test 2>&1; then
    log_info "All unit tests passed."
else
    log_error "Unit tests failed! Aborting release build."
    exit 1
fi

# --- Step 2: Release build ---
log_info "=== Step 2: Building release binary ==="
if cargo build --release 2>&1; then
    log_info "Release build succeeded: target/release/terminal-music-player"
else
    log_error "Release build failed! Aborting."
    exit 1
fi

log_info "=== Build complete ==="
