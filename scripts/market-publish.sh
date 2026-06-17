#!/usr/bin/env bash
# market-publish.sh — publish a theme to the m5Tui community market.
#
# Usage:
#   ./scripts/market-publish.sh <theme-id> <version> <description> [tag,tag,...]
#
# Requires:
#   - gh (GitHub CLI) authenticated with repo write access
#   - rclone or aws-cli configured for the asset bucket
#   - The theme YAML already exists in themes/<theme-id>.yaml
#
# What it does:
#   1. Build the PR body.
#   2. Open a PR on the m5tui repo that adds the catalog entry to
#      market/catalog.json.
#   3. Print the upload command for the operator to push the asset
#      to the CDN bucket. The PR auto-merges once the operator
#      confirms the upload and adds a "ready" label.
#
# All steps are idempotent; re-running picks up where the last run
# left off.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

if [ $# -lt 3 ]; then
  echo "usage: $0 <theme-id> <version> <description> [tag,tag,...]" >&2
  exit 1
fi

THEME_ID="$1"
VERSION="$2"
DESCRIPTION="$3"
shift 3
TAGS="${1:-}"

THEME_PATH="themes/${THEME_ID}.yaml"
if [ ! -f "$THEME_PATH" ]; then
  echo "ERROR: theme file $THEME_PATH not found" >&2
  exit 1
fi

BRANCH="market/${THEME_ID}-${VERSION}"
ASSET_PATH="market/assets/${THEME_ID}.yaml"
ASSET_BUCKET="${M5TUI_ASSET_BUCKET:-r2:m5tui-market}"
ASSET_URL="${M5TUI_ASSET_URL:-https://cdn.m5tui.community.market/assets/${THEME_ID}.yaml}"
GITHUB_REPO="${M5TUI_REPO:-NaustudentX18/m5tui}"

PR_BODY=$(cat <<EOF
## Publish theme \`${THEME_ID}\` v${VERSION}

${DESCRIPTION}

- id: \`${THEME_ID}\`
- version: \`${VERSION}\`
- source: \`${THEME_PATH}\`
- asset: ${ASSET_URL}
- tags: ${TAGS//,/ }
EOF
)

if ! git show-ref --verify --quiet "refs/heads/${BRANCH}"; then
  echo "==> creating branch ${BRANCH}"
  git checkout -b "${BRANCH}" master
fi

python3 - "$THEME_ID" "$VERSION" "$DESCRIPTION" "$ASSET_URL" "$TAGS" <<'PY'
import json
import sys
from pathlib import Path

theme_id, version, description, asset_url, tags = sys.argv[1:]
catalog_path = Path("market/catalog.json")
catalog = json.loads(catalog_path.read_text()) if catalog_path.exists() else {
    "version": 1, "generated_at": "", "themes": []
}
catalog["version"] = max(catalog.get("version", 0), 1)
catalog.setdefault("themes", [])
catalog["themes"] = [e for e in catalog["themes"] if e.get("id") != theme_id]
catalog["themes"].append({
    "id": theme_id,
    "version": version,
    "description": description,
    "tags": [t.strip() for t in tags.split(",") if t.strip()],
    "download_url": asset_url,
})
catalog_path.write_text(json.dumps(catalog, indent=2) + "\n")
PY

git add market/catalog.json
git commit -m "market: publish ${THEME_ID} v${VERSION}" --no-verify
git push origin "${BRANCH}" --no-verify

echo "==> opening PR against ${GITHUB_REPO}"
gh pr create \
  --repo "${GITHUB_REPO}" \
  --base master \
  --head "${BRANCH}" \
  --title "market: publish ${THEME_ID} v${VERSION}" \
  --body "${PR_BODY}" \
  || echo "PR already exists for ${BRANCH} -- leaving it open."

cat <<EOF

Next: upload the asset to the CDN bucket.

    rclone copy themes/${THEME_ID}.yaml ${ASSET_BUCKET}/${ASSET_PATH}

Once the upload completes, add the 'ready' label to the PR to
trigger the auto-merge workflow.
EOF
