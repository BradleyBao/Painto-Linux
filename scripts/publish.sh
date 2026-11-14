#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat <<'USAGE'
Usage: scripts/publish.sh VERSION "RELEASE DESCRIPTION"

Example:
  scripts/publish.sh 0.2.1 "Flatpak metadata corrections and toolbar fixes."

Creates two local commits and an immutable local tag. It does not push or
create a GitHub Release. The existing cargo-sources.json is reused when
external Cargo dependencies are unchanged. If they change, set
FLATPAK_CARGO_GENERATOR to the official flatpak-cargo-generator.py path
and PYTHON_BIN to a Python interpreter with aiohttp and tomlkit installed.
USAGE
}

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
    usage
    exit 0
fi
if [[ $# -ne 2 || ! "$1" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ || -z "$2" ]]; then
    usage >&2
    exit 2
fi

version=$1
description=$2
tag="v$version"
repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"

if [[ -n "$(git status --porcelain)" ]]; then
    printf 'Worktree is not clean. Commit your current changes before publishing.\n' >&2
    exit 1
fi
if git show-ref --verify --quiet "refs/tags/$tag"; then
    printf 'Tag %s already exists. Choose a new version.\n' "$tag" >&2
    exit 1
fi
branch=$(git branch --show-current)
if [[ -z "$branch" ]]; then
    printf 'Publishing requires a checked-out branch.\n' >&2
    exit 1
fi
git var GIT_AUTHOR_IDENT >/dev/null
git var GIT_COMMITTER_IDENT >/dev/null

python_bin=${PYTHON_BIN:-python3}
"$python_bin" -c 'import tomllib' || {
    printf 'The selected Python must provide tomllib (Python 3.11 or newer).\n' >&2
    exit 1
}
if [[ ! -s flatpak/cargo-sources.json ]]; then
    printf 'Generate flatpak/cargo-sources.json before publishing.\n' >&2
    exit 1
fi
external_deps() {
    "$python_bin" -c 'import json,sys,tomllib; p=tomllib.load(open(sys.argv[1],"rb"))["package"]; print(json.dumps(sorted((x["name"],x["version"],x.get("source"),x.get("checksum")) for x in p if x.get("source"))))' Cargo.lock
}
old_external_deps=$(external_deps)

export PAINTO_RELEASE_VERSION="$version"
export PAINTO_RELEASE_DESCRIPTION="$description"
PAINTO_RELEASE_DATE=$(date +%F)
export PAINTO_RELEASE_DATE

"$python_bin" - <<'PY'
import os
import re
import tomllib
from pathlib import Path
from xml.etree import ElementTree as ET
from xml.sax.saxutils import escape

version = os.environ["PAINTO_RELEASE_VERSION"]
description = os.environ["PAINTO_RELEASE_DESCRIPTION"]
date = os.environ["PAINTO_RELEASE_DATE"]
manifest = Path("Cargo.toml")
metadata = Path("data/com.tianyibrad.Painto.metainfo.xml")

cargo_text = manifest.read_text()
cargo = tomllib.loads(cargo_text)
if cargo["package"]["version"] == version:
    raise SystemExit(f"Cargo.toml is already at {version}")
package_section, marker, remaining = cargo_text.partition("[dependencies]")
if not marker:
    raise SystemExit("Could not find Cargo.toml dependencies section")
package_section, count = re.subn(
    r'(?m)^(version\s*=\s*)"[^"]+"$',
    lambda match: f'{match.group(1)}"{version}"',
    package_section,
)
if count != 1:
    raise SystemExit("Expected one package version in Cargo.toml")

xml_text = metadata.read_text()
root = ET.fromstring(xml_text)
releases = root.find("releases")
if releases is None or any(item.get("version") == version for item in releases):
    raise SystemExit("Missing releases element or version already present in AppStream metadata")
if xml_text.count("<releases>") != 1:
    raise SystemExit("Expected one AppStream releases element")
release = (
    f'  <releases>\n    <release version="{version}" date="{date}" type="development">\n'
    f'      <description>\n        <p>{escape(description)}</p>\n'
    f'      </description>\n    </release>'
)
manifest.write_text(package_section + marker + remaining)
metadata.write_text(xml_text.replace("  <releases>", release, 1))
PY

cargo update --offline -p painto-linux
if [[ "$(external_deps)" != "$old_external_deps" ]]; then
    generator=${FLATPAK_CARGO_GENERATOR:-}
    if [[ -z "$generator" ]]; then
        generator=$(command -v flatpak-cargo-generator.py || true)
    fi
    if [[ -z "$generator" || ! -f "$generator" ]]; then
        printf 'External dependencies changed. Set FLATPAK_CARGO_GENERATOR to the official generator path.\n' >&2
        exit 1
    fi
    "$python_bin" -c 'import aiohttp, tomlkit' || {
        printf 'The selected Python needs aiohttp and tomlkit to regenerate Cargo sources.\n' >&2
        exit 1
    }
    "$python_bin" "$generator" Cargo.lock -o flatpak/cargo-sources.json
fi
bash -e ./scripts/check.sh
appstreamcli validate --no-net data/com.tianyibrad.Painto.metainfo.xml
git diff --check

git add Cargo.toml Cargo.lock data/com.tianyibrad.Painto.metainfo.xml flatpak/cargo-sources.json
git commit -m "Release $tag"
git tag "$tag"
release_commit=$(git rev-parse "${tag}^{commit}")

export PAINTO_RELEASE_TAG="$tag"
export PAINTO_RELEASE_COMMIT="$release_commit"
"$python_bin" - <<'PY'
import os
import re
from pathlib import Path

path = Path("flatpak/com.tianyibrad.Painto.yml")
text = path.read_text()
url = "https://github.com/BradleyBao/Painto-Linux.git"
pattern = re.compile(
    rf"(?m)^(?P<indent>[ ]*)url: {re.escape(url)}\n"
    rf"(?P=indent)(?:branch|tag): [^\n]+\n"
    rf"(?:(?P=indent)commit: [0-9a-f]{{40}}\n)?"
)
def pin(match):
    indent = match.group("indent")
    return (
        f"{indent}url: {url}\n"
        f"{indent}tag: {os.environ['PAINTO_RELEASE_TAG']}\n"
        f"{indent}commit: {os.environ['PAINTO_RELEASE_COMMIT']}\n"
    )
updated, count = pattern.subn(pin, text)
if count != 1:
    raise SystemExit("Could not find exactly one Painto Git source in Flatpak manifest")
path.write_text(updated)
PY

git diff --check
git add flatpak/com.tianyibrad.Painto.yml
git commit -m "Pin Flatpak source to $tag"

printf '\nCreated local tag %s at %s. Push the branch and tag with:\n' "$tag" "$release_commit"
printf '  git push --atomic origin HEAD:%s refs/tags/%s\n' "$branch" "$tag"
