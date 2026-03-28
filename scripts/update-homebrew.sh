#!/bin/bash
# 更新 Homebrew Tap 中的 Formula
# 每个版本创建独立文件（Formula/sshm@VERSION.rb），同时更新主 formula（Formula/sshm.rb）

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

# 版本化 formula 的 class 名称，如 SshmAt001（Homebrew class 名必须是合法 Ruby 常量）
VERSIONED_VERSION_SUFFIX=$(echo "$VERSION" | tr -d '.')
VERSIONED_CLASS="${CLASS_NAME}AT${VERSIONED_VERSION_SUFFIX}"
MAIN_FORMULA_PATH="Formula/${BIN}.rb"
VERSIONED_FORMULA_PATH="Formula/${BIN}@${VERSION}.rb"

# Step 1: 下载并计算 SHA256
echo "Downloading release assets..."
TMPDIR=$(mktemp -d)
trap "rm -rf $TMPDIR" EXIT

cd "$TMPDIR"

for platform in macos-arm64 macos-amd64 linux-arm64 linux-amd64; do
    gh release download "v${VERSION}" --repo "${REPO}" \
        --pattern "${BIN}-v${VERSION}-${platform}" --output "${platform}" 2>/dev/null || true
done

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

cd - > /dev/null

# Step 2: 生成主 formula（CLASS_NAME，用于 brew install sshm）
echo "Generating main formula (${MAIN_FORMULA_PATH})..."
"${SCRIPT_DIR}/generate-homebrew-formula.sh" "$VERSION" "$BIN" "$REPO" "$CLASS_NAME" "$DESC" > "$TMPDIR/main.rb"

# Step 3: 生成版本化 formula（用于 brew install sshm@VERSION）
echo "Generating versioned formula (${VERSIONED_FORMULA_PATH})..."
"${SCRIPT_DIR}/generate-homebrew-formula.sh" "$VERSION" "$BIN" "$REPO" "$VERSIONED_CLASS" "$DESC" > "$TMPDIR/versioned.rb"

echo "--- Main formula ---"
cat "$TMPDIR/main.rb"
echo "--- Versioned formula ---"
cat "$TMPDIR/versioned.rb"

# 通用上传函数
upload_formula() {
    local local_file="$1"
    local remote_path="$2"
    local commit_message="$3"

    EXISTING_SHA=$(gh api "repos/${TAP_OWNER}/${TAP_REPO}/contents/${remote_path}" 2>/dev/null | jq -r '.sha // empty' || true)
    CONTENT=$(base64 -w 0 "$local_file")

    if [ -n "$EXISTING_SHA" ]; then
        echo "Updating ${remote_path} (sha: $EXISTING_SHA)..."
        gh api "repos/${TAP_OWNER}/${TAP_REPO}/contents/${remote_path}" \
            --method PUT \
            --field message="$commit_message" \
            --field content="$CONTENT" \
            --field sha="$EXISTING_SHA"
    else
        echo "Creating ${remote_path}..."
        gh api "repos/${TAP_OWNER}/${TAP_REPO}/contents/${remote_path}" \
            --method PUT \
            --field message="$commit_message" \
            --field content="$CONTENT"
    fi
}

# Step 4: 上传主 formula
echo "Uploading main formula to ${TAP_OWNER}/${TAP_REPO}..."
upload_formula "$TMPDIR/main.rb" "$MAIN_FORMULA_PATH" "chore: bump ${BIN} to v${VERSION}"

# Step 5: 上传版本化 formula（只在不存在时创建，永不覆盖）
VERSIONED_EXISTS=$(gh api "repos/${TAP_OWNER}/${TAP_REPO}/contents/${VERSIONED_FORMULA_PATH}" 2>/dev/null | jq -r '.sha // empty' || true)
if [ -n "$VERSIONED_EXISTS" ]; then
    echo "Versioned formula ${VERSIONED_FORMULA_PATH} already exists, skipping."
else
    echo "Creating versioned formula ${VERSIONED_FORMULA_PATH}..."
    upload_formula "$TMPDIR/versioned.rb" "$VERSIONED_FORMULA_PATH" "chore: add ${BIN} v${VERSION}"
fi

echo "Homebrew formula updated successfully!"
