#!/bin/bash
# Update ESPHome API proto files

set -euo pipefail

UPSTREAM=https://raw.githubusercontent.com/esphome/esphome/dev/esphome/components/api
PROTO_DIR=proto

FILES=("api.proto" "api_options.proto")

mkdir -p "$PROTO_DIR"

changed=0

for file in "${FILES[@]}"; do
    tmpfile="$PROTO_DIR/$file.new"
    finalfile="$PROTO_DIR/$file"

    echo "Downloading: $UPSTREAM/$file"
    curl -sSfL -o "$tmpfile" "$UPSTREAM/$file"

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

echo "Done."

if [[ $changed -eq 1 ]]; then
    echo
    git diff --stat "$PROTO_DIR"
    echo ">>> Proto files updated."
    echo ">>> Run: cargo clean && cargo build"
    echo
fi
