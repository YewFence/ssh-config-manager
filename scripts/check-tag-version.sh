#!/usr/bin/env bash
while read local_ref local_sha remote_ref remote_sha; do
  echo "$local_ref" | grep -q "^refs/tags/v" || continue
  TAG=$(basename "$local_ref")
  TAG=${TAG#v}
  CARGO_VERSION=$(cargo metadata --format-version=1 --no-deps | jq -r '.packages[0].version')
  if [ "$TAG" != "$CARGO_VERSION" ]; then
    echo "Tag v$TAG 与 Cargo.toml 版本 $CARGO_VERSION 不一致"
    exit 1
  fi
  echo "Tag v$TAG 与 Cargo.toml 版本一致"
done
