# Debian 13 development setup and hp-local change ledger

This guide has two jobs:

1. reproduce a complete Unseeing development environment on a fresh x86_64
   Debian 13 workstation; and
2. account for the exact `hp-local` setup performed on 2026-08-24, including
   what already existed, what changed, what did not change, retained evidence,
   generated build output, and bounded rollback.

The commands assume the local user is `galchenko` with home
`/home/galchenko`. For another workstation, review every absolute path before
substitution. Do not paste the dated rollback commands on a different account.

## Authority and supported versus observed versions

Checked-in files own the reproducible contract. The dated values show what the
Debian repositories and mutable installers resolved on `hp-local`; they are
evidence, not new project pins.

| Dependency | Supported reproduction contract | Exact dated result / status | Checked-in owner |
| --- | --- | --- | --- |
| Godot editor/templates | Exactly `4.7.1.stable.official` | `4.7.1.stable.official.a13da4feb` | `.godot-version` |
| Native Rust | Exactly stable `1.97.1`, `rustfmt`, `clippy`, and six declared targets | `rustc 1.97.1 (8bab26f4f 2026-07-14)` | `rust/rust-toolchain.toml` |
| Web Rust | Exactly `nightly-2026-05-25`, `rust-src`, and `wasm32-unknown-emscripten` | `rustc 1.98.0-nightly (423e3d252 2026-05-24)` | `rust/build-wasm.sh` |
| Emscripten | Exactly emsdk `4.0.20` | tag `4.0.20`, checkout `e4fe26ef59168ff44f4c23c466e497bf60b3411e`, Emscripten revision `6913738ec5371a88c4af5a80db0ab42bad3de681` | `rust/build-wasm.sh` |
| gdtoolkit | `4.*` | `4.5.0`; wheel SHA-256 `f25c5bf7f7fe861e1127164c5d73e0a7fb204ec74cf05d375b76a5dcf8610cdb` | `README.md` and `ci/pipeline.sh` |
| Godot MCP developer addon | Exactly `@satelliteoflove/godot-mcp@4.1.0`; ignored and installed per worktree | Not installed during the 2026-08-24 base setup; installed in the isolated proof worktree and verified by successful editor-only attempt 7 in the 2026-08-25 evidence series | `.mcp.json`, `tools/setup-mcp.sh`, and `.gitignore` |
| rustup installer | Current official x86_64 GNU installer, verified against its sidecar | rustup `1.29.0`; installer SHA-256 `4acc9acc76d5079515b46346a485974457b5a79893cfb01112423c89aeb5aa10` | installation procedure in this guide; compiler pins remain in the two Rust owner files above |
| Chromium and Brotli | Debian 13 packages sufficient for the Web smoke/precompression gates | Chromium `151.0.7922.169`; Brotli `1.1.0` | `test/web_smoke.sh` and `ci/pipeline.sh` |
| Node/npm for optional Godot MCP | Node 20 or newer; npm/npx sufficient to install the exact checked-in MCP pin | Node `20.19.2`; npm/npx `9.2.0` were pre-existing | `tools/setup-mcp.sh` and `.mcp.json` |
| Dated offline MCP controller | Ordinary addon installation still requires Node 20 or newer; the sealed 2026-08-25 controller alone requires exact Node `22.23.2` | `/opt/homebrew/Cellar/node@22/22.23.2_1/bin/node`, exact identity recorded below | The dated controller evidence and the MCP-loop document |
| Network time | Debian `systemd-timesyncd`, enabled and synchronized before any bounded remote proof | `257.13-1~deb13u1`; installed and enabled on 2026-08-26 after the first MCP attempt exposed clock skew | This guide's fresh-host prerequisite and dated addendum |
| Registered engine classes | Exact checked-in census | `19` checks | `ci/engine_class_count` |
| Unseeing source baseline | Exact dated setup input, not an ongoing moving pin | `d6285b0bba84dd29846a9613c2e8081191e46cfd` | `docs/superpowers/specs/2026-08-24-hp-local-development-setup-design.md` |
| Superpowers developer tooling | Exact gitlink and lock | `b36e0829c6d0140e93cfef2ca599b1b07d4a7797` (`v6.3.0`) | parent gitlink, `.gitmodules`, and `ci/superpowers.lock` |

The same `game/` Godot project remains authoritative for Web, macOS, Windows,
and Linux. This host setup adds no runtime technology and changes no game,
rendering, wave, physics, perception, scene, shader, or export law.

## Fresh Debian 13 onboarding

Choose one unused calendar date deliberately and export it once, for example
`export UNSEEING_SETUP_DATE=2026-09-17`. Replace that example with the date of
the actual run. Do not derive it automatically: an unattended retry must not
silently create a second audit. Every block below validates and reconstructs
the same date/root, and the first block refuses any existing path. The literal
date `2026-08-24` is reserved for the historical ledger later in this guide.

Each shell fence is independently fail-fast. If opened in a new shell without
`UNSEEING_SETUP_DATE`, it stops before changing anything.

### 1. Establish a private evidence directory and capture the baseline

```sh
set -eu
: "${UNSEEING_SETUP_DATE:?export one unused YYYY-MM-DD setup date}"
reproduction_date=$UNSEEING_SETUP_DATE
test "$reproduction_date" != 2026-08-24
test "$(date -d "$reproduction_date" +%F 2>/dev/null)" = "$reproduction_date"
home_root=$(realpath "$HOME")
setup_parent="$home_root/.local/state/unseeing/setup"
setup_root="$setup_parent/$reproduction_date"
download_root="$setup_root/downloads"
export PATH="$HOME/.local/bin:$PATH"

test ! -e "$setup_root" && test ! -L "$setup_root"
umask 077
for parent in "$home_root/.local" "$home_root/.local/state" \
  "$home_root/.local/state/unseeing" "$setup_parent"; do
  if [ -e "$parent" ] || [ -L "$parent" ]; then
    test ! -L "$parent" && test -d "$parent"
    test "$(stat -c %u "$parent")" = "$(id -u)"
  fi
done
mkdir -p "$setup_parent"
test "$(realpath "$setup_parent")" = "$setup_parent"
test ! -L "$setup_parent" && test -d "$setup_parent"
test "$(stat -c %u "$setup_parent")" = "$(id -u)"
mkdir -m 700 "$setup_root"
mkdir -m 700 "$download_root"
test "$(realpath "$setup_root")" = "$setup_root"
test "$(realpath "$download_root")" = "$download_root"
test ! -L "$setup_root" && test ! -L "$download_root"
test "$(stat -c %u:%a "$setup_root")" = "$(id -u):700"
test "$(stat -c %u:%a "$download_root")" = "$(id -u):700"

dpkg-query -W -f='${binary:Package}\t${Version}\n' | LC_ALL=C sort \
  > "$setup_root/before-packages.tsv"
apt-mark showmanual | LC_ALL=C sort > "$setup_root/before-manual-packages.txt"
for path in /etc/apt/sources.list /etc/apt/sources.list.d; do
  if [ -f "$path" ]; then
    sha256sum "$path"
  elif [ -d "$path" ]; then
    find "$path" -type f -print0 | LC_ALL=C sort -z | xargs -0 -r sha256sum
  else
    printf 'ABSENT\t%s\n' "$path"
  fi
done > "$setup_root/before-apt-source-hashes.txt"
for path in "$HOME/.profile" "$HOME/.bashrc"; do
  if [ -f "$path" ]; then sha256sum "$path"; else printf 'ABSENT\t%s\n' "$path"; fi
done > "$setup_root/before-startup-hashes.txt"
{
  printf 'captured_at='; date --iso-8601=seconds
  printf 'uid='; id -u
  printf 'gid='; id -g
  uname -a
  lscpu
  free -h
  df -h "$HOME"
} > "$setup_root/before-host.txt"
if [ -e "$HOME/.cargo" ] || [ -L "$HOME/.cargo" ]; then
  test ! -L "$HOME/.cargo" && test -d "$HOME/.cargo"
  find "$HOME/.cargo" -xdev -printf '%y\t%m\t%u\t%g\t%s\t%P\n' \
    | LC_ALL=C sort > "$setup_root/before-cargo-metadata.tsv"
  find "$HOME/.cargo" -xdev -type f -print0 | LC_ALL=C sort -z \
    | xargs -0 -r sha256sum > "$setup_root/before-cargo-files.sha256.txt"
else
  printf 'ABSENT\t%s\n' "$HOME/.cargo" \
    > "$setup_root/before-cargo-metadata.tsv"
  : > "$setup_root/before-cargo-files.sha256.txt"
fi
ssh-keygen -F github.com -f "$HOME/.ssh/known_hosts" \
  > "$setup_root/before-known-hosts-github-public.txt" 2>/dev/null || true
```

Do not capture `env`, credentials, tokens, private keys, authentication
material, or command-line secrets. Evidence belongs at mode `0700`; downloads
belong only in its separately removable `downloads/` child.

### 2. Install the Debian-owned prerequisites

This fresh-host transaction includes the basic build/download tools, the two
Debian-owned Web dependencies, Node/npm for the optional pinned MCP editor
loop, and synchronized system time for bounded remote evidence. It records
complete command output and later computes the actual package/manual deltas;
it does not assume either dated `hp-local` transaction.

```sh
set -eu
: "${UNSEEING_SETUP_DATE:?export the chosen YYYY-MM-DD setup date}"
reproduction_date=$UNSEEING_SETUP_DATE
test "$reproduction_date" != 2026-08-24
test "$(date -d "$reproduction_date" +%F 2>/dev/null)" = "$reproduction_date"
home_root=$(realpath "$HOME")
setup_root="$home_root/.local/state/unseeing/setup/$reproduction_date"
export PATH="$HOME/.local/bin:$PATH"
test "$(realpath "$setup_root")" = "$setup_root"
test ! -L "$setup_root" && test -d "$setup_root"
test "$(stat -c %u:%a "$setup_root")" = "$(id -u):700"
test ! -e "$setup_root/apt-update.log"
test ! -e "$setup_root/apt-install.log"

sudo apt-get update > "$setup_root/apt-update.log" 2>&1
sudo apt-get install -y \
  build-essential git curl ca-certificates python3 pipx unzip zip xz-utils \
  chromium brotli nodejs npm systemd-timesyncd \
  > "$setup_root/apt-install.log" 2>&1
# Refresh the capability cache if systemd-timedated predated the package, then
# reannounce the package-started synchronizer on D-Bus before status probes.
sudo systemctl restart systemd-timedated.service
sudo timedatectl set-ntp true
sudo systemctl restart systemd-timesyncd.service

attempt=0
while :; do
  timedate_state=$(timedatectl show \
    -p CanNTP -p NTP -p NTPSynchronized --value | paste -sd ' ' -)
  service_state=$(systemctl is-enabled systemd-timesyncd.service 2>/dev/null || true)
  active_state=$(systemctl is-active systemd-timesyncd.service 2>/dev/null || true)
  if [ "$timedate_state" = "yes yes yes" ] \
    && [ "$service_state" = enabled ] && [ "$active_state" = active ]; then
    break
  fi
  attempt=$((attempt + 1))
  test "$attempt" -lt 60
  sleep 1
done
timedatectl status > "$setup_root/timedatectl-status.txt"
timedatectl timesync-status > "$setup_root/timesync-status.txt"
```

This is condition polling, not an arbitrary startup delay. Stop if
`CanNTP`, `NTP`, or `NTPSynchronized` cannot reach `yes`; do not start a
deadline-bound remote proof against an unsynchronized host. Do not add a
third-party APT repository.

### 3. Clone current main, record it, and read pins from that clone

The ongoing onboarding path never asserts that moving `main` still equals the
2026-08-24 baseline. It resolves `main`, clones it over public HTTPS, proves the
clone matches that observation, records the full SHA, and treats checked-in
files in that clone as authority. The historical SHA appears only in the dated
ledger and rollback.

```sh
set -eu
: "${UNSEEING_SETUP_DATE:?export the chosen YYYY-MM-DD setup date}"
reproduction_date=$UNSEEING_SETUP_DATE
test "$reproduction_date" != 2026-08-24
test "$(date -d "$reproduction_date" +%F 2>/dev/null)" = "$reproduction_date"
home_root=$(realpath "$HOME")
setup_root="$home_root/.local/state/unseeing/setup/$reproduction_date"
source_parent="$home_root/src"
repo="$source_parent/unseeing"
export PATH="$HOME/.local/bin:$PATH"
test "$(realpath "$setup_root")" = "$setup_root"
test ! -L "$setup_root" && test "$(stat -c %u:%a "$setup_root")" = "$(id -u):700"
test ! -e "$repo" && test ! -L "$repo"
if [ -e "$source_parent" ] || [ -L "$source_parent" ]; then
  test ! -L "$source_parent" && test -d "$source_parent"
  test "$(stat -c %u "$source_parent")" = "$(id -u)"
else
  mkdir -m 755 "$source_parent"
fi
observed_main=$(git ls-remote https://github.com/cleveralbatraoz/unseeing.git \
  refs/heads/main | awk 'NR == 1 { print $1 }')
test "${#observed_main}" -eq 40
git clone --branch main --no-recurse-submodules \
  https://github.com/cleveralbatraoz/unseeing.git "$repo"
cd "$repo"
test "$(git rev-parse HEAD)" = "$observed_main"
test "$(git remote get-url origin)" = \
  https://github.com/cleveralbatraoz/unseeing.git

ci/verify-superpowers.sh metadata
git submodule update --init --depth 1 -- tools/superpowers
ci/verify-superpowers.sh full
git config --local user.name 'Dmitrii Galchenko'
git config --local user.email dggrus@gmail.com
git config --local core.hooksPath .githooks
test -z "$(git status --short)"

test ! -e "$setup_root/source-main.txt"
printf '%s  refs/heads/main\n' "$observed_main" > "$setup_root/source-main.txt"
sha256sum .godot-version rust/rust-toolchain.toml rust/build-wasm.sh \
  README.md ci/pipeline.sh ci/engine_class_count \
  > "$setup_root/checked-in-pin-files.sha256.txt"
```

Public HTTPS clone/fetch needs no GitHub credential. Pushing still requires the
human user to configure `gh auth login` or an SSH key outside this evidence.

### 4. Install the checksum-verified Godot pin and templates

The download block derives its version and asset names from the cloned
`.godot-version`, obtains the official release checksum list, and retains the
selected exact URLs and hashes as evidence.

```sh
set -eu
: "${UNSEEING_SETUP_DATE:?export the chosen YYYY-MM-DD setup date}"
reproduction_date=$UNSEEING_SETUP_DATE
test "$reproduction_date" != 2026-08-24
test "$(date -d "$reproduction_date" +%F 2>/dev/null)" = "$reproduction_date"
home_root=$(realpath "$HOME")
setup_root="$home_root/.local/state/unseeing/setup/$reproduction_date"
download_root="$setup_root/downloads"
repo="$home_root/src/unseeing"
export PATH="$HOME/.local/bin:$PATH"
test "$(realpath "$setup_root")" = "$setup_root"
test "$(realpath "$download_root")" = "$download_root"
test ! -L "$setup_root" && test ! -L "$download_root"
test "$(stat -c %u:%a "$setup_root")" = "$(id -u):700"
test "$(stat -c %u:%a "$download_root")" = "$(id -u):700"
test "$(realpath "$repo")" = "$repo" && test ! -L "$repo"
cd "$repo"
godot_pin=$(tr -d '\n' < .godot-version)
case "$godot_pin" in *.stable.official) ;; *) exit 2 ;; esac
godot_version=${godot_pin%.stable.official}
release="$godot_version-stable"
editor="Godot_v${release}_linux.x86_64.zip"
templates="Godot_v${release}_export_templates.tpz"
godot_base="https://github.com/godotengine/godot/releases/download/$release"
cd "$download_root"
for output in "$editor" "$templates" SHA512-SUMS.txt; do
  test ! -e "$output" && test ! -L "$output"
done
selected="$setup_root/godot-selected.sha512.txt"
urls="$setup_root/godot-download-urls.txt"
test ! -e "$selected" && test ! -L "$selected"
test ! -e "$urls" && test ! -L "$urls"
curl --fail --location --proto '=https' --tlsv1.2 --remote-name \
  "$godot_base/$editor"
curl --fail --location --proto '=https' --tlsv1.2 --remote-name \
  "$godot_base/$templates"
curl --fail --location --proto '=https' --tlsv1.2 --remote-name \
  "$godot_base/SHA512-SUMS.txt"
grep -F "  $editor" SHA512-SUMS.txt > "$selected"
grep -F "  $templates" SHA512-SUMS.txt >> "$selected"
test "$(wc -l < "$selected")" -eq 2
sha512sum --check "$selected"
printf '%s\n%s\n%s\n' "$godot_base/$editor" "$godot_base/$templates" \
  "$godot_base/SHA512-SUMS.txt" > "$urls"
```

This second fence deliberately repeats its date/root/PATH/input setup. Its
standard-library validator rejects empty archives, absolute or parent-
traversing names, unexpected top-level components and archived symlinks before
extracting anything.

```sh
set -eu
: "${UNSEEING_SETUP_DATE:?export the chosen YYYY-MM-DD setup date}"
reproduction_date=$UNSEEING_SETUP_DATE
test "$reproduction_date" != 2026-08-24
test "$(date -d "$reproduction_date" +%F 2>/dev/null)" = "$reproduction_date"
home_root=$(realpath "$HOME")
setup_root="$home_root/.local/state/unseeing/setup/$reproduction_date"
download_root="$setup_root/downloads"
repo="$home_root/src/unseeing"
export PATH="$HOME/.local/bin:$PATH"
test "$(realpath "$setup_root")" = "$setup_root"
test "$(realpath "$download_root")" = "$download_root"
test ! -L "$setup_root" && test ! -L "$download_root"
test "$(stat -c %u:%a "$setup_root")" = "$(id -u):700"
test "$(stat -c %u:%a "$download_root")" = "$(id -u):700"
test "$(realpath "$repo")" = "$repo" && test ! -L "$repo"
cd "$repo"
godot_pin=$(tr -d '\n' < .godot-version)
case "$godot_pin" in *.stable.official) ;; *) exit 2 ;; esac
godot_version=${godot_pin%.stable.official}
release="$godot_version-stable"
editor="Godot_v${release}_linux.x86_64.zip"
templates="Godot_v${release}_export_templates.tpz"
template_dir="$HOME/.local/share/godot/export_templates/${godot_version}.stable"
cd "$download_root"
test -f "$editor" && test ! -L "$editor"
test -f "$templates" && test ! -L "$templates"
selected="$setup_root/godot-selected.sha512.txt"
test -f "$selected" && test ! -L "$selected"
sha512sum --check "$selected"
python3 - "$editor" "$templates" <<'PY'
from pathlib import Path, PurePosixPath
import stat
import sys
import zipfile

checks = (
    (Path(sys.argv[1]), Path(sys.argv[1]).stem, True),
    (Path(sys.argv[2]), "templates", False),
)
for archive, expected_top, exact_one in checks:
    with zipfile.ZipFile(archive) as opened:
        members = opened.infolist()
        if not members:
            raise SystemExit(f"empty archive: {archive}")
        names = []
        for member in members:
            name = PurePosixPath(member.filename)
            if name.is_absolute() or ".." in name.parts:
                raise SystemExit(f"unsafe member: {member.filename}")
            if not name.parts or name.parts[0] != expected_top:
                raise SystemExit(f"unexpected top-level member: {member.filename}")
            if stat.S_ISLNK(member.external_attr >> 16):
                raise SystemExit(f"symlink member: {member.filename}")
            names.append(member.filename)
        if exact_one and names != [expected_top]:
            raise SystemExit(f"editor archive members: {names!r}")
print("Godot archives: validated")
PY
test ! -e godot-editor-extract && test ! -L godot-editor-extract
test ! -e godot-template-extract && test ! -L godot-template-extract
test ! -e "$HOME/.local/bin/Godot_v${release}_linux.x86_64"
test ! -e "$HOME/.local/bin/godot" && test ! -L "$HOME/.local/bin/godot"
test ! -e "$template_dir" && test ! -L "$template_dir"
for parent in "$HOME/.local" "$HOME/.local/bin" "$HOME/.local/share" \
  "$HOME/.local/share/godot" "$HOME/.local/share/godot/export_templates"; do
  if [ -e "$parent" ] || [ -L "$parent" ]; then
    test ! -L "$parent" && test -d "$parent"
    test "$(stat -c %u "$parent")" = "$(id -u)"
  fi
done
mkdir -m 700 godot-editor-extract godot-template-extract
python3 - "$editor" "$templates" <<'PY'
import sys
import zipfile
with zipfile.ZipFile(sys.argv[1]) as opened:
    opened.extractall("godot-editor-extract")
with zipfile.ZipFile(sys.argv[2]) as opened:
    opened.extractall("godot-template-extract")
PY
install -d -m 755 "$HOME/.local/bin"
test "$(realpath "$HOME/.local/bin")" = "$home_root/.local/bin"
install -m 755 "godot-editor-extract/Godot_v${release}_linux.x86_64" \
  "$HOME/.local/bin/Godot_v${release}_linux.x86_64"
ln -s "Godot_v${release}_linux.x86_64" "$HOME/.local/bin/godot"
install -d -m 755 "$template_dir"
test "$(realpath "$template_dir")" = "$template_dir"
cp -a godot-template-extract/templates/. "$template_dir/"
test -f "$template_dir/version.txt"
test -f "$template_dir/web_release.zip"
test -f "$template_dir/linux_release.x86_64"
godot --version > "$setup_root/godot-version.txt"
```

### 5. Install rustup and both repository-pinned Rust lanes

`~/.cargo` may pre-exist without rustup; Step 1 recorded it. This block reads
stable channel, components and all desktop targets from
`rust/rust-toolchain.toml`, and the Web nightly from `rust/build-wasm.sh`.

```sh
set -eu
: "${UNSEEING_SETUP_DATE:?export the chosen YYYY-MM-DD setup date}"
reproduction_date=$UNSEEING_SETUP_DATE
test "$reproduction_date" != 2026-08-24
test "$(date -d "$reproduction_date" +%F 2>/dev/null)" = "$reproduction_date"
home_root=$(realpath "$HOME")
setup_root="$home_root/.local/state/unseeing/setup/$reproduction_date"
download_root="$setup_root/downloads"
repo="$home_root/src/unseeing"
export PATH="$HOME/.local/bin:$PATH"
test "$(realpath "$setup_root")" = "$setup_root"
test "$(realpath "$download_root")" = "$download_root"
test ! -L "$setup_root" && test ! -L "$download_root"
test "$(stat -c %u:%a "$setup_root")" = "$(id -u):700"
test "$(stat -c %u:%a "$download_root")" = "$(id -u):700"
test "$(realpath "$repo")" = "$repo" && test ! -L "$repo"
test ! -e "$HOME/.rustup" && test ! -L "$HOME/.rustup"
stable_toolchain=$(python3 - "$repo/rust/rust-toolchain.toml" <<'PY'
import sys, tomllib
with open(sys.argv[1], "rb") as source:
    print(tomllib.load(source)["toolchain"]["channel"])
PY
)
python3 - "$repo/rust/rust-toolchain.toml" \
  "$setup_root/rust-stable-components.txt" \
  "$setup_root/rust-stable-targets.txt" <<'PY'
import sys, tomllib
with open(sys.argv[1], "rb") as source:
    toolchain = tomllib.load(source)["toolchain"]
for output, values in ((sys.argv[2], toolchain["components"]),
                       (sys.argv[3], toolchain["targets"])):
    with open(output, "x", encoding="utf-8") as opened:
        opened.write("".join(f"{value}\n" for value in values))
PY
nightly_toolchain=$(python3 - "$repo/rust/build-wasm.sh" <<'PY'
import re, sys
text = open(sys.argv[1], encoding="utf-8").read()
values = set(re.findall(r'^NIGHTLY="([^"]+)"$', text, re.MULTILINE))
if len(values) != 1:
    raise SystemExit("nightly pin is not unique")
print(values.pop())
PY
)
web_target=$(python3 - "$repo/rust/build-wasm.sh" <<'PY'
import re, sys
text = open(sys.argv[1], encoding="utf-8").read()
values = set(re.findall(r'--target ([a-z0-9_-]+)', text))
if len(values) != 1:
    raise SystemExit("Web target pin is not unique")
print(values.pop())
PY
)
printf '%s\n' "$nightly_toolchain" > "$setup_root/rust-web-nightly.txt"
printf '%s\n' "$web_target" > "$setup_root/rust-web-target.txt"
cd "$download_root"
test ! -e rustup-init && test ! -L rustup-init
test ! -e rustup-init.sha256 && test ! -L rustup-init.sha256
rustup_base=https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu
printf '%s\n%s\n' "$rustup_base/rustup-init" \
  "$rustup_base/rustup-init.sha256" > "$setup_root/rustup-download-urls.txt"
curl --fail --location --silent --show-error --proto '=https' --tlsv1.2 \
  --output rustup-init "$rustup_base/rustup-init"
curl --fail --location --silent --show-error --proto '=https' --tlsv1.2 \
  --output rustup-init.sha256 "$rustup_base/rustup-init.sha256"
grep -Eq '^[0-9a-f]{64} \*?\./rustup-init$' rustup-init.sha256
sha256sum --check rustup-init.sha256
chmod 0700 rustup-init
sha256sum rustup-init > "$setup_root/rustup-installer.sha256.txt"
./rustup-init -y --profile minimal --default-toolchain none
. "$HOME/.cargo/env"
rustup toolchain install "$stable_toolchain" --profile minimal
while IFS= read -r component; do
  rustup component add "$component" --toolchain "$stable_toolchain"
done < "$setup_root/rust-stable-components.txt"
while IFS= read -r target; do
  rustup target add "$target" --toolchain "$stable_toolchain"
done < "$setup_root/rust-stable-targets.txt"
rustup toolchain install "$nightly_toolchain" --profile minimal
rustup component add rust-src --toolchain "$nightly_toolchain"
rustup target add "$web_target" --toolchain "$nightly_toolchain"
rustup --version > "$setup_root/rustup-version.txt"
rustup toolchain list > "$setup_root/rustup-toolchains.txt"
for toolchain in "$stable_toolchain" "$nightly_toolchain"; do
  printf '[%s]\n' "$toolchain"
  rustup component list --toolchain "$toolchain" --installed
done > "$setup_root/rustup-components.txt"
for toolchain in "$stable_toolchain" "$nightly_toolchain"; do
  printf '[%s]\n' "$toolchain"
  rustup target list --toolchain "$toolchain" --installed
done > "$setup_root/rustup-targets.txt"
```

The rustup installer may append `. "$HOME/.cargo/env"` to `~/.profile` and
`~/.bashrc`; the after-state section records hashes and only this public,
installer-owned line. Debian `rustc`/`cargo` packages remain untouched.

### 6. Install and hash the resolved gdtoolkit 4.x wheel

The supported expression is checked in as `4.*`. The resolved version is read
from the installed pipx environment, range-checked, and used for the exact
wheel download. The historical 4.5.0 value is not assumed here.

```sh
set -eu
: "${UNSEEING_SETUP_DATE:?export the chosen YYYY-MM-DD setup date}"
reproduction_date=$UNSEEING_SETUP_DATE
test "$reproduction_date" != 2026-08-24
test "$(date -d "$reproduction_date" +%F 2>/dev/null)" = "$reproduction_date"
home_root=$(realpath "$HOME")
setup_root="$home_root/.local/state/unseeing/setup/$reproduction_date"
download_root="$setup_root/downloads"
repo="$home_root/src/unseeing"
export PATH="$HOME/.local/bin:$PATH"
test "$(realpath "$setup_root")" = "$setup_root"
test "$(realpath "$download_root")" = "$download_root"
test ! -L "$setup_root" && test ! -L "$download_root"
test "$(stat -c %u:%a "$setup_root")" = "$(id -u):700"
test "$(stat -c %u:%a "$download_root")" = "$(id -u):700"
test "$(realpath "$repo")" = "$repo" && test ! -L "$repo"
cd "$repo"
gdtoolkit_range=$(python3 - README.md ci/pipeline.sh <<'PY'
import re, sys
text = "\n".join(open(path, encoding="utf-8").read() for path in sys.argv[1:])
values = set(re.findall(r'gdtoolkit==([0-9]+\.\*)', text))
if len(values) != 1:
    raise SystemExit("gdtoolkit range is not unique")
print(values.pop())
PY
)
gdtoolkit_major=${gdtoolkit_range%.*}
pipx install "gdtoolkit==$gdtoolkit_range"
gdtoolkit_python="$HOME/.local/share/pipx/venvs/gdtoolkit/bin/python"
test -x "$gdtoolkit_python"
gdtoolkit_version=$("$gdtoolkit_python" - <<'PY'
from importlib.metadata import version
print(version("gdtoolkit"))
PY
)
case "$gdtoolkit_version" in "$gdtoolkit_major".*) ;; *) exit 2 ;; esac
wheel_root="$download_root/gdtoolkit-wheel"
test ! -e "$wheel_root" && test ! -L "$wheel_root"
mkdir -m 700 "$wheel_root"
"$gdtoolkit_python" -m pip download --no-deps --dest "$wheel_root" \
  "gdtoolkit==$gdtoolkit_version"
test "$(find "$wheel_root" -maxdepth 1 -type f | wc -l)" -eq 1
printf '%s\n' "$gdtoolkit_version" > "$setup_root/gdtoolkit-version.txt"
sha256sum "$wheel_root"/* > "$setup_root/gdtoolkit-wheel.sha256.txt"
gdformat --version > "$setup_root/gdformat-version.txt"
gdlint --version > "$setup_root/gdlint-version.txt"
```

Pipx owns the environment and the `gdformat`, `gdlint`, `gd2py`, `gdparse`,
and `gdradon` links under `~/.local/bin`.

### 7. Install the repository-pinned, non-global emsdk checkout

```sh
set -eu
: "${UNSEEING_SETUP_DATE:?export the chosen YYYY-MM-DD setup date}"
reproduction_date=$UNSEEING_SETUP_DATE
test "$reproduction_date" != 2026-08-24
test "$(date -d "$reproduction_date" +%F 2>/dev/null)" = "$reproduction_date"
home_root=$(realpath "$HOME")
setup_root="$home_root/.local/state/unseeing/setup/$reproduction_date"
repo="$home_root/src/unseeing"
emsdk_root="$home_root/emsdk"
export PATH="$HOME/.local/bin:$PATH"
test "$(realpath "$setup_root")" = "$setup_root"
test ! -L "$setup_root" && test "$(stat -c %u:%a "$setup_root")" = "$(id -u):700"
test "$(realpath "$repo")" = "$repo" && test ! -L "$repo"
test ! -e "$emsdk_root" && test ! -L "$emsdk_root"
emsdk_version=$(python3 - "$repo/rust/build-wasm.sh" <<'PY'
import re, sys
text = open(sys.argv[1], encoding="utf-8").read()
values = set(re.findall(r'emsdk install ([0-9]+(?:\.[0-9]+)+)', text))
if len(values) != 1:
    raise SystemExit("emsdk pin is not unique")
print(values.pop())
PY
)
resolved=$(git ls-remote --tags https://github.com/emscripten-core/emsdk.git \
  "refs/tags/$emsdk_version^{}" | awk 'NR == 1 { print $1 }')
if [ -z "$resolved" ]; then
  resolved=$(git ls-remote --tags https://github.com/emscripten-core/emsdk.git \
    "refs/tags/$emsdk_version" | awk 'NR == 1 { print $1 }')
fi
test "${#resolved}" -eq 40
git clone --branch "$emsdk_version" --depth 1 \
  https://github.com/emscripten-core/emsdk.git "$emsdk_root"
cd "$emsdk_root"
test "$(git remote get-url origin)" = https://github.com/emscripten-core/emsdk.git
test "$(git rev-parse HEAD)" = "$resolved"
./emsdk install "$emsdk_version"
./emsdk activate "$emsdk_version"
test ! -e "$HOME/.emscripten" && test ! -L "$HOME/.emscripten"
printf '%s\t%s\n' "$emsdk_version" "$resolved" \
  > "$setup_root/emsdk-version-commit.tsv"
(EMSDK_QUIET=1 . ./emsdk_env.sh && emcc --version) \
  > "$setup_root/emcc-version.txt"
```

Do not use `--global` or add emsdk to startup files. `rust/build-wasm.sh`
sources `~/emsdk/emsdk_env.sh` only for its own command.

### 8. Capture post-state, deltas, and installed-root manifests

This fence records actual results rather than predicting a package count.

```sh
set -eu
: "${UNSEEING_SETUP_DATE:?export the chosen YYYY-MM-DD setup date}"
reproduction_date=$UNSEEING_SETUP_DATE
test "$reproduction_date" != 2026-08-24
test "$(date -d "$reproduction_date" +%F 2>/dev/null)" = "$reproduction_date"
home_root=$(realpath "$HOME")
setup_root="$home_root/.local/state/unseeing/setup/$reproduction_date"
export PATH="$HOME/.local/bin:$PATH"
test "$(realpath "$setup_root")" = "$setup_root"
test ! -L "$setup_root" && test "$(stat -c %u:%a "$setup_root")" = "$(id -u):700"
for output in after-packages.tsv after-manual-packages.txt \
  after-apt-source-hashes.txt after-startup-hashes.txt package-delta.tsv \
  manual-packages-added.txt manual-packages-removed.txt apt-source-result.txt \
  rustup-startup-lines.txt chromium-version.txt brotli-version.txt; do
  test ! -e "$setup_root/$output" && test ! -L "$setup_root/$output"
done
dpkg-query -W -f='${binary:Package}\t${Version}\n' | LC_ALL=C sort \
  > "$setup_root/after-packages.tsv"
apt-mark showmanual | LC_ALL=C sort > "$setup_root/after-manual-packages.txt"
for path in /etc/apt/sources.list /etc/apt/sources.list.d; do
  if [ -f "$path" ]; then
    sha256sum "$path"
  elif [ -d "$path" ]; then
    find "$path" -type f -print0 | LC_ALL=C sort -z | xargs -0 -r sha256sum
  else
    printf 'ABSENT\t%s\n' "$path"
  fi
done > "$setup_root/after-apt-source-hashes.txt"
for path in "$HOME/.profile" "$HOME/.bashrc"; do
  if [ -f "$path" ]; then sha256sum "$path"; else printf 'ABSENT\t%s\n' "$path"; fi
done > "$setup_root/after-startup-hashes.txt"
grep -H -F -x '. "$HOME/.cargo/env"' "$HOME/.profile" "$HOME/.bashrc" \
  > "$setup_root/rustup-startup-lines.txt"
test "$(wc -l < "$setup_root/rustup-startup-lines.txt")" -eq 2
chromium --version > "$setup_root/chromium-version.txt"
brotli --version > "$setup_root/brotli-version.txt" 2>&1
python3 - "$setup_root/before-packages.tsv" "$setup_root/after-packages.tsv" \
  "$setup_root/package-delta.tsv" <<'PY'
import sys
def load(path):
    result = {}
    for line in open(path, encoding="utf-8"):
        name, version = line.rstrip("\n").split("\t", 1)
        result[name] = version
    return result
before, after = load(sys.argv[1]), load(sys.argv[2])
with open(sys.argv[3], "x", encoding="utf-8") as output:
    for name in sorted(before.keys() | after.keys(), key=lambda value: value.encode()):
        old, new = before.get(name), after.get(name)
        if old == new:
            continue
        state = "ADDED" if old is None else "REMOVED" if new is None else "CHANGED"
        output.write(f"{state}\t{name}\t{old or '-'}\t{new or '-'}\n")
PY
comm -13 "$setup_root/before-manual-packages.txt" \
  "$setup_root/after-manual-packages.txt" \
  > "$setup_root/manual-packages-added.txt"
comm -23 "$setup_root/before-manual-packages.txt" \
  "$setup_root/after-manual-packages.txt" \
  > "$setup_root/manual-packages-removed.txt"
if cmp -s "$setup_root/before-apt-source-hashes.txt" \
  "$setup_root/after-apt-source-hashes.txt"; then
  printf 'UNCHANGED\n' > "$setup_root/apt-source-result.txt"
else
  printf 'CHANGED: stop and review the two source-hash files\n' \
    > "$setup_root/apt-source-result.txt"
  exit 2
fi
```

The installed-root manifest fence walks without following symlinks, rejects
unexpected owners and unsafe names, records type/mode/uid/gid/size/link target,
hashes every regular file, and hashes each manifest pair. Outputs live outside
the installation roots they describe.

```sh
set -eu
: "${UNSEEING_SETUP_DATE:?export the chosen YYYY-MM-DD setup date}"
reproduction_date=$UNSEEING_SETUP_DATE
test "$reproduction_date" != 2026-08-24
test "$(date -d "$reproduction_date" +%F 2>/dev/null)" = "$reproduction_date"
home_root=$(realpath "$HOME")
setup_root="$home_root/.local/state/unseeing/setup/$reproduction_date"
repo="$home_root/src/unseeing"
export PATH="$HOME/.local/bin:$PATH"
test "$(realpath "$setup_root")" = "$setup_root"
test ! -L "$setup_root" && test "$(stat -c %u:%a "$setup_root")" = "$(id -u):700"
test "$(realpath "$repo")" = "$repo" && test ! -L "$repo"
godot_pin=$(tr -d '\n' < "$repo/.godot-version")
case "$godot_pin" in *.stable.official) ;; *) exit 2 ;; esac
godot_version=${godot_pin%.stable.official}
release="$godot_version-stable"
manifest_root="$setup_root/installed-roots"
test ! -e "$manifest_root" && test ! -L "$manifest_root"
mkdir -m 700 "$manifest_root"
python3 - "$manifest_root" \
  "godot-editor=$HOME/.local/bin/Godot_v${release}_linux.x86_64" \
  "godot-link=$HOME/.local/bin/godot" \
  "godot-templates=$HOME/.local/share/godot/export_templates/${godot_version}.stable" \
  "cargo=$HOME/.cargo" "rustup=$HOME/.rustup" \
  "gdtoolkit=$HOME/.local/share/pipx/venvs/gdtoolkit" \
  "emsdk=$HOME/emsdk" <<'PY'
import hashlib, os, stat, sys
from pathlib import Path

output_root = Path(sys.argv[1])
expected_uid = os.getuid()

def digest(path):
    value = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            value.update(block)
    return value.hexdigest()

def write_exclusive(path, data):
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    os.fchmod(descriptor, 0o600)
    with os.fdopen(descriptor, "wb") as opened:
        opened.write(data)
        opened.flush()
        os.fsync(opened.fileno())

for item in sys.argv[2:]:
    slug, raw = item.split("=", 1)
    root = Path(raw)
    if not root.exists() and not root.is_symlink():
        raise SystemExit(f"installed root absent: {slug}")
    pending = [(Path("."), root)]
    rows, files = [], []
    while pending:
        relative, path = pending.pop()
        metadata = path.lstat()
        if metadata.st_uid != expected_uid:
            raise SystemExit(f"owner mismatch: {path}")
        shown = relative.as_posix()
        if any(character in shown for character in "\t\r\n"):
            raise SystemExit(f"unsafe name: {path}")
        mode = stat.S_IMODE(metadata.st_mode)
        if stat.S_ISDIR(metadata.st_mode):
            kind, link = "d", "-"
            children = sorted(path.iterdir(), key=lambda child: os.fsencode(child.name), reverse=True)
            pending.extend((relative / child.name, child) for child in children)
        elif stat.S_ISREG(metadata.st_mode):
            kind, link = "f", "-"
            files.append((os.fsencode(shown), f"{digest(path)}  {shown}\n"))
        elif stat.S_ISLNK(metadata.st_mode):
            kind, link = "l", os.readlink(path)
            if any(character in link for character in "\t\r\n"):
                raise SystemExit(f"unsafe link: {path}")
        else:
            raise SystemExit(f"unsupported type: {path}")
        rows.append((os.fsencode(shown), f"{kind}\t{mode:04o}\t{metadata.st_uid}\t{metadata.st_gid}\t{metadata.st_size}\t{shown}\t{link}\n"))
    metadata_path = output_root / f"manifest-{slug}-metadata.tsv"
    files_path = output_root / f"manifest-{slug}-files.sha256.txt"
    root_path = output_root / f"manifest-{slug}-root.txt"
    write_exclusive(metadata_path, "".join(row for _, row in sorted(rows)).encode())
    write_exclusive(files_path, "".join(row for _, row in sorted(files)).encode())
    write_exclusive(root_path, f"{root}\n".encode())
    summary = (f"{digest(metadata_path)}  {metadata_path.name}\n"
               f"{digest(files_path)}  {files_path.name}\n"
               f"{digest(root_path)}  {root_path.name}\n")
    write_exclusive(output_root / f"manifest-{slug}.self-sha256.txt", summary.encode())
os.fsync(os.open(output_root, os.O_RDONLY | os.O_DIRECTORY))
PY
```

### 9. Run and preserve all four proof gates

`CARGO_BUILD_JOBS=4` is a command-local resource limit for a small workstation,
not a repository or startup setting. The runner below records each literal
command, complete combined log, start/end/duration and real exit status before
returning that status. A failed gate therefore remains evidence and stops the
sequence without being flattened by redirection.

```sh
set -eu
: "${UNSEEING_SETUP_DATE:?export the chosen YYYY-MM-DD setup date}"
reproduction_date=$UNSEEING_SETUP_DATE
test "$reproduction_date" != 2026-08-24
test "$(date -d "$reproduction_date" +%F 2>/dev/null)" = "$reproduction_date"
home_root=$(realpath "$HOME")
setup_root="$home_root/.local/state/unseeing/setup/$reproduction_date"
repo="$home_root/src/unseeing"
log_root="$setup_root/build-gates"
export PATH="$HOME/.local/bin:$PATH"
test "$(realpath "$setup_root")" = "$setup_root"
test ! -L "$setup_root" && test "$(stat -c %u:%a "$setup_root")" = "$(id -u):700"
test "$(realpath "$repo")" = "$repo" && test ! -L "$repo"
test ! -e "$log_root" && test ! -L "$log_root"
mkdir -m 700 "$log_root"
. "$HOME/.cargo/env"
cd "$repo"

run_gate() {
  name=$1
  literal=$2
  shift 2
  log="$log_root/$name.log"
  status_record="$log_root/$name.status.txt"
  test ! -e "$log" && test ! -L "$log"
  test ! -e "$status_record" && test ! -L "$status_record"
  started=$(date +%s)
  set +e
  "$@" > "$log" 2>&1
  status=$?
  set -e
  ended=$(date +%s)
  printf 'command=%s\nstarted_epoch=%s\nended_epoch=%s\nduration_seconds=%s\nexit_status=%s\n' \
    "$literal" "$started" "$ended" "$((ended - started))" "$status" \
    > "$status_record"
  python3 - "$log" "$status_record" <<'PY'
import os, sys
for path in sys.argv[1:]:
    descriptor = os.open(path, os.O_RDONLY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)
PY
  test "$status" -eq 0
}

run_gate bootstrap 'CARGO_BUILD_JOBS=4 tools/bootstrap.sh' \
  env CARGO_BUILD_JOBS=4 tools/bootstrap.sh
run_gate checks 'CARGO_BUILD_JOBS=4 SKIP_EXPORT=1 ci/pipeline.sh' \
  env CARGO_BUILD_JOBS=4 SKIP_EXPORT=1 ci/pipeline.sh
run_gate web 'CARGO_BUILD_JOBS=4 ci/pipeline.sh' \
  env CARGO_BUILD_JOBS=4 ci/pipeline.sh
run_gate linux 'CARGO_BUILD_JOBS=4 tools/export_linux.sh "Linux x86_64" build/linux/unseeing' \
  env CARGO_BUILD_JOBS=4 tools/export_linux.sh \
    'Linux x86_64' build/linux/unseeing

test -z "$(git status --short)"
git rev-parse HEAD > "$log_root/source-head.txt"
git status --short --ignored > "$log_root/git-status-ignored.txt"
python3 - "$repo" "$log_root/artifact-manifest.tsv" <<'PY'
import hashlib, os, stat, sys
from pathlib import Path
repo, output = Path(sys.argv[1]), Path(sys.argv[2])
paths = [repo / "game/build/linux.log", repo / "game/build/linux/unseeing",
         repo / "game/build/linux/libunseeing_core.so"]
paths += list((repo / "game/build/web").rglob("*"))
paths = [path for path in paths if path.is_file() and not path.is_symlink()]
if len(paths) < 4:
    raise SystemExit("artifact set incomplete")
rows = []
for path in paths:
    metadata = path.lstat()
    if not stat.S_ISREG(metadata.st_mode):
        raise SystemExit(f"artifact is not regular: {path}")
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    relative = path.relative_to(repo).as_posix()
    rows.append((relative.encode(), f"{relative}\t{metadata.st_size}\t{digest}\n"))
descriptor = os.open(output, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
os.fchmod(descriptor, 0o600)
with os.fdopen(descriptor, "wb") as opened:
    opened.write("".join(row for _, row in sorted(rows)).encode())
    opened.flush()
    os.fsync(opened.fileno())
PY
sha256sum "$log_root"/*.log "$log_root"/*.status.txt \
  "$log_root/artifact-manifest.tsv" > "$log_root/proof-files.sha256.txt"

post_build_root="$setup_root/post-build-user-state"
test ! -e "$post_build_root" && test ! -L "$post_build_root"
mkdir -m 700 "$post_build_root"
python3 - "$post_build_root" \
  "cargo=$HOME/.cargo=hash" "rustup=$HOME/.rustup=hash" \
  "godot-cache=$HOME/.cache/godot=hash" \
  "godot-config=$HOME/.config/godot=hash" \
  "unseeing-userdata=$HOME/.local/share/godot/app_userdata/Unseeing=hash" \
  "gdtoolkit-cache=$HOME/.cache/gdtoolkit=hash" \
  "chromium-config=$HOME/.config/chromium=hash" \
  "pki=$HOME/.local/share/pki=metadata-only" <<'PY'
import hashlib, os, stat, sys
from pathlib import Path

output_root, expected_uid = Path(sys.argv[1]), os.getuid()

def digest(path):
    value = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            value.update(block)
    return value.hexdigest()

def write_exclusive(path, payload):
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    os.fchmod(descriptor, 0o600)
    with os.fdopen(descriptor, "wb") as opened:
        opened.write(payload)
        opened.flush()
        os.fsync(opened.fileno())

for item in sys.argv[2:]:
    slug, raw, policy = item.rsplit("=", 2)
    root = Path(raw)
    metadata_rows, file_rows = [], []
    if not root.exists() and not root.is_symlink():
        metadata_rows.append((b".", f"ABSENT\t{root}\n"))
    else:
        pending = [(Path("."), root)]
        while pending:
            relative, path = pending.pop()
            metadata = path.lstat()
            if metadata.st_uid != expected_uid:
                raise SystemExit(f"post-build owner mismatch: {path}")
            shown = relative.as_posix()
            if any(character in shown for character in "\t\r\n"):
                raise SystemExit(f"unsafe post-build name: {path}")
            mode, link = stat.S_IMODE(metadata.st_mode), "-"
            if stat.S_ISDIR(metadata.st_mode):
                kind = "d"
                children = sorted(path.iterdir(), key=lambda child: os.fsencode(child.name), reverse=True)
                pending.extend((relative / child.name, child) for child in children)
            elif stat.S_ISREG(metadata.st_mode):
                kind = "f"
                if policy == "hash":
                    file_rows.append((os.fsencode(shown), f"{digest(path)}  {shown}\n"))
            elif stat.S_ISLNK(metadata.st_mode):
                kind, link = "l", os.readlink(path)
                if any(character in link for character in "\t\r\n"):
                    raise SystemExit(f"unsafe post-build link: {path}")
            else:
                raise SystemExit(f"unsupported post-build type: {path}")
            metadata_rows.append((os.fsencode(shown), f"{kind}\t{mode:04o}\t{metadata.st_uid}\t{metadata.st_gid}\t{metadata.st_size}\t{shown}\t{link}\n"))
    metadata_path = output_root / f"{slug}-metadata.tsv"
    files_path = output_root / f"{slug}-files.sha256.txt"
    root_path = output_root / f"{slug}-root-policy.txt"
    write_exclusive(metadata_path, "".join(row for _, row in sorted(metadata_rows)).encode())
    write_exclusive(files_path, "".join(row for _, row in sorted(file_rows)).encode())
    write_exclusive(root_path, f"root={root}\npolicy={policy}\n".encode())
    summary = (f"policy={policy}\n{digest(metadata_path)}  {metadata_path.name}\n"
               f"{digest(files_path)}  {files_path.name}\n"
               f"{digest(root_path)}  {root_path.name}\n")
    write_exclusive(output_root / f"{slug}.self-sha256.txt", summary.encode())
parent = os.open(output_root, os.O_RDONLY | os.O_DIRECTORY)
os.fsync(parent)
os.close(parent)
PY
```

Do not set `SKIP_EXPORT` or `SKIP_SMOKE` on the full Web gate. The post-build
walk distinguishes installed-root state from subsequently generated Cargo,
rustup, Godot, gdtoolkit and Chromium state. PKI is metadata-only because that
shared location may contain sensitive material.

### 10. Remove only download scratch and seal the future-run evidence

This is the complete guarded manual procedure; there is no reusable untracked
helper that a future setup is expected to find. Task 5's ignored, hard-coded
evidence helper belongs only to the dated 2026-08-24 proof. Its reviewed local
source is
`.superpowers/sdd/2026-08-24-hp-local-development-setup/task-5-cleanup-seal.py`
at SHA-256
`0bd912cc0ccd3b43c5c69e1d28c3ba89b0c8f9cce4b14e232a009777ce6af0f7`;
the table in the ledger truthfully leaves its live copy/run result unclaimed.
First write and fsync a complete hash inventory of the exact download child,
revalidate its canonical owner/mode/type, remove only that child, prove
absence, and record the result. Then seal every remaining regular evidence
file. The manifest excludes exactly itself and its digest, so the construction
is non-self-referential; the digest is the last evidence-root write.

```sh
set -eu
: "${UNSEEING_SETUP_DATE:?export the chosen YYYY-MM-DD setup date}"
reproduction_date=$UNSEEING_SETUP_DATE
test "$reproduction_date" != 2026-08-24
test "$(date -d "$reproduction_date" +%F 2>/dev/null)" = "$reproduction_date"
home_root=$(realpath "$HOME")
setup_root="$home_root/.local/state/unseeing/setup/$reproduction_date"
download_root="$setup_root/downloads"
inventory="$setup_root/downloads-pre-cleanup.sha256.txt"
cleanup_record="$setup_root/downloads-cleanup.json"
final_manifest="$setup_root/final-evidence-seal.sha256.txt"
final_digest="$setup_root/final-evidence-seal.digest.txt"
export PATH="$HOME/.local/bin:$PATH"
test "$(realpath "$setup_root")" = "$setup_root"
test "$(realpath "$download_root")" = "$download_root"
test "$download_root" = \
  "$home_root/.local/state/unseeing/setup/$reproduction_date/downloads"
test ! -L "$setup_root" && test -d "$setup_root"
test ! -L "$download_root" && test -d "$download_root"
test "$(stat -c %u:%a "$setup_root")" = "$(id -u):700"
test "$(stat -c %u:%a "$download_root")" = "$(id -u):700"
for output in "$inventory" "$cleanup_record" "$final_manifest" "$final_digest"; do
  test ! -e "$output" && test ! -L "$output"
done
python3 - "$download_root" "$inventory" <<'PY'
import hashlib, os, stat, sys
from pathlib import Path
root, output = Path(sys.argv[1]), Path(sys.argv[2])
uid, rows = os.getuid(), []
for path in root.rglob("*"):
    metadata = path.lstat()
    relative = path.relative_to(root.parent).as_posix()
    if any(character in relative for character in "\r\n"):
        raise SystemExit(f"unsafe download name: {path}")
    if metadata.st_uid != uid:
        raise SystemExit(f"download owner mismatch: {path}")
    if stat.S_ISDIR(metadata.st_mode):
        continue
    if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISREG(metadata.st_mode):
        raise SystemExit(f"unsupported download type: {path}")
    value = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            value.update(block)
    rows.append((relative.encode(), f"{value.hexdigest()}  {relative}\n"))
descriptor = os.open(output, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
os.fchmod(descriptor, 0o600)
with os.fdopen(descriptor, "wb") as opened:
    opened.write("".join(row for _, row in sorted(rows)).encode())
    opened.flush()
    os.fsync(opened.fileno())
parent = os.open(output.parent, os.O_RDONLY | os.O_DIRECTORY)
os.fsync(parent)
os.close(parent)
PY
test "$(realpath "$setup_root")" = "$setup_root"
test "$(realpath "$download_root")" = "$download_root"
test ! -L "$download_root" && test -d "$download_root"
test "$(stat -c %u:%a "$download_root")" = "$(id -u):700"
rm -rf -- "$download_root"
test ! -e "$download_root" && test ! -L "$download_root"
python3 - "$cleanup_record" "$inventory" "$download_root" <<'PY'
import hashlib, json, os, sys
from pathlib import Path
record, inventory, target = map(Path, sys.argv[1:])
payload = {
    "command": f"rm -rf -- {target}",
    "inventory": inventory.name,
    "inventory_entries": len(inventory.read_bytes().splitlines()),
    "inventory_sha256": hashlib.sha256(inventory.read_bytes()).hexdigest(),
    "owner_uid": os.getuid(),
    "result": "absent",
    "schema": "unseeing.setup.download-cleanup.v1",
    "target": str(target),
}
descriptor = os.open(record, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
os.fchmod(descriptor, 0o600)
with os.fdopen(descriptor, "wb") as opened:
    opened.write((json.dumps(payload, sort_keys=True, separators=(",", ":")) + "\n").encode())
    opened.flush()
    os.fsync(opened.fileno())
parent = os.open(record.parent, os.O_RDONLY | os.O_DIRECTORY)
os.fsync(parent)
os.close(parent)
PY
python3 - "$setup_root" "$final_manifest" "$final_digest" <<'PY'
import hashlib, os, stat, sys
from pathlib import Path
root, manifest, digest_record = map(Path, sys.argv[1:])
excluded = {manifest.name, digest_record.name}
uid = os.getuid()

def scan():
    rows = []
    for path in root.rglob("*"):
        metadata = path.lstat()
        relative = path.relative_to(root).as_posix()
        if any(character in relative for character in "\r\n"):
            raise SystemExit(f"unsafe evidence name: {path}")
        if stat.S_ISLNK(metadata.st_mode):
            raise SystemExit(f"evidence symlink rejected: {path}")
        if stat.S_ISDIR(metadata.st_mode):
            continue
        if not stat.S_ISREG(metadata.st_mode) or metadata.st_uid != uid:
            raise SystemExit(f"unsupported evidence file: {path}")
        if relative in excluded:
            continue
        value = hashlib.sha256()
        with path.open("rb") as source:
            for block in iter(lambda: source.read(1024 * 1024), b""):
                value.update(block)
        rows.append((relative.encode(), f"{value.hexdigest()}  {relative}\n"))
    return "".join(row for _, row in sorted(rows)).encode()

payload = scan()
descriptor = os.open(manifest, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
os.fchmod(descriptor, 0o600)
with os.fdopen(descriptor, "wb") as opened:
    opened.write(payload)
    opened.flush()
    os.fsync(opened.fileno())
digest = hashlib.sha256(payload).hexdigest()
descriptor = os.open(digest_record, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
os.fchmod(descriptor, 0o600)
with os.fdopen(descriptor, "wb") as opened:
    opened.write(f"{digest}  {manifest.name}\n".encode())
    opened.flush()
    os.fsync(opened.fileno())
parent = os.open(root, os.O_RDONLY | os.O_DIRECTORY)
os.fsync(parent)
os.close(parent)
# No evidence-root write is permitted below this line.
if manifest.read_bytes() != payload or scan() != payload:
    raise SystemExit("final manifest regeneration mismatch")
if digest_record.read_bytes() != f"{digest}  {manifest.name}\n".encode():
    raise SystemExit("final digest mismatch")
print(f"FINAL_SEAL entries={len(payload.splitlines())} sha256={digest}")
PY
```

## Daily authoring and verification

```sh
set -eu
: "${UNSEEING_TASK_BRANCH:?export the simple branch name of an existing isolated task worktree}"
branch=$UNSEEING_TASK_BRANCH
case "$branch" in
  *[!A-Za-z0-9._-]*|'') exit 2 ;;
esac
durable=$(realpath "$HOME/src/unseeing")
worktree="$durable/.worktrees/$branch"
export PATH="$HOME/.local/bin:$PATH"
test "$durable" = "$HOME/src/unseeing" && test ! -L "$durable"
test "$(realpath "$worktree")" = "$worktree" && test ! -L "$worktree"
test "$(git -C "$worktree" rev-parse --show-toplevel)" = "$worktree"
test "$(git -C "$worktree" symbolic-ref --short HEAD)" = "$branch"
test "$(realpath "$(git -C "$worktree" rev-parse --git-common-dir)")" = \
  "$durable/.git"
. "$HOME/.cargo/env"
cd "$worktree"
git status --short --branch
tools/run_game.sh --windowed
```

Create or select that task worktree through the project worktree workflow;
never author or run from the durable primary. Open `game/project.godot` from
the selected task worktree in the installed Godot editor to author levels.
`game/` is the only project; do not create a Linux-specific copy. Use editor-
authored scenes and registered Rust nodes, not shipped GDScript. The editor
tour is `docs/opening-in-godot.md`.

Useful focused checks are:

```sh
set -eu
: "${UNSEEING_TASK_BRANCH:?export the simple branch name of an existing isolated task worktree}"
branch=$UNSEEING_TASK_BRANCH
case "$branch" in
  *[!A-Za-z0-9._-]*|'') exit 2 ;;
esac
durable=$(realpath "$HOME/src/unseeing")
worktree="$durable/.worktrees/$branch"
export PATH="$HOME/.local/bin:$PATH"
test "$durable" = "$HOME/src/unseeing" && test ! -L "$durable"
test "$(realpath "$worktree")" = "$worktree" && test ! -L "$worktree"
test "$(git -C "$worktree" rev-parse --show-toplevel)" = "$worktree"
test "$(git -C "$worktree" symbolic-ref --short HEAD)" = "$branch"
test "$(realpath "$(git -C "$worktree" rev-parse --git-common-dir)")" = \
  "$durable/.git"
. "$HOME/.cargo/env"
cd "$worktree"
cargo test --manifest-path rust/Cargo.toml
SKIP_EXPORT=1 ci/pipeline.sh
ci/pipeline.sh
```

The game normally boots fullscreen. For a deterministic windowed diagnostic,
create ignored `game/override.cfg` only for the run and always remove that exact
file afterward; never commit it.

## Godot 4.7.1 editor and pinned godot-mcp workflow

The structured editor loop is optional developer tooling, not a game or build
dependency. Godot remains pinned by `.godot-version`; `.mcp.json` and
`tools/setup-mcp.sh` both pin `@satelliteoflove/godot-mcp@4.1.0`.
`GODOT_MCP_VERSION` is deliberately not an override: the installer rejects
any present value before invoking `npx`, so the client and addon cannot drift.
`game/addons/godot_mcp/` is project-relative, ignored and export-excluded.
Consequently every task worktree that uses the loop must build its own native
library and install its own addon. Never enable it in the durable primary
checkout.

The complete interactive semantics are owned by
[`docs/superpowers/mcp/godot-mcp-loop.md`](superpowers/mcp/godot-mcp-loop.md).
The steps here are the onboarding path: install, capture, Enable, exercise,
restore, and optionally uninstall.

### Create and provision the isolated worktree

Choose one unused branch name explicitly; do not reuse a prior MCP worktree.
This example deliberately avoids an automatically generated name:

```sh
set -eu
: "${UNSEEING_MCP_BRANCH:?export one unused simple branch name, for example mcp-level-check-2026-09-17}"
branch=$UNSEEING_MCP_BRANCH
case "$branch" in
  *[!A-Za-z0-9._-]*|'') exit 2 ;;
esac
durable=$(realpath "$HOME/src/unseeing")
worktree="$durable/.worktrees/$branch"
export PATH="$HOME/.local/bin:$PATH"
test "$durable" = "$HOME/src/unseeing"
test ! -L "$durable" && test -d "$durable"
test -z "$(git -C "$durable" status --short)"
test "$(git -C "$durable" symbolic-ref --short HEAD)" = main
git -C "$durable" check-ignore -q .worktrees/probe
test ! -e "$worktree" && test ! -L "$worktree"
test -z "$(git -C "$durable" branch --list "$branch")"
git -C "$durable" worktree add -b "$branch" "$worktree" main
test "$(realpath "$worktree")" = "$worktree"
test -z "$(git -C "$worktree" status --short)"
```

Node 20 or newer is required only for this developer tool. The registry query
below verifies the reviewed integrity before the checked-in installer invokes
`npx`; it is the normal online onboarding path, not the special offline dated
proof. The addon manifest is ignored evidence for guarded removal, not a
helper or a tracked artifact.

```sh
set -eu
: "${UNSEEING_MCP_BRANCH:?export the chosen MCP branch name}"
branch=$UNSEEING_MCP_BRANCH
case "$branch" in
  *[!A-Za-z0-9._-]*|'') exit 2 ;;
esac
worktree="$HOME/src/unseeing/.worktrees/$branch"
addon="$worktree/game/addons/godot_mcp"
manifest="$worktree/.superpowers/mcp-addon-4.1.0-manifest.json"
export PATH="$HOME/.local/bin:$PATH"
test "$(realpath "$worktree")" = "$worktree"
test ! -L "$worktree" && test -d "$worktree"
test -z "$(git -C "$worktree" status --short)"
test ! -e "$addon" && test ! -L "$addon"
test ! -e "$manifest" && test ! -L "$manifest"
. "$HOME/.cargo/env"
cd "$worktree"
CARGO_BUILD_JOBS=4 CARGO_NET_OFFLINE=true RUSTUP_AUTO_INSTALL=0 \
  tools/bootstrap.sh
node_major=$(node --version | sed 's/^v//' | cut -d. -f1)
test "$node_major" -ge 20
grep -Fq '@satelliteoflove/godot-mcp@4.1.0' .mcp.json
grep -Fq 'readonly VERSION=4.1.0' tools/setup-mcp.sh
test "$(npm view @satelliteoflove/godot-mcp@4.1.0 dist.integrity)" = \
  'sha512-uq3Gh5n7fos8vIoXpr32/K7r9tL9eYLbERr+Tolksg3Y+FC5coYEkRkbJ1JktMMhoH/BnGWsWhE5E+XJ/nMEPg=='
/usr/bin/env -u GODOT_MCP_VERSION ./tools/setup-mcp.sh
test "$(sed -n 's/^version="\([^"]*\)"$/\1/p' \
  "$addon/plugin.cfg")" = 4.1.0
git check-ignore -q game/addons/godot_mcp/plugin.cfg
test -z "$(git status --short)"
mkdir -p "$worktree/.superpowers"
test ! -L "$worktree/.superpowers" && test -d "$worktree/.superpowers"
test "$(stat -c %u "$worktree/.superpowers")" = "$(id -u)"
python3 - "$addon" "$manifest" <<'PY'
# addon-manifest-generator-v1: BEGIN
import hashlib
import json
import os
from pathlib import Path
import stat
import sys

root, manifest = map(Path, sys.argv[1:])
uid = os.getuid()
if not root.is_absolute() or not manifest.is_absolute():
    raise SystemExit("addon and manifest paths must be absolute")
if root.as_posix().endswith("/game/addons/godot_mcp") is False:
    raise SystemExit("unexpected addon path")
if root.resolve(strict=True) != root:
    raise SystemExit("addon path is not canonical")
root_stat = root.lstat()
if stat.S_ISLNK(root_stat.st_mode) or not stat.S_ISDIR(root_stat.st_mode):
    raise SystemExit("addon root is not a real directory")
if root_stat.st_uid != uid:
    raise SystemExit("addon root owner mismatch")
if manifest.exists() or manifest.is_symlink():
    raise SystemExit("manifest output collision")
if manifest.parent.resolve(strict=True) != manifest.parent:
    raise SystemExit("manifest parent is not canonical")
parent_stat = manifest.parent.lstat()
if not stat.S_ISDIR(parent_stat.st_mode) or parent_stat.st_uid != uid:
    raise SystemExit("manifest parent boundary mismatch")

def file_digest(path):
    value = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            value.update(block)
    return value.hexdigest()

entries = []
for path in [root, *root.rglob("*")]:
    metadata = path.lstat()
    relative = "." if path == root else path.relative_to(root).as_posix()
    if any(character in relative for character in "\0\t\r\n"):
        raise SystemExit(f"unsafe addon name: {relative!r}")
    if metadata.st_uid != uid:
        raise SystemExit(f"addon owner mismatch: {relative}")
    entry = {
        "gid": metadata.st_gid,
        "mode": stat.S_IMODE(metadata.st_mode),
        "path": relative,
        "uid": metadata.st_uid,
    }
    if stat.S_ISDIR(metadata.st_mode):
        entry["type"] = "directory"
    elif stat.S_ISREG(metadata.st_mode):
        entry.update({
            "sha256": file_digest(path),
            "size": metadata.st_size,
            "type": "file",
        })
    else:
        raise SystemExit(f"unsupported addon entry: {relative}")
    entries.append(entry)
entries.sort(key=lambda entry: os.fsencode(entry["path"]))
payload = {
    "entries": entries,
    "root": "game/addons/godot_mcp",
    "schema": "unseeing.godot-mcp-addon-manifest.v1",
    "version": "4.1.0",
}
encoded = (json.dumps(payload, sort_keys=True, separators=(",", ":")) + "\n").encode()
descriptor = os.open(manifest, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
os.fchmod(descriptor, 0o600)
with os.fdopen(descriptor, "wb") as opened:
    opened.write(encoded)
    opened.flush()
    os.fsync(opened.fileno())
parent = os.open(manifest.parent, os.O_RDONLY | os.O_DIRECTORY)
os.fsync(parent)
os.close(parent)
# addon-manifest-generator-v1: END
PY
test ! -L "$manifest" && test -s "$manifest"
test "$(stat -c %u:%a "$manifest")" = "$(id -u):600"
```

### Capture, Enable, and use one local editor session

Install is once per worktree; Enable is once per worktree session. The next
block refuses a dirty project, captures its exact tracked preimage, and creates
the only permitted window override. It uses reviewable shell operations only;
the repository does not promise a session helper.

```sh
set -eu
: "${UNSEEING_MCP_BRANCH:?export the chosen MCP branch name}"
branch=$UNSEEING_MCP_BRANCH
case "$branch" in
  *[!A-Za-z0-9._-]*|'') exit 2 ;;
esac
worktree="$HOME/src/unseeing/.worktrees/$branch"
project="$worktree/game/project.godot"
override="$worktree/game/override.cfg"
session="$worktree/.superpowers/mcp-session"
export PATH="$HOME/.local/bin:$PATH"
test "$(realpath "$worktree")" = "$worktree"
test ! -L "$project" && test -f "$project"
test "$(stat -c %u "$project")" = "$(id -u)"
test "$(git -C "$worktree" ls-files -v -- game/project.godot | cut -c1)" = H
git -C "$worktree" diff --quiet HEAD -- game/project.godot
test ! -e "$session" && test ! -L "$session"
test ! -e "$worktree/.superpowers/mcp-session-restored" \
  && test ! -L "$worktree/.superpowers/mcp-session-restored"
test ! -e "$override" && test ! -L "$override"
umask 077
mkdir -m 700 "$session"
python3 - "$project" "$session/project.godot.preimage" \
  "$session/project.godot.preimage.json" <<'PY'
# project-preimage-capture-v1: BEGIN
import hashlib
import json
import os
from pathlib import Path
import stat
import sys

project, preimage, metadata = map(Path, sys.argv[1:])
uid = os.getuid()
if not all(path.is_absolute() for path in (project, preimage, metadata)):
    raise SystemExit("capture paths must be absolute")
if project.parts[-2:] != ("game", "project.godot"):
    raise SystemExit("unexpected project path")
if (preimage.name, metadata.name) != (
    "project.godot.preimage",
    "project.godot.preimage.json",
) or preimage.parent != metadata.parent or preimage.parent.name != "mcp-session":
    raise SystemExit("unexpected capture paths")
session = preimage.parent
if project.resolve(strict=True) != project or session.resolve(strict=True) != session:
    raise SystemExit("capture boundary is not canonical")
session_stat = session.lstat()
if (
    not stat.S_ISDIR(session_stat.st_mode)
    or session_stat.st_uid != uid
    or stat.S_IMODE(session_stat.st_mode) != 0o700
):
    raise SystemExit("session boundary mismatch")
if any(path.exists() or path.is_symlink() for path in (preimage, metadata)):
    raise SystemExit("capture output collision")

no_follow = getattr(os, "O_NOFOLLOW", 0)
source_fd = os.open(project, os.O_RDONLY | no_follow)
preimage_created = False
try:
    before = os.fstat(source_fd)
    if (
        not stat.S_ISREG(before.st_mode)
        or before.st_uid != uid
        or before.st_nlink != 1
    ):
        raise SystemExit("project boundary mismatch")
    target_fd = os.open(
        preimage,
        os.O_WRONLY | os.O_CREAT | os.O_EXCL | no_follow,
        0o600,
    )
    preimage_created = True
    digest = hashlib.sha256()
    size = 0
    with os.fdopen(target_fd, "wb") as target:
        while True:
            block = os.read(source_fd, 1024 * 1024)
            if not block:
                break
            target.write(block)
            digest.update(block)
            size += len(block)
        target.flush()
        os.fsync(target.fileno())
    after = os.fstat(source_fd)
    stable_fields = (
        "st_dev", "st_ino", "st_uid", "st_gid", "st_mode", "st_nlink",
        "st_size", "st_mtime_ns",
    )
    if any(getattr(before, field) != getattr(after, field) for field in stable_fields):
        raise SystemExit("project changed during capture")
    if size != before.st_size:
        raise SystemExit("captured size mismatch")
    captured = {
        "device": before.st_dev,
        "gid": before.st_gid,
        "inode": before.st_ino,
        "mode": stat.S_IMODE(before.st_mode),
        "schema": "unseeing.project-preimage.v1",
        "sha256": digest.hexdigest(),
        "size": before.st_size,
        "uid": before.st_uid,
    }
    encoded = (json.dumps(captured, sort_keys=True, separators=(",", ":")) + "\n").encode()
    metadata_fd = os.open(
        metadata,
        os.O_WRONLY | os.O_CREAT | os.O_EXCL | no_follow,
        0o600,
    )
    with os.fdopen(metadata_fd, "wb") as target:
        target.write(encoded)
        target.flush()
        os.fsync(target.fileno())
    directory_fd = os.open(session, os.O_RDONLY | os.O_DIRECTORY)
    os.fsync(directory_fd)
    os.close(directory_fd)
except BaseException:
    if preimage_created:
        try:
            os.unlink(preimage)
        except FileNotFoundError:
            pass
    try:
        os.unlink(metadata)
    except FileNotFoundError:
        pass
    raise
finally:
    os.close(source_fd)
# project-preimage-capture-v1: END
PY
cat > "$override" <<'EOF'
[display]

window/size/mode=0
window/size/viewport_width=1280
window/size/viewport_height=720
EOF
chmod 600 "$override"
test "$(sha256sum "$override" | awk '{print $1}')" = \
  2eab687e0c3b52888cae279e915c6b3263681173893874f0b57b598a2ed999b8
git -C "$worktree" check-ignore -q game/override.cfg
python3 - "$session" "$override" <<'PY'
import os, sys
for raw in sys.argv[1:]:
    descriptor = os.open(raw, os.O_RDONLY)
    os.fsync(descriptor)
    os.close(descriptor)
PY
```

Open this worktree's `game/project.godot` in the official editor and verify
`4.7.1.stable.official.a13da4feb`. In **Project → Project Settings → Plugins**,
Enable **Godot MCP**. Version 4.1.0 writes an enabled-plugin row, the
`MCPGameBridge` autoload, and exactly four `godot_mcp/*` settings. Disabling
later removes the row but leaves the autoload and four settings, which is why
deleting the addon is not an uninstall.

For a local editor, use only its default loopback endpoint
`127.0.0.1:6550`; start one MCP-capable client from this worktree so its
`.mcp.json` launches the pinned package. A second client is rejected, not
queued; disconnect the owner before retrying. List tools first. Require
addon/server 4.1.0, this worktree's project path, Godot 4.7.1, the configured
main scene, and the expected editor state. Open `res://scenes/level_02.tscn`,
read its scene tree, select one returned child, and read back that selection.
Take exactly one 640-pixel-wide 3D editor viewport capture, inspect it
transiently, and retain no screenshot bytes.

After those editor checks, execute this exact ordered six-call runtime workflow:

1. `godot_editor_edit {action:"run",frozen:true}` starts the configured main
   with its clock stopped.
2. `godot_input {action:"get_map"}` must report the running game's
   `move_forward` action with at least one event. `godot_project get_settings
   category=input` reads editor settings and is not this runtime proof.
3. `godot_game_time {action:"step",frames:2}` performs the exact initialization.
4. `godot_exec {action:"run",source:<snapshot source>}` captures the before snapshot.
5. `godot_game_time {action:"step",frames:30,inputs:[{action_name:"move_forward",start_ms:0,duration_ms:500}]}`
   sends the movement input inside the step request. Do not inject it separately
   while frozen. The request is `frames:30`; Godot MCP 4.1.0's accepted reply
   reports `frames:31`, comprising the 30 requested frames plus one input-settle frame.
6. `godot_exec {action:"run",source:<snapshot source>}` captures the after snapshot.

For both calls represented by `<snapshot source>`, `root` and `tree` are
injected and an explicit return is required:

```gdscript
var main := tree.current_scene
return JSON.stringify(main.observer.snapshot(main.now))
```

Require the fifth call's exact `completed:true`, `frozen:true`, `frames:31`,
zero-dropped-event result, and require both hero and eye positions between the
two snapshots to move more than `0.25 m`. Then run
`godot_validate_meshes {max_findings:25}` and require the complete plain-text
result to start with `Checked 144 meshes (144 surfaces) — no integrity
problems.` Require no new editor or game errors. Save no scene, resource, node,
setting, or source file.

For the ordinary manual lifecycle, stop the game, close the MCP client,
Disable the plugin, and close the editor, in that order. Only then inspect
`git diff -- game/project.godot`. The post-disable diff must contain exactly
the surviving `MCPGameBridge` autoload and four `godot_mcp/*` settings and no
enabled-plugin row. If any other byte is present, retain the session preimage
as recovery evidence and stop without overwriting the project.

After that one human diff review, record the current post-disable SHA-256 and
run the guarded restore block with the two exact acknowledgements. It verifies
the captured preimage, live file metadata, post-disable digest, override, and
unused output paths before replacing anything. It preserves the capture and a
new restoration record for review instead of deleting or overwriting them:

```sh
set -eu
: "${UNSEEING_MCP_BRANCH:?export the chosen MCP branch name}"
: "${UNSEEING_MCP_POST_DISABLE_REVIEWED:?set to autoload-four-settings-only}"
: "${UNSEEING_MCP_POST_DISABLE_SHA256:?copy the reviewed post-disable project SHA-256}"
test "$UNSEEING_MCP_POST_DISABLE_REVIEWED" = autoload-four-settings-only
case "$UNSEEING_MCP_POST_DISABLE_SHA256" in
  *[!0-9a-f]*|'') exit 2 ;;
esac
test "${#UNSEEING_MCP_POST_DISABLE_SHA256}" -eq 64
branch=$UNSEEING_MCP_BRANCH
case "$branch" in
  *[!A-Za-z0-9._-]*|'') exit 2 ;;
esac
worktree="$HOME/src/unseeing/.worktrees/$branch"
project="$worktree/game/project.godot"
override="$worktree/game/override.cfg"
session="$worktree/.superpowers/mcp-session"
preimage="$session/project.godot.preimage"
metadata="$session/project.godot.preimage.json"
record="$session/project.godot.restore-result.json"
export PATH="$HOME/.local/bin:$PATH"
test "$(realpath "$worktree")" = "$worktree"
test -z "$(git -C "$worktree" diff --cached --name-only)"
test "$(git -C "$worktree" diff --name-only)" = game/project.godot
git -C "$worktree" diff --check
python3 - "$project" "$preimage" "$metadata" "$override" \
  "$UNSEEING_MCP_POST_DISABLE_SHA256" "$record" <<'PY'
# project-preimage-restore-v1: BEGIN
import hashlib
import json
import os
from pathlib import Path
import stat
import sys

project, preimage, metadata, override = map(Path, sys.argv[1:5])
post_digest = sys.argv[5]
record = Path(sys.argv[6])
uid = os.getuid()
paths = (project, preimage, metadata, override, record)
if not all(path.is_absolute() for path in paths):
    raise SystemExit("restore paths must be absolute")
if project.parts[-2:] != ("game", "project.godot"):
    raise SystemExit("unexpected project path")
session = preimage.parent
if (
    preimage.name != "project.godot.preimage"
    or metadata != session / "project.godot.preimage.json"
    or record != session / "project.godot.restore-result.json"
    or session.name != "mcp-session"
    or override != project.parent / "override.cfg"
):
    raise SystemExit("unexpected restore paths")
if len(post_digest) != 64 or any(character not in "0123456789abcdef" for character in post_digest):
    raise SystemExit("invalid reviewed post-disable digest")
if project.resolve(strict=True) != project or session.resolve(strict=True) != session:
    raise SystemExit("restore boundary is not canonical")
session_stat = session.lstat()
if (
    not stat.S_ISDIR(session_stat.st_mode)
    or session_stat.st_uid != uid
    or stat.S_IMODE(session_stat.st_mode) != 0o700
):
    raise SystemExit("session boundary mismatch")
if record.exists() or record.is_symlink():
    raise SystemExit("restore record collision")
temporary = project.with_name("project.godot.unseeing-mcp-restore")
if temporary.exists() or temporary.is_symlink():
    raise SystemExit("restore temporary collision")
if {path.name for path in session.iterdir()} != {
    "project.godot.preimage",
    "project.godot.preimage.json",
}:
    raise SystemExit("unexpected session entry")

def regular_metadata(path, required_mode=None):
    value = path.lstat()
    if (
        not stat.S_ISREG(value.st_mode)
        or value.st_uid != uid
        or value.st_nlink != 1
        or (required_mode is not None and stat.S_IMODE(value.st_mode) != required_mode)
    ):
        raise SystemExit(f"file boundary mismatch: {path.name}")
    return value

preimage_stat = regular_metadata(preimage, 0o600)
regular_metadata(metadata, 0o600)
live_stat = regular_metadata(project)
override_stat = regular_metadata(override, 0o600)
if hashlib.sha256(override.read_bytes()).hexdigest() != \
        "2eab687e0c3b52888cae279e915c6b3263681173893874f0b57b598a2ed999b8":
    raise SystemExit("override content mismatch")
try:
    captured = json.loads(metadata.read_bytes())
except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
    raise SystemExit(f"invalid capture metadata: {error}")
expected_keys = {
    "device", "gid", "inode", "mode", "schema", "sha256", "size", "uid",
}
if not isinstance(captured, dict) or set(captured) != expected_keys:
    raise SystemExit("capture metadata shape mismatch")
if captured["schema"] != "unseeing.project-preimage.v1":
    raise SystemExit("capture metadata identity mismatch")
for key in expected_keys - {"schema", "sha256"}:
    if type(captured[key]) is not int or captured[key] < 0:
        raise SystemExit(f"invalid capture metadata field: {key}")
if (
    not isinstance(captured["sha256"], str)
    or len(captured["sha256"]) != 64
    or any(character not in "0123456789abcdef" for character in captured["sha256"])
):
    raise SystemExit("invalid captured digest")
preimage_payload = preimage.read_bytes()
if (
    len(preimage_payload) != captured["size"]
    or preimage_stat.st_size != captured["size"]
    or hashlib.sha256(preimage_payload).hexdigest() != captured["sha256"]
):
    raise SystemExit("preimage content mismatch")
if (
    live_stat.st_uid != captured["uid"]
    or live_stat.st_gid != captured["gid"]
    or stat.S_IMODE(live_stat.st_mode) != captured["mode"]
):
    raise SystemExit("live project metadata mismatch")
if hashlib.sha256(project.read_bytes()).hexdigest() != post_digest:
    raise SystemExit("reviewed post-disable digest mismatch")

no_follow = getattr(os, "O_NOFOLLOW", 0)
temporary_fd = os.open(
    temporary,
    os.O_WRONLY | os.O_CREAT | os.O_EXCL | no_follow,
    0o600,
)
temporary_created = True
try:
    with os.fdopen(temporary_fd, "wb", closefd=False) as target:
        target.write(preimage_payload)
        target.flush()
    os.fchmod(temporary_fd, captured["mode"])
    temporary_stat = os.fstat(temporary_fd)
    if (temporary_stat.st_uid, temporary_stat.st_gid) != (captured["uid"], captured["gid"]):
        os.fchown(temporary_fd, captured["uid"], captured["gid"])
    os.fsync(temporary_fd)
except BaseException:
    try:
        os.unlink(temporary)
        temporary_created = False
    except FileNotFoundError:
        pass
    raise
finally:
    os.close(temporary_fd)

try:
    prepared_stat = regular_metadata(temporary, captured["mode"])
    if (
        prepared_stat.st_uid != captured["uid"]
        or prepared_stat.st_gid != captured["gid"]
        or temporary.read_bytes() != preimage_payload
        or hashlib.sha256(temporary.read_bytes()).hexdigest() != captured["sha256"]
    ):
        raise SystemExit("prepared restoration mismatch")
    os.replace(temporary, project)
    temporary_created = False
    game_fd = os.open(project.parent, os.O_RDONLY | os.O_DIRECTORY)
    os.fsync(game_fd)
    os.close(game_fd)
    restored_payload = project.read_bytes()
    restored_stat = regular_metadata(project, captured["mode"])
    restored_digest = hashlib.sha256(restored_payload).hexdigest()
    if restored_payload != preimage_payload or restored_digest != captured["sha256"]:
        raise SystemExit("restored project content mismatch")
    if (
        restored_stat.st_uid != captured["uid"]
        or restored_stat.st_gid != captured["gid"]
    ):
        raise SystemExit("restored project metadata mismatch")
    os.unlink(override)
    if override.exists() or override.is_symlink():
        raise SystemExit("override removal failed")
    result = {
        "inode_equal_after_atomic_replace": restored_stat.st_ino == captured["inode"],
        "original": {
            "device": captured["device"],
            "gid": captured["gid"],
            "inode": captured["inode"],
            "mode": captured["mode"],
            "sha256": captured["sha256"],
            "size": captured["size"],
            "uid": captured["uid"],
        },
        "restored": {
            "device": restored_stat.st_dev,
            "gid": restored_stat.st_gid,
            "inode": restored_stat.st_ino,
            "mode": stat.S_IMODE(restored_stat.st_mode),
            "sha256": restored_digest,
            "size": restored_stat.st_size,
            "uid": restored_stat.st_uid,
        },
        "schema": "unseeing.project-restore-result.v1",
    }
    encoded = (json.dumps(result, sort_keys=True, separators=(",", ":")) + "\n").encode()
    record_fd = os.open(
        record,
        os.O_WRONLY | os.O_CREAT | os.O_EXCL | no_follow,
        0o600,
    )
    with os.fdopen(record_fd, "wb") as target:
        target.write(encoded)
        target.flush()
        os.fsync(target.fileno())
    session_fd = os.open(session, os.O_RDONLY | os.O_DIRECTORY)
    os.fsync(session_fd)
    os.close(session_fd)
finally:
    if temporary_created:
        try:
            os.unlink(temporary)
        except FileNotFoundError:
            pass
# project-preimage-restore-v1: END
PY
test -z "$(git -C "$worktree" diff --name-only)"
test -z "$(git -C "$worktree" diff --cached --name-only)"
test ! -e "$override" && test ! -L "$override"
test ! -L "$record" && test -s "$record"
test "$(stat -c %u:%a "$record")" = "$(id -u):600"
```

The manual acknowledgements are fail-closed review gates, not automated proof.
If the diff is unfamiliar, preserve it and diagnose instead of setting them.
The retained ignored session directory records both the captured original inode
and the inode installed by atomic replacement; those values are facts, not an
equality guard. Review or archive that owner-only directory before reusing the
worktree. The restored file is proven by its output bytes, SHA-256, UID, GID
and mode, rather than by comparing the preimage record with itself. Device and
original/restored inode values are recorded as identity facts; atomic
replacement does not promise inode equality. Access and modification
timestamps are deliberately outside the restoration contract.
The fixed-path, owner-only worktree leaves a residual same-UID pathname race;
do not run another process as the same user against it during restoration.

### Remote proof boundary, evidence, updates, and rollback

A remote controller must not share the default port. The approved hp-local
proof reserves `127.0.0.1:16550` at both ends, sets the addon's two port
override settings to 16550, and leases one loopback-only SSH `-L` after both
base and full `ssh -G` configurations prove there are no inherited forwards,
control sockets, proxies, local commands, or
`ForkAfterAuthentication` other than `no`. It uses one owned offline SDK client
from exact cached NPX root
`/Users/dmgalchenko/.npm/_npx/e9af8ac9cd94a1c8`: Godot MCP 4.1.0 integrity is
`sha512-uq3Gh5n7fos8vIoXpr32/K7r9tL9eYLbERr+Tolksg3Y+FC5coYEkRkbJ1JktMMhoH/BnGWsWhE5E+XJ/nMEPg==`
and resolved MCP SDK 1.30.0 integrity is
`sha512-xKd8OIzlqNzcqcNumGAa6g+PW2kjD5vrpcKOnfldAUPP3j7lnqMPwlTXQm8gF+UwH72z0lqaRbjr9hqGz0eITA==`.
The SDK child receives only present safe names `HOME`, `LOGNAME`, `PATH`,
`SHELL`, `TERM`, and `USER`, plus fixed `GODOT_HOST`, `GODOT_PORT`, and
`GODOT_MCP_USAGE_LOG=0`. The dated proof never runs npm/npx or uses the
application's built-in MCP connection.

Both base and full effective configurations reject `ClearAllForwardings=yes`.
That value would silently discard the required command-line `-L`; the full
configuration must instead retain exactly the sole 16550 loopback forward.

Its exact target worktree is
`/home/galchenko/src/unseeing/.worktrees/hp-local-mcp-setup`. A reviewed
supervisor owns the official graphical editor through one transient uid-1000
user-systemd unit in the existing GNOME Wayland session, the temporary
project diff, and exact `override.cfg`; a separate lease owner owns this sole
transport:

```text
ssh -N -T \
  -o BatchMode=yes \
  -o ExitOnForwardFailure=yes \
  -o ControlMaster=no \
  -o ControlPath=none \
  -o ControlPersist=no \
  -o ForkAfterAuthentication=no \
  -o PermitLocalCommand=no \
  -o UpdateHostKeys=no \
  -o StrictHostKeyChecking=yes \
  -o ServerAliveInterval=15 \
  -o ServerAliveCountMax=3 \
  -L 127.0.0.1:16550:127.0.0.1:16550 \
  hp-local
```

The dated proof has one absolute monotonic 1200-second deadline and a
1170-second mutation-capable work cutoff. Its final fixed 30-second cleanup
reserve is cleanup-only: no editor, tunnel, controller, or MCP mutation may
start after the work cutoff. The deadline starts before the first
supervisor/editor, tunnel, or owned-controller startup and includes startup
polls, all calls, controller and editor shutdown, direct project restoration,
tunnel cleanup, and listener-absence checks; no component can mint, restart, or
extend either endpoint. The full runtime protocol's automated lifecycle is
intentionally different from the manual path above: stop the game, close the
owned controller and child, stop the editor while the addon remains enabled,
require the complete live diff to contain exactly the enabled row, autoload and
four settings, and restore the captured project directly. It never disables
the addon and never creates a post-disable third phase.

The separate target evidence root is
`/home/galchenko/.local/state/unseeing/mcp-setup/2026-08-25`; it is outside the
sealed setup evidence. The exact listener ownership, maximum lease,
signal-cleanup, manifest and seal rules are in the owned MCP-loop document
linked above.

The first 2026-08-25 preflight observed GNOME Wayland ready, official Godot
CLI identity `4.7.1.stable.official.a13da4feb`, Node `20.19.2`, npm/npx
`9.2.0`, the addon and listener absent, and the controller's pre-existing
usage-log baseline. Those facts were inputs to the later attempts; the final
editor-only result is recorded below.

Ordinary local clients may use the package's default usage logging. Only the
dated owned controller may claim its captured shared usage log unchanged,
because only that boundary sets `GODOT_MCP_USAGE_LOG=0` and verifies the
preimage. Retain shared npm and Godot caches unless a separate complete
ownership proof permits removal.

The target's ordinary Node `20.19.2` fact is separate from the dated controller.
That controller requires exact Node `22.23.2` and validates the executable's
canonical path, UID, mode, link count, device, inode, size, SHA-256, and version.
The observed executable was
`/opt/homebrew/Cellar/node@22/22.23.2_1/bin/node`: UID 501, mode `0555`, link
count 1, device 16777230, inode 11534500, and size 67024; its SHA-256 was
`0143ba3f1c2d9e586115f17d1adcf454079ff28ddc7d684339b43e7dbacc1a1e`.
It descriptor-reads the reviewed NPX tree into a private execution capsule,
holds each module in a held `Buffer` through `registerHooks()`, and admits only
the sealed parent and child resolution ledgers. Neither parent nor child may
reopen a reviewed module pathname after the bytes are held.

The dated usage-log boundary is the complete path, device, inode, UID, mode,
link count, size, line count, SHA-256, and exact descriptor `mtime_ns`
`1787340988551255243`. The earlier Phase-A prose report printed the rejected
rounded value `1787340988000000000`; that is a transcription defect, not an
alternative baseline. The descriptor-backed controller source and regression
test own the exact value, and all fields must remain unchanged after child
cleanup.

Controller success is deliberately nonterminal. Before SDK import, the
supervisor supplies a fresh contract binding the previously absent named unit,
journal start cursor, unit and Godot start identities, exact project and
evidence roots, and pending gate. A clean controller may report only
`controller_lane_status:"passed"` with
`integrated_proof_status:"pending_game_log_gate"`. After the game stops, the
supervisor-owned game-process journal finalizer reads only that unit and cursor
interval, sanitizes and hashes it, requires zero game-process errors, and binds
the terminal result back to the contract. Only that bound result is integrated
proof success.

#### Observed editor-only result and attempt ledger

Attempt 7 is the successful editor-only proof. It used a
direct-owned Godot and SSH fallback after the reviewed supervisor path proved
incompatible with the required host boundaries. Addon readiness was polled through the real
startup race: `connected=false` became `connected=true`, then initially
unknown fields became the full server/addon/project/editor handshake. The MCP
UI reported Godot `4.7.1-stable (official)` while the same official binary's
CLI identity remained `4.7.1.stable.official.a13da4feb`; both exact observed
formats are retained instead of normalizing one into the other.

Through the owned MCP connection the operator opened
`res://scenes/level_02.tscn`, inspected its editor tree, selected and confirmed
`/root/Level02/Room`, and captured one `640×432` 3D editor image. Its SHA-256
was `ae6148b621b4a63c4a2d776e0e705b5ee95d45c76917e96700c04318a3ea6a82`;
the image was visually acknowledged and then retired rather than tracked or
stored in the durable evidence. The editor error cursors `0→0`, the complete
usage-log path/metadata/content boundary remained unchanged, and no scene,
resource, game source, or project setting was saved by a tool call.

No runtime-game MCP claim is made. Attempt 7 deliberately exercised the
editor-only controller mode: it did not launch the game, issue movement or
runtime snapshot calls, validate the running 144-mesh scene, or satisfy the
separate supervisor game-journal gate. The earlier native player, Web export,
and Chromium smoke results under **Build proof** remain the independent
run/export evidence; they are not relabelled as MCP results.

The durable attempt records are
`/home/galchenko/.local/state/unseeing/mcp-setup/2026-08-25/attempt-{1..7}`.
Their concise chronology is:

1. **Attempt 1:** a measured host clock offset of about `+15.7 s` exceeded the
   `±5 s` pre-launch gate; Debian time synchronization was installed and the
   bounded midpoint check later measured `+0.32 s`.
2. **Attempt 2:** the supervisor assumed `project.godot` mode `0644`, but the
   required `umask 077` worktree had tracked-clean mode `0600`; the boundary
   and its tests were corrected to preserve `0600`.
3. **Attempt 3:** a generic post-unit-start `TypeError` triggered cleanup; the
   launch boundary was totalized, but that was not yet the root cause.
4. **Attempt 4:** the repeated failure proved the exception occurred outside
   `runtime.launch`, so the next attempt enabled a private diagnostic
   traceback rather than guessing again.
5. **Attempt 5:** the traceback identified the raw three-argument
   `_probe_godot_version` default crossing a two-argument capture boundary;
   the default became `None` and a deadline-aware wrapper now supplies it.
6. **Attempt 6:** the editor and listener became live, but macOS could not
   execute the reviewed held `/dev/fd/<ssh-fd>` lease. Godot then atomically
   replaced the project inode, so the identity-bound supervisor correctly
   refused automatic restoration. A separately reviewed guarded recovery
   restored the exact bytes/SHA-256/UID/GID/mode, recorded old and new inode
   facts, and led to the direct-owned fallback with content-based guarded
   restoration for this known Godot save behavior.
7. **Attempt 7:** direct owned Godot and SSH reached the full editor handshake,
   completed the scoped calls above, and passed every closing gate.

Final cleanup closed the owned MCP child and transport, direct SSH process,
both loopback listeners, and editor; removed the exact temporary override;
restored the tracked project bytes, SHA-256, UID, GID, and mode while recording
device and old/new inode as facts; and left both tracked worktrees clean. The
ready isolated worktree remains at
`/home/galchenko/src/unseeing/.worktrees/hp-local-mcp-setup`, branch
`chore/hp-local-mcp-setup`, HEAD `d6285b0`, with exactly the retained ignored
roots `.superpowers/`, `game/.godot/`, `game/addons/godot_mcp/`, and
`rust/target/`. There is no unit, PID, listener, deadline, override, recovery,
or temporary residue. This is a retention record, not a deletion claim. No
screenshot byte, ignored controller helper, credential, raw environment, or
private usage-log content enters this tracked guide.

To update godot-mcp, change the exact version in both `.mcp.json` and
`tools/setup-mcp.sh` on an isolated branch, review the new registry integrity
and package source, install into a fresh worktree, and repeat the complete GUI,
structured movement, mesh, error, restore, and clean-status proof. Never use an
unpinned/latest addon.

To remove only the addon after a restored, stopped session, verify the ignored
install against the manifest created above, then remove that exact tree:

```sh
set -eu
: "${UNSEEING_MCP_BRANCH:?export the chosen MCP branch name}"
branch=$UNSEEING_MCP_BRANCH
case "$branch" in
  *[!A-Za-z0-9._-]*|'') exit 2 ;;
esac
worktree="$HOME/src/unseeing/.worktrees/$branch"
addon="$worktree/game/addons/godot_mcp"
manifest="$worktree/.superpowers/mcp-addon-4.1.0-manifest.json"
test "$(realpath "$worktree")" = "$worktree"
test "$(realpath "$addon")" = "$addon"
test ! -L "$addon" && test -d "$addon"
test "$(stat -c %u "$addon")" = "$(id -u)"
test ! -L "$manifest" && test -f "$manifest"
test "$(stat -c %u:%a "$manifest")" = "$(id -u):600"
test -z "$(git -C "$worktree" status --short)"
test ! -e "$worktree/game/override.cfg" \
  && test ! -L "$worktree/game/override.cfg"
python3 - "$addon" "$manifest" <<'PY'
# addon-manifest-remover-v1: BEGIN
import hashlib
import json
import os
from pathlib import Path
import stat
import sys

root, manifest = map(Path, sys.argv[1:])
uid = os.getuid()
if not root.is_absolute() or not manifest.is_absolute():
    raise SystemExit("addon and manifest paths must be absolute")
if root.as_posix().endswith("/game/addons/godot_mcp") is False:
    raise SystemExit("unexpected addon path")
if root.resolve(strict=True) != root:
    raise SystemExit("addon path is not canonical")
if manifest.resolve(strict=True) != manifest:
    raise SystemExit("manifest path is not canonical")
manifest_stat = manifest.lstat()
if not stat.S_ISREG(manifest_stat.st_mode) or manifest_stat.st_uid != uid:
    raise SystemExit("manifest boundary mismatch")
if stat.S_IMODE(manifest_stat.st_mode) != 0o600:
    raise SystemExit("manifest mode mismatch")
try:
    payload = json.loads(manifest.read_bytes())
except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
    raise SystemExit(f"invalid addon manifest: {error}")
if not isinstance(payload, dict) or {
    "root": payload.get("root"),
    "schema": payload.get("schema"),
    "version": payload.get("version"),
} != {
    "root": "game/addons/godot_mcp",
    "schema": "unseeing.godot-mcp-addon-manifest.v1",
    "version": "4.1.0",
}:
    raise SystemExit("addon manifest identity mismatch")
recorded_entries = payload.get("entries")
if not isinstance(recorded_entries, list) or not recorded_entries:
    raise SystemExit("addon manifest entries missing")

def digest_path(path):
    value = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            value.update(block)
    return value.hexdigest()

def scan_tree():
    entries = []
    for path in [root, *root.rglob("*")]:
        metadata = path.lstat()
        relative = "." if path == root else path.relative_to(root).as_posix()
        if any(character in relative for character in "\0\t\r\n"):
            raise SystemExit(f"unsafe addon name: {relative!r}")
        if metadata.st_uid != uid:
            raise SystemExit(f"addon owner mismatch: {relative}")
        entry = {
            "gid": metadata.st_gid,
            "mode": stat.S_IMODE(metadata.st_mode),
            "path": relative,
            "uid": metadata.st_uid,
        }
        if stat.S_ISDIR(metadata.st_mode):
            entry["type"] = "directory"
        elif stat.S_ISREG(metadata.st_mode):
            entry.update({
                "sha256": digest_path(path),
                "size": metadata.st_size,
                "type": "file",
            })
        else:
            raise SystemExit(f"unsupported addon entry: {relative}")
        entries.append(entry)
    entries.sort(key=lambda entry: os.fsencode(entry["path"]))
    return entries

parent_fd = os.open(root.parent, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
root_fd = None
try:
    path_stat = os.stat(root.name, dir_fd=parent_fd, follow_symlinks=False)
    if not stat.S_ISDIR(path_stat.st_mode) or path_stat.st_uid != uid:
        raise SystemExit("addon root boundary mismatch")
    root_fd = os.open(
        root.name,
        os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW,
        dir_fd=parent_fd,
    )
    held_stat = os.fstat(root_fd)
    if (held_stat.st_dev, held_stat.st_ino) != (path_stat.st_dev, path_stat.st_ino):
        raise SystemExit("addon root identity changed")
    current_entries = scan_tree()
    if current_entries != recorded_entries:
        raise SystemExit("addon tree differs from recorded manifest")

    by_path = {entry["path"]: entry for entry in recorded_entries}
    if len(by_path) != len(recorded_entries) or "." not in by_path:
        raise SystemExit("addon manifest contains duplicate or missing paths")
    # Validate every recorded entry again before the first unlink. Intermediate
    # same-UID pathname replacement remains the documented owner-only race.
    for relative, entry in by_path.items():
        metadata = os.stat(relative, dir_fd=root_fd, follow_symlinks=False)
        if metadata.st_uid != entry["uid"] or metadata.st_gid != entry["gid"]:
            raise SystemExit(f"addon owner changed before removal: {relative}")
        if stat.S_IMODE(metadata.st_mode) != entry["mode"]:
            raise SystemExit(f"addon mode changed before removal: {relative}")
        if entry["type"] == "directory":
            if not stat.S_ISDIR(metadata.st_mode):
                raise SystemExit(f"addon directory type changed: {relative}")
        elif entry["type"] == "file":
            if not stat.S_ISREG(metadata.st_mode) or metadata.st_size != entry["size"]:
                raise SystemExit(f"addon file boundary changed: {relative}")
            descriptor = os.open(relative, os.O_RDONLY | os.O_NOFOLLOW, dir_fd=root_fd)
            try:
                opened_stat = os.fstat(descriptor)
                if (opened_stat.st_dev, opened_stat.st_ino) != (metadata.st_dev, metadata.st_ino):
                    raise SystemExit(f"addon file identity changed: {relative}")
                value = hashlib.sha256()
                for block in iter(lambda: os.read(descriptor, 1024 * 1024), b""):
                    value.update(block)
                actual_hash = value.hexdigest()
                if actual_hash != entry["sha256"]:
                    raise SystemExit(f"addon file hash changed: {relative}")
            finally:
                os.close(descriptor)
        else:
            raise SystemExit(f"unsupported recorded addon type: {relative}")

    for relative, entry in sorted(
        by_path.items(), key=lambda item: os.fsencode(item[0])
    ):
        if entry["type"] == "file":
            os.unlink(relative, dir_fd=root_fd)
    directories = [
        relative for relative, entry in by_path.items()
        if entry["type"] == "directory" and relative != "."
    ]
    for relative in sorted(
        directories,
        key=lambda name: (-name.count("/"), os.fsencode(name)),
    ):
        os.rmdir(relative, dir_fd=root_fd)
    final_held = os.fstat(root_fd)
    if (final_held.st_dev, final_held.st_ino) != (held_stat.st_dev, held_stat.st_ino):
        raise SystemExit("held addon root identity changed")
    os.close(root_fd)
    root_fd = None
    final_path = os.stat(root.name, dir_fd=parent_fd, follow_symlinks=False)
    if (final_path.st_dev, final_path.st_ino) != (held_stat.st_dev, held_stat.st_ino):
        raise SystemExit("addon root path identity changed")
    os.rmdir(root.name, dir_fd=parent_fd)
    os.fsync(parent_fd)
finally:
    if root_fd is not None:
        os.close(root_fd)
    os.close(parent_fd)
# addon-manifest-remover-v1: END
PY
test ! -e "$addon" && test ! -L "$addon"
rm -f -- "$manifest"
```

Do not delete the worktree merely to uninstall the addon. Whole-worktree
rollback additionally requires exact branch/HEAD, clean tracked state, no
owned editor/client/tunnel/unit, restored `project.godot`, and complete
manifests for every ignored output. Remove those bounded outputs first, then
use non-forced `git worktree remove` and safe `git branch -d`; never force or
recursively delete Git worktree administration state.

## Exact hp-local ledger — 2026-08-24

### Host role and audited starting point

- SSH alias: `hp-local`; hostname: `antisleep`; user/group:
  `galchenko:galchenko` (uid/gid 1000); x86_64.
- Debian 13 (`trixie`), kernel `6.12.101+deb13-amd64`; AMD Ryzen 7 5800U;
  7.1 GiB RAM, 7.3 GiB swap, and 421 GiB free at audit time.
- GNOME 48 graphical session with AMD Cezanne `/dev/dri` devices.
- Build baseline: public `main` at
  `d6285b0bba84dd29846a9613c2e8081191e46cfd`.
- Pre-existing package versions included `build-essential 12.12`,
  `git 2.47.3`, `gh 2.46.0`, GCC package `4:14.2.0-1`, Python `3.13.5`,
  Clang 19 package `1:19.1.7-3+b1`, Clang 22 package
  `1:22.1.8~++20260613092233+e80beda6e255-1~exp1~20260613092250.77`,
  pipx `1.7.1`, Node `20.19.2`, npm `9.2.0`, ShellCheck `0.10.0`, curl
  `8.14.1`, coreutils `9.7`, gzip `1.13`, tar `1.35`, unzip `6.0`, zip
  `3.0`, xz-utils `5.8.1`, wget `1.25.0`, OpenSSH client `10.0p1`, and
  CA certificates `20250419`.
- Godot, templates, rustup, the pinned Rust lanes, gdtoolkit, Chromium,
  Brotli, and emsdk were absent. No Unseeing checkout existed.
- `/home/galchenko/.cargo` pre-existed at mode `0775` with only empty
  `.crates.toml` and `.crates2.json`, both mode `0664` and SHA-256
  `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`.
- GitHub CLI was not authenticated; `gh auth status` returned 1. GitHub SSH
  authentication was also unavailable.

The audit facts are recorded in `before-system-identity.*`,
`before-packages.*`, `before-manual-packages.*`, `before-tool-state.*`,
`before-cargo-root-{metadata,file-hashes}.*`, and `before-gh-state.*` beneath
the evidence root.

### Known-host audit side effect

An earlier failed `ssh -T git@github.com` audit accepted GitHub's public
ED25519 host key into `/home/galchenko/.ssh/known_hosts`. This happened before
Task 2's baseline and did not authenticate the user. The recorded public key
fingerprint is:

```text
SHA256:+DiY3wvvV6TuJJhbpZisF/zLDA0zPMSvHdkr4UvCOqU
```

The post-side-effect whole-file SHA-256 is
`b6d915f0f612aa8aae124f43d5d9c0b073bc0f879e6079af77f414304c0e0f4f`.
`before-known-hosts-github-public.*`,
`before-known-hosts-github-fingerprint.*`, and `before-known-hosts-hash.*`
own the raw public evidence. The hash remained unchanged through installation.
The narrow rollback is given below; do not use a broad SSH-directory removal.

### Debian transaction

For the 2026-08-24 base transaction, the only APT commands were
`apt-get update` and `apt-get install -y chromium brotli`. The independent
2026-08-26 network-time addition is recorded in the next subsection. The base
install transaction ran from
`2026-08-24 19:19:10 +0300` through `19:19:19 +0300`. Exactly 11 packages were
new, none were removed, and exactly `chromium` and `brotli` became newly
manual:

| Package | Exact version | Marking from transaction |
| --- | --- | --- |
| `avahi-utils` | `0.8-16` | automatic |
| `brotli` | `1.1.0-2+b7` | manual |
| `chromium` | `151.0.7922.169-1~deb13u1` | manual |
| `chromium-common` | `151.0.7922.169-1~deb13u1` | automatic |
| `chromium-sandbox` | `151.0.7922.169-1~deb13u1` | automatic |
| `gir1.2-handy-1:amd64` | `1.8.3-2` | automatic |
| `gir1.2-packagekitglib-1.0` | `1.3.1-1+deb13u1` | automatic |
| `libdouble-conversion3:amd64` | `3.3.1-1` | automatic |
| `libminizip1t64:amd64` | `1:1.3.dfsg+really1.3.1-1+b1` | automatic |
| `libxnvctrl0:amd64` | `535.171.04-1+b2` | automatic |
| `system-config-printer` | `1.5.18-4` | automatic |

The exact owner records are `resume-1/apt-history-transaction.stdout.txt`
(SHA-256 `1430333f72a3b47afa813651a102b16a44232aa630683b292779346951c58ffc`),
`resume-1/apt-new-packages.stdout.txt` (SHA-256
`9046f89a70b2dc397144e4a747a4f3e25675bb95f7d1801b46f143739b1d6d5b`),
and `resume-1/apt-new-manual-packages.stdout.txt` (SHA-256
`86f0a3f0fe6735e6441b30ac89607749df894c561478a8071f03f30ddb5f3b8b`).
APT source hashes were unchanged; no repository was added. Package index/cache
refreshes are regenerated APT metadata, not manually rewound files.

### Network-time prerequisite added — 2026-08-26

The first live-editor preflight exposed an approximately 16-second remote
clock lead, so the proof stopped before launching Godot. A bounded local/remote
midpoint sample measured a `656.481 ms` round trip: the pre-fix midpoint offset
was `+16.040137223 s`, with the conservative interval
`+15.711896723..+16.368377723 s`.

APT simulation first showed exact package `257.13-1~deb13u1`, one new package,
no upgrade/removal, and no plan drift. No `apt-get update` was needed. The
approved command was:

```sh
sudo -n apt-get install --yes systemd-timesyncd=257.13-1~deb13u1
```

It installed exactly one new package, `systemd-timesyncd` `257.13-1~deb13u1`, with zero upgrades and zero removals. APT fetched
`93.3 kB`, reported `208 kB` additional disk use, and created the package-owned
`systemd-timesync` user/group at UID/GID 986 plus its normal enablement and
D-Bus symlinks.

The first `sudo timedatectl set-ntp true` returned `NTP not supported` because
the already-running `systemd-timedated` still held its pre-install capability
cache. Restarting only `systemd-timedated.service` and repeating the command
enabled NTP. Detailed status then timed out because the package-started
timesync daemon preceded the D-Bus trigger; restarting only
`systemd-timesyncd.service` made the interface available. No time was set
manually.

Final checks reported the service enabled and active, with `CanNTP=yes`,
`NTP=yes`, and `NTPSynchronized=yes`. `timedatectl timesync-status` named
`0.debian.pool.ntp.org` at `51.250.53.172`, stratum 2, root distance
`24.558 ms`, daemon offset `-46.997 ms`, and delay `117.285 ms`. A bounded
`250.725 ms` round-trip sample then found the post-fix midpoint offset was
`+0.321950582 s`, interval `+0.196588082..+0.447313082 s`, safely inside the
`±1 s` closing gate. One later local reporting poll printed `TIMEOUT` only
because its parser failed to strip whitespace before `Server:`; all captured
host values remained green, so no host command was repeated.

### User-owned tools and persistent paths

The dated Godot downloads were exactly:

```text
https://github.com/godotengine/godot/releases/download/4.7.1-stable/Godot_v4.7.1-stable_linux.x86_64.zip
SHA-512 4ccdab7a48eeccbe8819a2fc1f6262f8d72065d98601bcb3743fcbd7ebd39f373758a788ee3293a05ec5b2c48538266c437404312e372225cd2df273945a2de9
https://github.com/godotengine/godot/releases/download/4.7.1-stable/Godot_v4.7.1-stable_export_templates.tpz
SHA-512 afcc83d8d3d298038f19c58744a0d660fa75dd4baa33cb55d1011bb2565a2a8c2381728924564cb909e37c205a23f21b521b23bd057993afd43ae4da0b2f9d47
https://github.com/godotengine/godot/releases/download/4.7.1-stable/SHA512-SUMS.txt
```

The rustup installer and sidecar URLs were
`https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init`
and the same URL plus `.sha256`; the resolved installer SHA-256 was
`4acc9acc76d5079515b46346a485974457b5a79893cfb01112423c89aeb5aa10`.
The gdtoolkit 4.5.0 wheel SHA-256 was
`f25c5bf7f7fe861e1127164c5d73e0a7fb204ec74cf05d375b76a5dcf8610cdb`.
Rustup installed stable `1.97.1` with `rustfmt`, `clippy`, and exactly these six
stable targets: `aarch64-apple-darwin`, `x86_64-apple-darwin`,
`x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc`,
`aarch64-unknown-linux-gnu`, and `x86_64-unknown-linux-gnu`. The separate Web
lane is `nightly-2026-05-25` with `rust-src` and
`wasm32-unknown-emscripten`.

| Owner | Persistent path or state | Observed result | Verification / bounded rollback owner |
| --- | --- | --- | --- |
| Godot install | `~/.local/bin/Godot_v4.7.1-stable_linux.x86_64` | official binary, mode `0755` | `resume-1/manifest-godot-editor-*`; exact-path removal below |
| Godot install | `~/.local/bin/godot` | relative link to the versioned binary | `resume-1/final-tool-paths.*`; unlink only this name |
| Godot templates | `~/.local/share/godot/export_templates/4.7.1.stable/` | 35 regular files | `resume-1/manifest-godot-templates-*`; exact-root removal below |
| rustup | `~/.rustup/` and rustup-managed additions under pre-existing `~/.cargo/` | rustup `1.29.0`, stable/nightly lanes above | `resume-1/manifest-{rustup,cargo}-*`; rustup rollback must restore the two empty pre-existing Cargo metadata files |
| rustup installer | `~/.profile`, `~/.bashrc` | added exactly `. "$HOME/.cargo/env"` once to each | before hashes `55d100...` / `0cfb4c...`; after hashes `2d03ca...` / `de42f9...`; remove only exact final lines after hash guards |
| pipx | `~/.local/share/pipx/venvs/gdtoolkit/` and five console links | gdtoolkit `4.5.0` | `resume-1/manifest-pipx-gdtoolkit-*`; `pipx uninstall gdtoolkit` |
| emsdk | `/home/galchenko/emsdk/` | tag `4.0.20`, exact commit above | `resume-1/manifest-emsdk-*`; emsdk uninstall then guarded exact-root removal |
| evidence | `~/.local/state/unseeing/setup/2026-08-24/` | owner `galchenko`, mode `0700` | retained audit boundary; remove only as the final deliberate ledger rollback |

Godot's official release SHA-512 values are recorded earlier in this guide.
The resolved rustup installer and gdtoolkit wheel hashes are also recorded in
`resume-1/rustup-installer-resolved-sha256.stdout.txt` and
`resume-1/gdtoolkit-wheel-sha256.stdout.txt`. Emsdk was activated only inside
its checkout: no `/home/galchenko/.emscripten` or startup activation appeared.

The six installed-root manifest pairs are complete sorted metadata plus
regular-file hashes. The values below hash the manifest files themselves:

| Root | Metadata entries | Regular files | Metadata-manifest SHA-256 | File-manifest SHA-256 |
| --- | ---: | ---: | --- | --- |
| Cargo | 19 | 4 | `91ceca7eb49640a3d6094720159860dad8b66bbcd84fee37dadda2af21fe6153` | `680edd095e654f18318bdf53e5de2e318991b0b8ecf8a7cc712be0e9afeb99d4` |
| emsdk | 22,194 | 19,846 | `10e05c27241021684ea146bd64fa031a90f260b682283939a3071b86a3b0024b` | `65d8b9728950d376e290dc8dc3375c1b65b688cf51ccbbf757df084f1cdc9647` |
| Godot editor | 2 | 1 | `75382b9a1874114c09c491bf8e7b7eb00b0961fcb073020c8d9f39814b6288c1` | `66e7b2b4beddee4c55edaca4704127fb17080523e0a585ebe0856e62e260519d` |
| Godot templates | 36 | 35 | `c8deadae70b1a6468f62904f411a2c0e78b84f74f1ae576a2d51747fc423704c` | `bcc0d0378f44042551c4dafc919f36fbcba5a4077435e66e3649cc4864a4327e` |
| pipx gdtoolkit | 1,144 | 978 | `3c678af003f5954db133a7d39b3b9f2be696a48d4894abac357b2ef3e581887b` | `171871c2dd9ef1cb4e378d168ddac5fed4b0ddc8895186630ccf7599b44f6489` |
| rustup | 5,186 | 4,213 | `47b40e489463bef3955c97b26c97509b1ff274a89f1bdfe3003b44014a56d67a` | `1f41a3fe2fe03ed8c0a56e36cc62e272d6c510453b0ed0030a5b6567c3f1c015` |

Each pair is `resume-1/manifest-<root>-metadata.tsv` and
`resume-1/manifest-<root>-files.sha256.txt`; the matching
`manifest-<root>.self-sha256.txt` binds both.

### Durable checkout and local-only configuration

Task 3 created `/home/galchenko/src/` (owner `galchenko`, mode `0755`) and the
HTTPS clone `/home/galchenko/src/unseeing/`. The clone remained clean on
`main` at `d6285b0bba84dd29846a9613c2e8081191e46cfd`, with canonical origin and
the sole `tools/superpowers` submodule at
`b36e0829c6d0140e93cfef2ca599b1b07d4a7797` (`v6.3.0`). `.git/config` owns
the only Git settings changed:

```text
user.name=Dmitrii Galchenko
user.email=dggrus@gmail.com
core.hooksPath=.githooks
```

The required identity and hooks policy is owned by `AGENTS.md`; the submodule
pin is owned by the parent gitlink and `ci/superpowers.lock`. No global Git
configuration changed. The ignored submodule checkout is
developer tooling and never enters deployment output.

### Failed attempts, recovery, sanitization, and historical checkpoint

The evidence is append-only about failures as well as success:

1. The first baseline attempt stopped before installation when a display-only
   Bash `printf` parsed a leading `--` as an option. The already-created
   evidence root and empty downloads child were retained. Fresh comparisons
   proved no package, APT source, startup, known-host, or tool change.
2. Resume 1 installed all prerequisites, then stopped because a composite
   Emscripten probe leaked setup text to protected stderr evidence and masked a
   producer's nonzero status. The raw values are not reproduced here.
3. Resume 2 validated that exact file but stopped before removal when shell
   expansion of an embedded awk field made a value-free tombstone renderer
   fail. The partial tombstone and original file were preserved.
4. The reviewed recovery helper ran once. It kept the same approved inode,
   replaced the affected file with a canonical 288-byte value-free record at
   mode `0600`, and recorded intent, result, corrected direct probe channels,
   and supersession. The sanitized record SHA-256 is
   `8809f5546aafe42d6b741573a86e0e6b1cfce66802d27906f8c73fcb7f6ef67a`;
   the reviewed helper SHA-256 is
   `e9d1df4fed774b9fdb877e1e06fa4c0130e3026d00ae934d01cd55e41cb162f5`.
5. The recovery seal contains 490 sorted relative entries and excludes exactly
   `resume-3-evidence-seal.sha256.txt` and
   `resume-3-evidence-seal.digest.txt`. Its digest is
   `ad0a3c2626a4c7c85e8a0f04a7f15bffa0fd5affe1d7065c1da8f4b5fd272385`.

That seal is the immutable **historical pre-cleanup checkpoint**, not the
current-tree final seal. At Phase-A review time the evidence tree has 492
regular files total; the checkpoint includes 42 regular download files in four
directories, no download symlink, and 3,592,932,582 download bytes. Task 5
will retain the checkpoint pair, remove only its exact `downloads/` child, and
seal the post-cleanup tree.

### Observed editor-only result and pending setup final-seal table

This is the guide's sole mixed-disposition table. It separates Task 4A's
completed 2026-08-25 editor-only MCP boundary from Task 5's still-pending
dated-setup download cleanup and final seal. Completed rows contain only the
observed scoped result; pending rows remain visibly unclaimed.

| Required observed value | Observed state |
| --- | --- |
| Task 4A isolated worktree, bootstrap and addon-install verdict/manifests | Passed: worktree-local bootstrap/class census and exact ignored addon 4.1.0 install verified; addon retained |
| Task 4A GUI, structured MCP, movement, mesh and error results | Editor-only scope passed in attempt 7: exact handshake, level 02 tree/selection/capture, and zero new editor errors. No game, movement, runtime snapshot, or running-mesh result claimed |
| Task 4A project restore, process/port cleanup and separate evidence | Passed: exact tracked project restoration and clean status; owned editor/controller/SSH/listeners absent; attempt evidence retained under the separate MCP evidence root |
| Exact downloads-removal verdict and absence proof | Awaiting the reviewed Phase-B run |
| Copied cleanup-helper SHA-256 | Awaiting the reviewed Phase-B copy verification |
| Cleanup/supersession record path and SHA-256 | Awaiting the reviewed Phase-B run |
| Final manifest entry count | Awaiting read-only post-run verification |
| Final evidence regular-file count | Awaiting read-only post-run verification |
| Final manifest full SHA-256 and digest record | Awaiting read-only post-run verification |

### Controller Phase-A-only changes and retired synthetic-fixture cleanup

Task 4A's controller Phase A did not contact `hp-local`, start SSH, open a
listener, start Godot/MCP, or run npm/npx. It installed no package. It created
only ignored controller sources, tests, private execution capsules and
short-lived local Node processes under the task's `.superpowers/` or macOS
temporary boundaries. The exact controller runtime was Node `22.23.2`; the
ordinary hp-local Node `20.19.2` prerequisite remained untouched. The shared
usage log remained 92,992 bytes / 594 lines with SHA-256
`4a470e3854b12fdb0db7915ffc6940c1b6332d77f14f570cfaadeb15a1ff7929`
and exact descriptor `mtime_ns` `1787340988551255243`.

One later, separately reviewed local cleanup removed only this exact 14-path
synthetic-fixture roster beneath the controller user's private macOS temporary
directory:

```text
unseeing-child-byte-loader-MsGe3W
unseeing-controller-mutations-LNA1Sb
unseeing-held-esm-loader-1ixauo
unseeing-held-esm-loader-2AX5d3
unseeing-held-esm-loader-xiBsgn
unseeing-real-sdk-init-failure-6V3nnh
unseeing-real-sdk-init-failure-8Dg80o
unseeing-real-sdk-init-failure-KTaeqm
unseeing-real-sdk-init-failure-R0Mqun
unseeing-real-sdk-init-failure-SCJD5p
unseeing-real-sdk-init-failure-WUs5HM
unseeing-real-sdk-init-failure-Wlx8qf
unseeing-real-sdk-init-failure-npjFuy
unseeing-replaced-esm-loader-5xXW6Y
```

The preflight reconciled an earlier 11-name observation to that exact roster,
then matched every literal root and member by non-following type, UID, mode,
link count, device/inode/time identity, size, and regular-file SHA-256 before
quarantine. The retained disposition records removal and all 14 original paths
absent. No game, repository, or user-authored data was removed. Later review
identified an irreducible theoretical final-pathname race: another same-UID
process could create a matching pathname after the final census but before
return. The destructive helper is retired fail-closed and must not be reused;
the disposition proves only the bounded deletion that actually occurred, not
permanent absence after its final observation.

### Build proof

All four commands ran once, in order, with status 0 against the clean baseline
SHA. Logs and literal status files are under ignored
`game/reports/hp-local-setup/`.

| Gate | Exact command | Result | Duration | Log SHA-256 |
| --- | --- | --- | ---: | --- |
| Bootstrap | `CARGO_BUILD_JOBS=4 tools/bootstrap.sh` | `probe: PASS (19 checks)`; `bootstrap: OK` | 174 s | `f94ee785da52fcadc67536622f6c3a79e4f377c491b125b182031f5ffc000ed0` |
| Checks only | `CARGO_BUILD_JOBS=4 SKIP_EXPORT=1 ci/pipeline.sh` | 568 Rust tests passed; 33 gdUnit suites / 361 cases, zero errors/failures/skips; `ci: OK` | 575 s | `403ad9feeb18f44c7a32894eaa2d94a63e50dcd6e6349668082ced1caf74eba3` |
| Full Web | `CARGO_BUILD_JOBS=4 ci/pipeline.sh` | same 568 and 361 counts; wasm/export/precompression; two Chromium smoke passes; `ci: OK` | 516 s | `73aa96fe76f78b5472557820375ce1ec82eeae0fe9c67100b3d3fe3280fcd349` |
| Linux x86_64 | `CARGO_BUILD_JOBS=4 tools/export_linux.sh "Linux x86_64" build/linux/unseeing` | 73,561,400-byte executable, adjacent GDExtension, no loose `.pck`; `export-linux: OK` | 5 s | `1e17db56a3e19b1aa9b07100b81f9b232e7ee60d5258119d02f08a9f99c43a5e` |

The four status-record hashes, in the same order, are
`f0c98e48c47cb8eae286afb139c832976ec9f91a8318714c3e770d92bb176316`,
`90268b8c282513f8817dd2df96fb179d8d97130c2ce929a427df34c89274bd83`,
`4c0bc033543851031a573cf6bcc172976da45116be3532d2bb9de7426838098b`,
and `ff8940564212e4b1320b6174643c40ea0b054c710c2e22f33a279acffad72e48`.
PowerShell self-tests and the macOS universal check were the only expected
platform skips on Debian; the full Web export and smoke test were not skipped.

The artifact manifest has 28 entries and SHA-256
`f1464e106d7765b37818deedc7302c0c75e3625134be56d9c5b9923b24888350`:

```text
game/build/linux.log	7434	17cf27b93e995b096ea50427074949c0afc5b42804648797b73a6488f41619d1
game/build/linux/libunseeing_core.so	7981096	e899263c8d668ffbf541c65f19ce722c96e164123ba867308e8675b0fc925e6d
game/build/linux/unseeing	73561400	d45d1d1b70bbaedcab9338b232d568fbbeea1ac235d9e78616749967de993b00
game/build/web/index.apple-touch-icon.png	12364	81774f774b986fb194384348c63095d34d4f2b74422f2097d0146f0e0bad0375
game/build/web/index.audio.position.worklet.js	2973	be33985bc7160d6bf9646f259cd86b259cd67b02ccb297ee5c44f8ac84327bc8
game/build/web/index.audio.position.worklet.js.br	895	6b8771b3e0036d1a57f705d56a767eaa286338eac273ea6272107605ff256990
game/build/web/index.audio.position.worklet.js.gz	1193	cf5ede3ca25ef22289e7e4ff09e6d9c5414f7e5a41864381e7a9338dbb4e853d
game/build/web/index.audio.worklet.js	7298	5b476a9c9ce642c0ee4256436d1bc31d9c38f868aca0f9a8e2a57c18d2dec2a3
game/build/web/index.audio.worklet.js.br	1821	a6c7859736ff0566376f5c655ad6dc0873eb39268e115e8db84ef4c2903bc184
game/build/web/index.audio.worklet.js.gz	2217	cdaf16482a8c72819db347841c7e71533372fd6f7c79a3c29ce5a327e967ac8f
game/build/web/index.html	5962	3cdac41a83a5003a2796fde374b1295f8e0d21358617969a227e4c253139222c
game/build/web/index.icon.png	5414	a93b80fcf438ce3f94f7d4c5b33bc0bb88bd0b9deeefc5fce4b4cb2a6911eb96
game/build/web/index.js	2859484	631bbecee5e5136b5f29ed7dedb31a5d022d65da4b639b6e52a48d0c5e3d3154
game/build/web/index.js.br	253226	70c8a36a0e1b245636dead2e64c1c439ca09aa5e14751f37d2cc08c77171460a
game/build/web/index.js.gz	358183	533c915266a0ee6c93bfbba391e2627719f21c572cad6a00745473f56ee108de
game/build/web/index.pck	91128	9b86dd611a1afcf8a93c89e979203dcb5896a30566c8c9bd39852e54959f1b43
game/build/web/index.pck.br	28289	6d9a4c486296fa56500cdf28ca61e79bf7cd3a751df13d5724408b0c823e92b1
game/build/web/index.pck.gz	33876	4c7a679263a1d1f1e171788a6986abf90be1677679eecf6b062e626bf29a86d3
game/build/web/index.png	21443	3cb4495c0b98dfbe4b663cbf2b6836473572339beb66d902367893162a70be0e
game/build/web/index.side.wasm	44077147	7e4df20e1d767f033a2267b94dddcf82718d38dd8b69f6d45c60a6fc90d464f6
game/build/web/index.side.wasm.br	7071944	622bd7c6b52a39157119542eb08f1d2d06a99bed40e92f7aeedcfdc4be89ca59
game/build/web/index.side.wasm.gz	10485382	c246e0ab3eaba8d466f57641d6ad3e5c07135d39c856c636125877f0ed480d3c
game/build/web/index.wasm	1508095	d37e06f7126849a81fa099f1cf89f0d8ccbb87502b8c857492490dff66fad95a
game/build/web/index.wasm.br	475121	c6f0a7bdaf3b1c4748c135abb242e4c015090a90b2e18b3dbde7c23b8f9ae823
game/build/web/index.wasm.gz	582004	0b45d49ea73ef1302f01ea6c8688a5cb615944b185446dcb2ff88634275e4b0f
game/build/web/unseeing_core.wasm	1946677	075aaa2a34e5ac48d8994054bf909cba1db015ffc23a22a56d678834825f3bfb
game/build/web/unseeing_core.wasm.br	375400	f69c0b133f8a075a71da6c7dba6cfd4240262b74f153d9531b089c8eb4511a3b
game/build/web/unseeing_core.wasm.gz	536810	e843912afaaaa7454bda63489ee338d99be93d385fe19490118f291fc0a1c604
```

### Generated ignored artifacts, temporary removals, and no-change results

The build intentionally retained these ignored paths for local reuse:

```text
rust/target/
game/.godot/
game/build/linux.log
game/build/linux/
game/build/web/
game/reports/hp-local-setup/
tools/superpowers/
```

`git status --short` remained empty; `git status --short --ignored` named only
those build/cache/report roots. `game/override.cfg` was absent. No `.pck` was
loose beside the Linux player.

The recovery helper's unique transfer directory under `~/.cache/` was removed
after guarded source installation and directory fsync. The evidence root and
historical manifests remain. The download scratch is accounted for by the
pending rows in the table until its one reviewed cleanup; build artifacts are
not download scratch and remain untouched.

Task 4 also generated persistent user state outside the checkout. These paths
postdate Task 2's installed-root manifests and must not be described as files
installed by Task 2:

| Generated path/state | Exact post-build observation | Ownership / disposition |
| --- | --- | --- |
| `~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/` | Post-build owner `galchenko`, mode `0775`, mtime `2026-08-24 22:29:04 +0300`; 110 regular files / 202,399,219 bytes; `bin/cargo` existed and `bin/rustc` was absent | Broken partial tracking-toolchain install, not an approved pin. Read-only evidence indicates the timed-out 30-second `rustc +stable --version` verifier initiated it: the sealed list lacked this alias, the post-build list added it, and rustup download/tmp residue shared the timestamp. One separately approved remediation attempt removed it successfully on 2026-08-25; its sealed evidence is described below. |
| `~/.rustup/` overall | 5,316 metadata entries immediately after the build versus 5,186 in Task 2's sealed manifest; the pre-remediation list added `stable-x86_64-unknown-linux-gnu` beside nightly and active/default `1.97.1` | Mutable rustup state. The exact Task 2 installed-root manifest remains historical and is not rewritten. Final remediation verification found exactly the pinned `1.97.1-x86_64-unknown-linux-gnu` and `nightly-2026-05-25-x86_64-unknown-linux-gnu` directories. |
| `~/.cargo/registry/` | owner `galchenko`, mode `0775`; 2,152 regular files / 108,020,576 bytes, first created/modified in the 22:58 Task-4 Cargo build window | Normal Cargo download/cache state, not installed tool files. Preserve unless a separate cache cleanup is wanted. |
| `~/.cargo/.global-cache`, `.package-cache`, `.package-cache-mutate` | `.global-cache` is 57,344 bytes; both package-cache marker files are empty | Normal Cargo cache/database state. Never erase `~/.cargo` broadly. |
| `~/.cache/godot/` | mode `0775`; one 3,269,270-byte `editor_doc_cache-4.7.res`, mtime 23:01 | Godot-generated cache. |
| `~/.config/godot/` | mode `0775`; one 17,678-byte editor-settings file plus three empty directories, mtime 23:20 | Godot-generated configuration. |
| `~/.local/share/godot/app_userdata/Unseeing/` | mode `0775`; five log files / 3,485 bytes plus directories, first mtime 23:01 | Game-generated logs/user data. |
| `~/.cache/gdtoolkit/` | owner `galchenko`, mode `0775`; two parser-cache files / 505,736 bytes, mtime 23:01 | gdtoolkit 4.5.0-generated cache. |
| `~/.config/chromium/` | mode `0700`; one 40-byte file plus Crash Reports directories, mtime 23:19 | Chromium smoke-test state. |
| `~/.local/share/pki/` | owner `galchenko`, mode `0700`; three files / 65,998 bytes, mtime 23:19 | Chromium-created NSS database shape. Record metadata only: this location can hold shared sensitive state and has no broad deletion recommendation without a complete unchanged-boundary proof. |

The ordinary cache/config/log roots above may be removed only by an optional,
separately reviewed exact-root cleanup that first proves their complete current
trees still match a setup-owned manifest. The current ledger has only the
top-level facts needed to account for them, so it deliberately gives no
copy-paste deletion command. `~/.local/share/pki` requires the stronger shared-
state boundary just stated.

The completed stable-alias remediation has its own evidence root,
`/home/galchenko/.local/state/unseeing/remediation/2026-08-25-stable-timeout`,
outside the sealed setup ledger. On 2026-08-25 the controller invoked the
reviewed helper exactly once as
`/usr/bin/python3 /home/galchenko/.cache/unseeing-stable-remediation.8uakva/task-5-stable-remediation.py --live`;
it returned status `0` and `stable alias removed and remediation sealed`.
That helper made exactly one rustup command attempt, also status `0`:
`/usr/bin/env -u RUSTUP_TOOLCHAIN RUSTUP_AUTO_INSTALL=0 RUSTUP_HOME=/home/galchenko/.rustup CARGO_HOME=/home/galchenko/.cargo /home/galchenko/.cargo/bin/rustup toolchain uninstall stable-x86_64-unknown-linux-gnu`.
Its reviewed source SHA-256 is
`112b767f1e57b04d8eaeefbc0c8ead328e588829516885b8c2eee6361155ff58`.

Read-only verification reproduced the separate seal: 25 manifest entries, 27
total evidence files, and final-manifest SHA-256
`98865ea6f93416f0916ede7ce2f8b0a0bbbb116cd57b379f5db32be72178d0f2`.
The broken alias and its update hash are absent; rustup downloads/tmp remain as
intentional cache residue. Both pinned roots and their direct identities remain
exactly `rustc 1.97.1 (8bab26f4f 2026-07-14)` / `cargo 1.97.1
(c980f4866 2026-06-30)` and `rustc 1.98.0-nightly (423e3d252 2026-05-24)` /
`cargo 1.98.0-nightly (4d1f98451 2026-05-15)`. Task 2's original 490-entry
checkpoint remains byte-current at
`ad0a3c2626a4c7c85e8a0f04a7f15bffa0fd5affe1d7065c1da8f4b5fd272385`;
all Task-5 cleanup/final-seal outputs remain absent. Independent review then
approved and verified removal of the one guarded transfer directory
`/home/galchenko/.cache/unseeing-stable-remediation.8uakva`; the remediation
evidence root remains. The first read-only verifier had incorrectly required
empty rustup stderr; sealed evidence instead contained rustup's expected
`uninstalling` and `uninstalled` informational lines. Correcting that verifier
made the full read-only check pass, and no live command was repeated.

No APT source, known-host content after the audit baseline, global Git config,
credential, authentication state, game source, tracked checkout file, or
deployment setting changed. No PowerShell, agent CLI, second Godot package,
global emsdk activation, or external storage system was installed. The
optional Godot MCP addon was not installed by the completed 2026-08-24 base
setup/build tasks. It was later installed only in the isolated Task-4A proof
worktree, where the separately dated editor-only proof passed as recorded
above; the addon remains ignored, untracked, and export-independent. The wiki
was not edited or pushed by Tasks 2--5.

## Updating the environment deliberately

Do not turn a dated resolution into an implicit moving dependency.

1. Update the checked-in owner first in an isolated branch:
   `.godot-version` for Godot, `rust/rust-toolchain.toml` for native Rust,
   `rust/build-wasm.sh` for nightly/emsdk, or `README.md` and `ci/pipeline.sh`
   for gdtoolkit's supported range.
2. Review upstream release notes and official checksums/commit identity. Godot
   download hashes and mutable rustup/wheel hashes must be freshly captured.
3. Install beside the prior user-owned version where practical; do not
   overwrite an unverified path.
4. Regenerate installed-root manifests and repeat all four gates without Web
   skips.
5. Record the new date, exact resolved versions, hashes, paths, transaction,
   rollback, and review. Only then retire the old exact root.

Debian security upgrades may change Chromium and transitive package versions
inside the supported Debian 13 range. Capture the new APT transaction and
rerun the full Web smoke gate.

## Troubleshooting and security

- `bootstrap: FAILED` for Godot: compare `godot --version` with
  `.godot-version`, verify the relative `~/.local/bin/godot` link and templates
  directory, then rerun. Do not install a second package to mask a bad path.
- Missing Rust compiler/component/target: source `~/.cargo/env`, inspect
  `rust/rust-toolchain.toml`, and run `rustup show`. Web-only failures also
  require the exact nightly, `rust-src`, wasm target, and emsdk `4.0.20` from
  `rust/build-wasm.sh`.
- Emsdk environment failures: verify the checkout origin/commit and run its
  commands from `/home/galchenko/emsdk`. Do not dump the sourced environment;
  test `emcc --version` directly with setup output suppressed.
- Web smoke failures: confirm `chromium --version`, Python 3, and that neither
  `SKIP_EXPORT` nor `SKIP_SMOKE` is set. The smoke test's render verdict, not a
  screenshot, is the proof.
- Linux export missing its library: require both `game/build/linux/unseeing`
  and adjacent `libunseeing_core.so`; a loose `.pck` is not the approved shape.
- GitHub push rejected: builds are complete without authentication. The human
  user must configure `gh auth login` or an SSH key outside this ledger.
- An unexpected helper, cleanup, package, or build result is a stop condition.
  Preserve its output, form one hypothesis, and follow the repository-pinned
  systematic-debugging workflow. Do not retry destructive cleanup.

Keep the evidence root mode `0700`. It may contain public system paths and
command output but must never contain tokens, private keys, credentials, or a
raw environment dump. Treat official TLS plus published hashes as the Godot
and rustup trust boundary; the emsdk tag is not a signature, so bind it to the
reviewed official-remote commit.

## Guarded rollback by owner

Rollback is intentionally explicit. First archive the evidence root elsewhere
and verify its seal. Run only the section for the owner being reversed. Stop if
any canonical path, owner, type, mode, version, hash, or clean-checkout guard
fails; changed state needs a new review, not a broader deletion.

### Remove generated build output only

```sh
set -eu
repo=/home/galchenko/src/unseeing
test "$(realpath "$repo")" = /home/galchenko/src/unseeing
test ! -L "$repo" && test -d "$repo"
test "$(stat -c %U "$repo")" = galchenko
for path in "$repo/rust/target" "$repo/game/.godot" \
  "$repo/game/build/web" "$repo/game/build/linux" \
  "$repo/game/reports/hp-local-setup" "$repo/game/build/linux.log"; do
  if [ -e "$path" ] || [ -L "$path" ]; then
    test ! -L "$path"
    case "$(realpath "$path")" in "$repo"/*) ;; *) exit 2 ;; esac
    test "$(stat -c %U "$path")" = galchenko
  fi
done
rm -rf -- "$repo/rust/target" "$repo/game/.godot" \
  "$repo/game/build/web" "$repo/game/build/linux" \
  "$repo/game/reports/hp-local-setup"
rm -f -- "$repo/game/build/linux.log"
for path in "$repo/rust/target" "$repo/game/.godot" \
  "$repo/game/build/web" "$repo/game/build/linux" \
  "$repo/game/reports/hp-local-setup" "$repo/game/build/linux.log"; do
  test ! -e "$path" && test ! -L "$path"
done
```

These are generated ignored artifacts. This does not uninstall a prerequisite
or remove the clone.

### Uninstall gdtoolkit

```sh
set -eu
export PATH="$HOME/.local/bin:$PATH"
venv=/home/galchenko/.local/share/pipx/venvs/gdtoolkit
test "$(realpath "$venv")" = \
  /home/galchenko/.local/share/pipx/venvs/gdtoolkit
test ! -L "$venv" && test -d "$venv"
test "$(stat -c %U "$venv")" = galchenko
test "$("$venv/bin/python" -c \
  'from importlib.metadata import version; print(version("gdtoolkit"))')" = 4.5.0
pipx uninstall gdtoolkit
test ! -e "$venv" && test ! -L "$venv"
for link in gdformat gdlint gd2py gdparse gdradon; do
  test ! -e "/home/galchenko/.local/bin/$link" \
    && test ! -L "/home/galchenko/.local/bin/$link"
done
```

Pipx owns the environment and all five console links. Do not remove
`~/.local/bin` or the pipx root.

### Uninstall emsdk

The retained Task-2 `resume-1/manifest-emsdk-*` pair is historical. A Git
origin/HEAD check cannot account for ignored SDK payloads, later untracked
files, or modified tracked bytes, so it is not deletion authority. Run this
read-only preflight to identify the boundary, then stop:

```sh
set -eu
emsdk_root=/home/galchenko/emsdk
test "$(realpath "$emsdk_root")" = /home/galchenko/emsdk
test ! -L "$emsdk_root" && test -d "$emsdk_root"
test "$(stat -c %U "$emsdk_root")" = galchenko
test "$(git -C "$emsdk_root" remote get-url origin)" = \
  https://github.com/emscripten-core/emsdk.git
test "$(git -C "$emsdk_root" rev-parse HEAD)" = \
  e4fe26ef59168ff44f4c23c466e497bf60b3411e
git -C "$emsdk_root" status --short --untracked-files=all
find /home/galchenko/.local/state/unseeing/setup/2026-08-24 \
  -maxdepth 2 -type f -name 'manifest-emsdk-*' -print | LC_ALL=C sort
```

Deletion requires a newly reviewed, complete current-tree manifest that
matches the sealed roster by relative path, type, owner, mode, link count,
symlink target, size, and file SHA-256, followed by a manifest-specific owner
that removes only those entries and proves absence. No copy-paste deletion
command is provided for emsdk. Stop on any difference; do not broaden the
target. There is no global activation or `/home/galchenko/.emscripten` to
remove.

### Remove only the setup-owned Rust toolchain lanes

Do not run `rustup self uninstall` on this host. `~/.cargo` pre-existed, and
Task 4 subsequently created registry/cache/database state that is not part of
the Task 2 installation manifest. A broad uninstall would erase that later
state. The bounded rollback below asks rustup to remove only the two exact
toolchain lanes installed by Task 2; it deliberately retains rustup itself,
the installer-owned startup lines, the two pre-existing empty Cargo metadata
files, and all Cargo registry/cache state. The partial `stable` alias was
observed after Task 4 and removed exactly once by the separately sealed
2026-08-25 remediation. Only its rustup download/tmp cache residue remains; the
alias itself is absent and must not be uninstalled again.

```sh
set -eu
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
. "$HOME/.cargo/env"
rustup_root=/home/galchenko/.rustup
test "$(realpath "$rustup_root")" = /home/galchenko/.rustup
test ! -L "$rustup_root" && test -d "$rustup_root"
test "$(stat -c %U "$rustup_root")" = galchenko
rustup toolchain list | sed 's/ (.*$//' \
  | grep -Fxq '1.97.1-x86_64-unknown-linux-gnu'
rustup toolchain list | sed 's/ (.*$//' \
  | grep -Fxq 'nightly-2026-05-25-x86_64-unknown-linux-gnu'
for lane in \
  /home/galchenko/.rustup/toolchains/1.97.1-x86_64-unknown-linux-gnu \
  /home/galchenko/.rustup/toolchains/nightly-2026-05-25-x86_64-unknown-linux-gnu; do
  test "$(realpath "$lane")" = "$lane"
  test ! -L "$lane" && test -d "$lane"
  test "$(stat -c %U "$lane")" = galchenko
done
rustup toolchain uninstall 1.97.1-x86_64-unknown-linux-gnu
rustup toolchain uninstall nightly-2026-05-25-x86_64-unknown-linux-gnu
test ! -e /home/galchenko/.rustup/toolchains/1.97.1-x86_64-unknown-linux-gnu \
  && test ! -L /home/galchenko/.rustup/toolchains/1.97.1-x86_64-unknown-linux-gnu
test ! -e /home/galchenko/.rustup/toolchains/nightly-2026-05-25-x86_64-unknown-linux-gnu \
  && test ! -L /home/galchenko/.rustup/toolchains/nightly-2026-05-25-x86_64-unknown-linux-gnu
```

This is intentionally a partial tool rollback, not a rustup-state cleanup. The
broken partial stable alias was already removed by the separate sealed
2026-08-25 remediation recorded above. Do not attempt that uninstall again,
manually alter its retained cache residue, or conflate its evidence root with
the setup evidence root.

### Remove Godot and templates

The editor and template manifests are likewise historical. Version output and
top-level ownership do not prove that the executable bytes, 35-file template
roster, or link boundary remain unchanged. This is a read-only identity
preflight, not removal authorization:

```sh
set -eu
editor=/home/galchenko/.local/bin/Godot_v4.7.1-stable_linux.x86_64
link=/home/galchenko/.local/bin/godot
templates=/home/galchenko/.local/share/godot/export_templates/4.7.1.stable
test "$(realpath "$editor")" = "$editor"
test ! -L "$editor" && test -f "$editor"
test "$(stat -c %U "$editor")" = galchenko
test "$(stat -c %a "$editor")" = 755
test "$($editor --version)" = 4.7.1.stable.official.a13da4feb
test -L "$link"
test "$(readlink "$link")" = Godot_v4.7.1-stable_linux.x86_64
test "$(realpath "$templates")" = "$templates"
test ! -L "$templates" && test -d "$templates"
test "$(stat -c %U "$templates")" = galchenko
find /home/galchenko/.local/state/unseeing/setup/2026-08-24 \
  -maxdepth 2 -type f \
  \( -name 'manifest-godot-editor-*' -o -name 'manifest-godot-templates-*' \) \
  -print | LC_ALL=C sort
```

Deletion requires a newly reviewed, complete current-tree manifest that
matches both sealed manifest pairs by relative path, type, owner, mode, link
count, symlink target, size, and file SHA-256. A manifest-specific owner must
then unlink only the exact versioned binary, relative link, and validated
template members and prove absence. No copy-paste deletion command is provided
for Godot or its templates. Stop on any difference instead of deleting a whole
root.

### Remove the later network-time package only

Do this only after every remote proof and SSH/editor lifecycle has ended.
Removing network time reintroduces the clock-risk that stopped MCP attempt 1.
The later package transaction is independent of the original 11-package Web
transaction below, so simulate and reverse it separately:

```sh
set -eu
: "${UNSEEING_REMOVE_TIMESYNCD:?set to yes after reviewing the simulation}"
test "$UNSEEING_REMOVE_TIMESYNCD" = yes
test "$(dpkg-query -W -f='${Version}' systemd-timesyncd)" = \
  257.13-1~deb13u1
simulation_root=$(mktemp -d /tmp/unseeing-timesyncd-rollback.XXXXXX)
trap 'find "$simulation_root" -depth -delete' EXIT HUP INT TERM
LC_ALL=C sudo apt-get -s remove --purge systemd-timesyncd \
  > "$simulation_root/apt-simulation.txt"
test -z "$(awk '/^Inst / { print }' "$simulation_root/apt-simulation.txt")"
test "$(awk '/^Remv / { print $2 }' "$simulation_root/apt-simulation.txt")" = \
  systemd-timesyncd
cat "$simulation_root/apt-simulation.txt"
sudo apt-get remove --purge -y systemd-timesyncd
! dpkg-query -W -f='${db:Status-Abbrev}' systemd-timesyncd 2>/dev/null \
  | grep -q '^ii '
sudo systemctl restart systemd-timedated.service
find "$simulation_root" -depth -delete
trap - EXIT HUP INT TERM
```

Stop if simulation proposes any other removal or any installation. Do not
delete service users, D-Bus state, package indexes, or cache paths manually;
APT and systemd own them.

### Restore the Debian package/manual boundary

Review `apt-new-packages.stdout.txt` again immediately before removal. The
single simulated transaction below names only the two setup-owned manual roots
(`brotli` and `chromium`) and asks APT for explicit autoremove semantics. It
must propose exactly the recorded 11-package set—no later reverse dependency
or unrelated orphan—before the real transaction can run.

```sh
set -eu
for pair in \
  'avahi-utils=0.8-16' \
  'brotli=1.1.0-2+b7' \
  'chromium=151.0.7922.169-1~deb13u1' \
  'chromium-common=151.0.7922.169-1~deb13u1' \
  'chromium-sandbox=151.0.7922.169-1~deb13u1' \
  'gir1.2-handy-1:amd64=1.8.3-2' \
  'gir1.2-packagekitglib-1.0=1.3.1-1+deb13u1' \
  'libdouble-conversion3:amd64=3.3.1-1' \
  'libminizip1t64:amd64=1:1.3.dfsg+really1.3.1-1+b1' \
  'libxnvctrl0:amd64=535.171.04-1+b2' \
  'system-config-printer=1.5.18-4'; do
  package=${pair%%=*}
  version=${pair#*=}
  test "$(dpkg-query -W -f='${Version}' "$package")" = "$version"
done
test "$(apt-mark showmanual | grep -Ec '^(brotli|chromium)$')" -eq 2
simulation_root=$(mktemp -d /tmp/unseeing-apt-rollback.XXXXXX)
trap 'find "$simulation_root" -depth -delete' EXIT HUP INT TERM
cat > "$simulation_root/expected-removals.txt" <<'EOF'
avahi-utils
brotli
chromium
chromium-common
chromium-sandbox
gir1.2-handy-1:amd64
gir1.2-packagekitglib-1.0
libdouble-conversion3:amd64
libminizip1t64:amd64
libxnvctrl0:amd64
system-config-printer
EOF
LC_ALL=C sort -o "$simulation_root/expected-removals.txt" \
  "$simulation_root/expected-removals.txt"
sudo apt-get -s autoremove --purge brotli chromium \
  > "$simulation_root/apt-simulation.txt"
awk '/^Remv / { print $2 }' "$simulation_root/apt-simulation.txt" \
  | LC_ALL=C sort > "$simulation_root/proposed-removals.txt"
diff -u "$simulation_root/expected-removals.txt" \
  "$simulation_root/proposed-removals.txt"
cat "$simulation_root/apt-simulation.txt"
sudo apt-get autoremove --purge -y brotli chromium
for package in $(cat "$simulation_root/expected-removals.txt"); do
  if dpkg-query -W -f='${db:Status-Abbrev}' "$package" 2>/dev/null \
    | grep -q '^ii '; then
    exit 2
  fi
done
find "$simulation_root" -depth -delete
trap - EXIT HUP INT TERM
```

Do not remove package indexes or cache by filesystem deletion. APT owns that
metadata.

### Remove the known-host audit side effect

The audit added one exact public GitHub line, but `known_hosts` is shared SSH
state. A prior recipe checked one pathname state and later replaced another;
that could overwrite a concurrent update. Use this read-only check only:

```sh
set -eu
known=/home/galchenko/.ssh/known_hosts
test "$(realpath "$known")" = "$known"
test ! -L "$known" && test -f "$known"
test "$(stat -c %U "$known")" = galchenko
test "$(sha256sum "$known" | awk '{print $1}')" = \
  b6d915f0f612aa8aae124f43d5d9c0b073bc0f879e6079af77f414304c0e0f4f
python3 - "$known" <<'PY'
from pathlib import Path
import sys
path = Path(sys.argv[1])
entry = b'|1|GwndCqREKcg/Be7wNbSP6SC7HTU=|s3DGEAoSaOqaNMDMOdQWeCc3H6Y= ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl\n'
data = path.read_bytes()
if data.count(entry) != 1:
    raise SystemExit("exact known-host line guard failed")
print("known-host rollback preflight only: exact dated file and line present")
PY
```

A future owner must open the source with `O_NOFOLLOW`, bind `lstat`/`fstat`
identity and complete bytes, create a recovery copy and replacement
exclusively, and revalidate the live pathname's inode and complete bytes
immediately before atomic replacement. It must preserve both states on a
conflict and fsync the parent directory. No copy-paste mutation command is
provided. Authentication credentials were never installed, so there is no
credential rollback.

### Remove the durable clone

The durable clone owns the common Git directory for every linked worktree.
First run each linked worktree's owning teardown: stop its exact
editor/unit/client/tunnel, restore `project.godot`, remove only
manifest-authorized ignored output, then use non-forced `git worktree remove`,
prove both path and admin entry absent, and use `git branch -d` only when it has
no unique commit. Never manually recurse through `.worktrees/` or shared Git
administration. The following is a read-only sole-worktree preflight:

```sh
set -eu
repo=/home/galchenko/src/unseeing
test "$(realpath "$repo")" = /home/galchenko/src/unseeing
test ! -L "$repo" && test -d "$repo"
test "$(stat -c %U "$repo")" = galchenko
test "$(git -C "$repo" remote get-url origin)" = \
  https://github.com/cleveralbatraoz/unseeing.git
test "$(git -C "$repo" rev-parse HEAD)" = \
  d6285b0bba84dd29846a9613c2e8081191e46cfd
test -z "$(git -C "$repo" status --short)"
test "$(git -C "$repo" rev-parse --path-format=absolute --git-common-dir)" = \
  /home/galchenko/src/unseeing/.git
worktrees=$(git -C "$repo" worktree list --porcelain \
  | awk '$1 == "worktree" { print $2 }')
test "$(printf '%s\n' "$worktrees" | sed '/^$/d' | wc -l)" -eq 1
test "$worktrees" = /home/galchenko/src/unseeing
git -C "$repo" worktree list --porcelain
git -C "$repo" status --short --untracked-files=all --ignored
```

Even a sole remaining worktree is not deletion authority: clean tracked status
does not account for ignored build output, addons, evidence, or later user
files. Require a complete approved manifest of every remaining path and a
separately reviewed clone owner before removal. No recursive clone deletion
command is provided. Preserve the clone and stop if the worktree roster or any
current path differs.

### Remove the retained evidence last

The evidence root is the rollback proof, so keep it unless an independently
verified archive exists. This block is deliberately fail-closed during Phase A:
`UNSEEING_FINAL_GUIDE` must name the reviewed Phase-B-complete guide, whose
completion-table row contains the exact final-manifest SHA-256. It verifies
that independent literal, the digest record, and a full read-only manifest
regeneration before deletion. The unresolved Phase-A row cannot pass.

```sh
set -eu
: "${UNSEEING_FINAL_GUIDE:?absolute path to the reviewed Phase-B-complete guide}"
evidence=/home/galchenko/.local/state/unseeing/setup/2026-08-24
guide=$(realpath "$UNSEEING_FINAL_GUIDE")
test ! -L "$UNSEEING_FINAL_GUIDE" && test -f "$guide"
test "$(realpath "$evidence")" = "$evidence"
test ! -L "$evidence" && test -d "$evidence"
test "$(stat -c %U:%a "$evidence")" = galchenko:700
manifest="$evidence/task-5-final-evidence-seal.sha256.txt"
digest_record="$evidence/task-5-final-evidence-seal.digest.txt"
test ! -L "$manifest" && test -f "$manifest"
test ! -L "$digest_record" && test -f "$digest_record"
test "$(stat -c %U:%a "$manifest")" = galchenko:600
test "$(stat -c %U:%a "$digest_record")" = galchenko:600
expected=$(sed -n \
  's/^| Final manifest full SHA-256 and digest record | `\([0-9a-f]\{64\}\)`.*/\1/p' \
  "$guide")
case "$expected" in
  [0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f]* ) ;;
  *) exit 2 ;;
esac
test "${#expected}" -eq 64
test "$(sha256sum "$manifest" | awk '{print $1}')" = "$expected"
test "$(cat "$digest_record")" = "$expected  $(basename "$manifest")"
python3 - "$evidence" "$manifest" "$digest_record" <<'PY'
import hashlib, os, pwd, stat, sys
from pathlib import Path
root, manifest, digest_record = map(Path, sys.argv[1:])
excluded = {manifest.name, digest_record.name}
expected_uid = pwd.getpwnam("galchenko").pw_uid
rows = []
for path in root.rglob("*"):
    metadata = path.lstat()
    relative = path.relative_to(root).as_posix()
    if stat.S_ISLNK(metadata.st_mode):
        raise SystemExit(f"seal tree contains symlink: {relative}")
    if stat.S_ISDIR(metadata.st_mode):
        continue
    if not stat.S_ISREG(metadata.st_mode):
        raise SystemExit(f"seal tree contains unsupported type: {relative}")
    if metadata.st_uid != expected_uid or metadata.st_nlink != 1:
        raise SystemExit(f"seal file identity mismatch: {relative}")
    if relative in excluded:
        continue
    value = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            value.update(block)
    rows.append((relative.encode(), f"{value.hexdigest()}  {relative}\n"))
regenerated = "".join(row for _, row in sorted(rows)).encode()
if manifest.read_bytes() != regenerated:
    raise SystemExit("final manifest regeneration mismatch")
PY
rm -rf -- /home/galchenko/.local/state/unseeing/setup/2026-08-24
test ! -e "$evidence" && test ! -L "$evidence"
```

Parent directories under `~/.local/state/unseeing/setup` were not individually
proven new, so this rollback does not remove them.
