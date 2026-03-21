#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
CARGO_TOML="$PROJECT_ROOT/Cargo.toml"

# バージョン決定: 引数があればそれを使い、なければ Cargo.toml から取得
if [[ $# -ge 1 ]]; then
  VERSION="$1"
else
  VERSION="$(grep '^version' "$CARGO_TOML" | head -1 | sed 's/.*"\(.*\)".*/\1/')"
  echo "==> Cargo.toml のバージョンを使用: $VERSION"
fi

# semver バリデーション
if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "ERROR: 不正なバージョン形式: $VERSION (x.y.z 形式で指定してください)" >&2
  exit 1
fi

TAG="v${VERSION}"

# 既存タグチェック
if git tag -l "$TAG" | grep -q .; then
  echo "ERROR: タグ ${TAG} は既に存在します" >&2
  exit 1
fi

# 引数指定時は Cargo.toml のバージョンを更新してコミット
CURRENT_VERSION="$(grep '^version' "$CARGO_TOML" | head -1 | sed 's/.*"\(.*\)".*/\1/')"
if [[ "$VERSION" != "$CURRENT_VERSION" ]]; then
  echo "==> Cargo.toml のバージョンを ${CURRENT_VERSION} → ${VERSION} に更新します..."
  sed -i.bak "s/^version = \"${CURRENT_VERSION}\"/version = \"${VERSION}\"/" "$CARGO_TOML"
  rm -f "$CARGO_TOML.bak"
  git -C "$PROJECT_ROOT" add Cargo.toml
  git -C "$PROJECT_ROOT" commit -m "chore: bump version to ${VERSION}"
  git -C "$PROJECT_ROOT" push origin HEAD
  echo "==> Cargo.toml を更新してコミット・push しました。"
fi

# タグを作成して push → GitHub Actions (release.yml) がリリース・Tap更新を実行
echo "==> タグ ${TAG} を作成して push します..."
git tag "$TAG"
git push origin "$TAG"

echo ""
echo "==> タグ ${TAG} を push しました。"
echo "    GitHub Actions が Release 作成・Homebrew tap 更新を自動実行します。"
echo "    進捗: https://github.com/somei-san/cliip-show/actions"
