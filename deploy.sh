#!/bin/sh
# Test-gated deploy: refuses to ship unless the full suite passes,
# then uploads to the droplet and verifies the served bytes.
set -eu
DIR="$(cd "$(dirname "$0")" && pwd)"

echo "== running tests =="
"$DIR/test/run.sh"

echo "== deploying =="
scp -o BatchMode=yes "$DIR/index.html" vpn:/var/www/unseeing/index.html
scp -o BatchMode=yes "$DIR/index.html" vpn:www/index.html

hashof() {
  if command -v md5 >/dev/null 2>&1; then md5 -q "$1"; else md5sum "$1" | cut -d' ' -f1; fi
}
L="$(hashof "$DIR/index.html")"
TMP="$(mktemp)"
curl -s --max-time 10 http://dggrus.hlab.kz/ > "$TMP"
R="$(hashof "$TMP")"
rm -f "$TMP"
if [ "$L" = "$R" ]; then
  echo "DEPLOYED OK — http://dggrus.hlab.kz ($L)"
else
  echo "HASH MISMATCH: local $L vs served $R"
  exit 1
fi
