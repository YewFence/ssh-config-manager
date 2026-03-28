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

class ${CLASS_NAME} < Formula
  desc "${DESC}"
  homepage "https://github.com/${REPO}"
  version "${VERSION}"
  license "MIT"

EOF

# macOS ARM
if [ -n "$MACOS_ARM64" ]; then
    cat <<EOF
  on_macos do
    on_arm do
      url "https://github.com/${REPO}/releases/download/v#{version}/${BIN}-v#{version}-macos-arm64"
      sha256 "${MACOS_ARM64}"

      def install
        bin.install "${BIN}-v#{version}-macos-arm64" => "${BIN}"
      end
    end
EOF
fi

# macOS Intel
if [ -n "$MACOS_AMD64" ]; then
    if [ -z "$MACOS_ARM64" ]; then
        echo "  on_macos do"
    fi
    cat <<EOF
    on_intel do
      url "https://github.com/${REPO}/releases/download/v#{version}/${BIN}-v#{version}-macos-amd64"
      sha256 "${MACOS_AMD64}"

      def install
        bin.install "${BIN}-v#{version}-macos-amd64" => "${BIN}"
      end
    end
EOF
    if [ -z "$MACOS_ARM64" ]; then
        echo "  end"
    else
        echo "  end"
    fi
fi

# Linux ARM
if [ -n "$LINUX_ARM64" ]; then
    cat <<EOF
  on_linux do
    on_arm do
      url "https://github.com/${REPO}/releases/download/v#{version}/${BIN}-v#{version}-linux-arm64"
      sha256 "${LINUX_ARM64}"

      def install
        bin.install "${BIN}-v#{version}-linux-arm64" => "${BIN}"
      end
    end
EOF
fi

# Linux Intel
if [ -n "$LINUX_AMD64" ]; then
    if [ -z "$LINUX_ARM64" ]; then
        echo "  on_linux do"
    fi
    cat <<EOF
    on_intel do
      url "https://github.com/${REPO}/releases/download/v#{version}/${BIN}-v#{version}-linux-amd64"
      sha256 "${LINUX_AMD64}"

      def install
        bin.install "${BIN}-v#{version}-linux-amd64" => "${BIN}"
      end
    end
EOF
    if [ -z "$LINUX_ARM64" ]; then
        echo "  end"
    else
        echo "  end"
    fi
fi

cat <<EOF

  test do
    assert_match version.to_s, shell_output("#{bin} --version")
  end
end
EOF
