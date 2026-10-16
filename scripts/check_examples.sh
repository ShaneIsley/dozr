#!/usr/bin/env bash
# Checks that every dozr command shown in README.md and examples.md parses
# and validates.
#
# Commands come from fenced ```bash blocks and from inline code in Markdown
# tables. Each runs with its probability replaced by `-p 0`, so dozr checks
# all arguments and then skips the wait. A documented probability value is
# therefore not itself checked.
#
# Usage: scripts/check_examples.sh [FILE...]
# Set DOZR to test a specific binary (default: target/debug/dozr, built if missing).
set -euo pipefail

cd "$(dirname "$0")/.."
DOZR=${DOZR:-target/debug/dozr}
if [[ ! -x "$DOZR" ]]; then
    cargo build --quiet
fi
files=("$@")
[[ ${#files[@]} -gt 0 ]] || files=(README.md examples.md)

extract() {
    awk '
        /^```bash/ { in_block = 1; next }
        /^```/     { in_block = 0; next }
        in_block && /^[[:space:]]*dozr / { print; next }
        /^\|/ {
            line = $0
            while (match(line, /`dozr [^`]*`/)) {
                print substr(line, RSTART + 1, RLENGTH - 2)
                line = substr(line, RSTART + RLENGTH)
            }
        }
    ' "$@"
}

checked=0
failed=0
while IFS= read -r line; do
    # Drop comments, redirections, and pipelines, then the leading "dozr".
    cmd=$(sed -E 's/[[:space:]]#.*$//; s/[[:space:]]*[0-9]*>.*$//; s/[[:space:]]*\|.*$//' <<<"$line")
    cmd=$(sed -E 's/^[[:space:]]*dozr[[:space:]]+//; s/(^|[[:space:]])(-p|--probability)[[:space:]]+[^[:space:]]+//g' <<<"$cmd")
    read -ra args <<<"$cmd"
    checked=$((checked + 1))
    if ! output=$("$DOZR" "${args[@]}" -p 0 2>&1 >/dev/null); then
        failed=$((failed + 1))
        echo "FAIL: $line"
        sed 's/^/    /' <<<"$output"
    fi
done < <(extract "${files[@]}" | sort -u)

if [[ $checked -eq 0 ]]; then
    echo "No dozr commands found in: ${files[*]}" >&2
    exit 1
fi
echo "Checked $checked documented commands, $failed failed."
[[ $failed -eq 0 ]]
