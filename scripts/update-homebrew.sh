#!/bin/bash
# 更新 Homebrew Tap 中的 Formula

set -e

VERSION="$1"
BIN="$2"
REPO="$3"
TAP_OWNER="$4"
TAP_REPO="$5"
CLASS_NAME="${6:-Sshm}"
DESC="${7:-SSH config manager CLI}"

if [ -z "$VERSION" ] || [ -z "$BIN" ] || [ -z "$REPO" ] || [ -z "$TAP_OWNER" ] || [ -z "$TAP_REPO" ]; then
    echo "Usage: $0 <version> <bin_name> <repo> <tap_owner> <tap_repo> [class_name] [description]" >&2
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FORMULA_PATH="Formula/${BIN}.rb"

# Step 1: 下载并计算 SHA256
echo "Downloading release assets..."
TMPDIR=$(mktemp -d)
trap "rm -rf $TMPDIR" EXIT

cd "$TMPDIR"

for platform in macos-arm64 macos-amd64 linux-arm64 linux-amd64; do
    gh release download "v${VERSION}" --repo "${REPO}" \
        --pattern "${BIN}-v${VERSION}-${platform}" --output "${platform}" 2>/dev/null || true
done

# 导出 SHA256 到环境变量
export MACOS_ARM64=""
export MACOS_AMD64=""
export LINUX_ARM64=""
export LINUX_AMD64=""

for platform in macos-arm64 macos-amd64 linux-arm64 linux-amd64; do
    if [ -f "$platform" ]; then
        sha=$(sha256sum "$platform" | cut -d' ' -f1)
        varname=$(echo "$platform" | tr '-' '_' | tr '[:lower:]' '[:upper:]')
        export "$varname=$sha"
        echo "Found $platform: $sha"
    fi
done

# Step 2: 生成 Formula
echo "Generating formula..."
cd - > /dev/null
"${SCRIPT_DIR}/generate-homebrew-formula.sh" "$VERSION" "$BIN" "$REPO" "$CLASS_NAME" "$DESC" > "$TMPDIR/formula.rb"

echo "Generated formula:"
cat "$TMPDIR/formula.rb"

# Step 3: 获取现有文件 SHA（如果存在）
echo "Checking existing formula..."
EXISTING_SHA=$(gh api "repos/${TAP_OWNER}/${TAP_REPO}/contents/${FORMULA_PATH}" 2>/dev/null | jq -r '.sha // empty' || true)

# Step 4: 上传 Formula
echo "Uploading formula to ${TAP_OWNER}/${TAP_REPO}..."
CONTENT=$(base64 -w 0 "$TMPDIR/formula.rb")

if [ -n "$EXISTING_SHA" ]; then
    echo "Updating existing formula (sha: $EXISTING_SHA)..."
    gh api "repos/${TAP_OWNER}/${TAP_REPO}/contents/${FORMULA_PATH}" \
        --method PUT \
        --field message="chore: bump ${BIN} to v${VERSION}" \
        --field content="$CONTENT" \
        --field sha="$EXISTING_SHA"
else
    echo "Creating new formula..."
    gh api "repos/${TAP_OWNER}/${TAP_REPO}/contents/${FORMULA_PATH}" \
        --method PUT \
        --field message="chore: add ${BIN} v${VERSION}" \
        --field content="$CONTENT"
fi

echo "Homebrew formula updated successfully!"
