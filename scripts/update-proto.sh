#!/bin/bash
# Update ESPHome API proto files and generate NOTICE.txt

set -euo pipefail

UPSTREAM_REPO=https://github.com/esphome/esphome
UPSTREAM_RAW=https://raw.githubusercontent.com/esphome/esphome/dev/esphome/components/api
PROTO_DIR=proto
FILES=("api.proto" "api_options.proto")

mkdir -p "$PROTO_DIR"

changed=0

for file in "${FILES[@]}"; do
    tmpfile="$PROTO_DIR/$file.new"
    finalfile="$PROTO_DIR/$file"

    echo "Downloading: $UPSTREAM_RAW/$file"
    curl -sSfL -o "$tmpfile" "$UPSTREAM_RAW/$file"

    if [[ -f "$finalfile" ]]; then
        oldhash=$(sha256sum "$finalfile" | cut -d' ' -f1)
        newhash=$(sha256sum "$tmpfile" | cut -d' ' -f1)

        if [[ "$oldhash" == "$newhash" ]]; then
            echo "No change in $file — keeping existing"
            rm -f "$tmpfile"
        else
            echo "Updated $file — replacing"
            mv "$tmpfile" "$finalfile"
            changed=1
        fi
    else
        echo "Adding new $file"
        mv "$tmpfile" "$finalfile"
        changed=1
    fi
done

# Fetch current upstream commit hash from GitHub API
echo
echo "Fetching ESPHome dev branch commit hash..."
commit_hash=$(curl -s https://api.github.com/repos/esphome/esphome/branches/dev | jq -r .commit.sha)

echo "Writing NOTICE.txt..."
cat > "$PROTO_DIR/NOTICE.txt" <<EOF
The files in this directory are derived from the ESPHome project:

  ${UPSTREAM_REPO}

Specifically, they are copied from:
  ${UPSTREAM_REPO}/tree/dev/esphome/components/api

Files:
$(for f in "${FILES[@]}"; do echo "  - $f"; done)

Upstream branch: dev
Upstream commit: $commit_hash

ESPHome is licensed under the GNU General Public License v3.0 (GPL-3.0).
See: https://github.com/esphome/esphome/blob/dev/LICENSE

These .proto files are used to generate compatible code for the ESPHome API.
Their inclusion in this project is governed by the terms of the GPL.
EOF

echo
echo "NOTICE.txt updated:"
cat "$PROTO_DIR/NOTICE.txt"

echo
if [[ $changed -eq 1 ]]; then
    echo ">>> Proto files updated."
    echo ">>> Run: cargo clean && cargo build"
else
    echo ">>> No proto changes detected."
fi
