#!/bin/bash
# 下载 release 资源并计算 SHA256

set -e

VERSION="$1"
BIN="$2"
REPO="$3"

if [ -z "$VERSION" ] || [ -z "$BIN" ] || [ -z "$REPO" ]; then
    echo "Usage: $0 <version> <bin_name> <repo>" >&2
    exit 1
fi

OUTPUT_FILE="${4:-/dev/stdout}"

# 创建临时目录
TMPDIR=$(mktemp -d)
trap "rm -rf $TMPDIR" EXIT

cd "$TMPDIR"

# 下载各平台二进制文件
for platform in macos-arm64 macos-amd64 linux-arm64 linux-amd64; do
    gh release download "v${VERSION}" --repo "${REPO}" \
        --pattern "${BIN}-v${VERSION}-${platform}" --output "${platform}" 2>/dev/null || true
done

# 输出 SHA256 值
{
    for platform in macos-arm64 macos-amd64 linux-arm64 linux-amd64; do
        if [ -f "$platform" ]; then
            sha=$(sha256sum "$platform" | cut -d' ' -f1)
            # 输出格式: PLATFORM_NAME=sha256_value
            echo "${platform//-/_}=${sha}"
        fi
    done
} > "$OUTPUT_FILE"
