#!/bin/bash
# 生成 Homebrew Formula 文件

set -e

VERSION="$1"
BIN="$2"
REPO="$3"
CLASS_NAME="${4:-Sshm}"
DESC="${5:-SSH config manager CLI}"

if [ -z "$VERSION" ] || [ -z "$BIN" ] || [ -z "$REPO" ]; then
    echo "Usage: $0 <version> <bin_name> <repo> [class_name] [description]" >&2
    exit 1
fi

# 读取 SHA256 值（从环境变量）
MACOS_ARM64="${MACOS_ARM64:-}"
MACOS_AMD64="${MACOS_AMD64:-}"
LINUX_ARM64="${LINUX_ARM64:-}"
LINUX_AMD64="${LINUX_AMD64:-}"

cat <<EOF
# typed: false
# frozen_string_literal: true

# ${DESC}
class ${CLASS_NAME} < Formula
  desc "${DESC}"
  homepage "https://github.com/${REPO}"
  version "${VERSION}"
  license "MIT"
EOF

# 标记是否有平台块输出
HAS_PLATFORM=false

# macOS 部分
if [ -n "$MACOS_ARM64" ] || [ -n "$MACOS_AMD64" ]; then
    HAS_PLATFORM=true
    echo ""
    echo "  on_macos do"

    if [ -n "$MACOS_ARM64" ]; then
        cat <<EOF
    on_arm do
      url "https://github.com/${REPO}/releases/download/v#{version}/${BIN}-v#{version}-macos-arm64.zip"
      sha256 "${MACOS_ARM64}"

      define_method(:install) do
        bin.install "${BIN}"
      end
    end
EOF
    fi

    if [ -n "$MACOS_AMD64" ]; then
        cat <<EOF
    on_intel do
      url "https://github.com/${REPO}/releases/download/v#{version}/${BIN}-v#{version}-macos-amd64.zip"
      sha256 "${MACOS_AMD64}"

      define_method(:install) do
        bin.install "${BIN}"
      end
    end
EOF
    fi

    echo "  end"
fi

# Linux 部分
if [ -n "$LINUX_ARM64" ] || [ -n "$LINUX_AMD64" ]; then
    if [ "$HAS_PLATFORM" = true ]; then
        echo ""
    fi
    HAS_PLATFORM=true
    echo "  on_linux do"

    if [ -n "$LINUX_ARM64" ]; then
        cat <<EOF
    on_arm do
      url "https://github.com/${REPO}/releases/download/v#{version}/${BIN}-v#{version}-linux-arm64.zip"
      sha256 "${LINUX_ARM64}"

      define_method(:install) do
        bin.install "${BIN}"
      end
    end
EOF
    fi

    if [ -n "$LINUX_AMD64" ]; then
        cat <<EOF
    on_intel do
      url "https://github.com/${REPO}/releases/download/v#{version}/${BIN}-v#{version}-linux-amd64.zip"
      sha256 "${LINUX_AMD64}"

      define_method(:install) do
        bin.install "${BIN}"
      end
    end
EOF
    fi

    echo "  end"
fi

echo ""
cat <<EOF
  test do
    assert_match version.to_s, shell_output("#{bin} --version")
  end
end
EOF
