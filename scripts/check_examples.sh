#!/usr/bin/env bash
# Checks that every dozr command shown in README.md and examples.md parses
# and validates.
#
# Commands come from fenced ```bash blocks and from inline code in Markdown
# tables. Each runs exactly as written plus `--dry-run`, so dozr parses and
# validates every argument, prints the duration it picked, and does not wait.
# A command passes if it exits 0, or with the status it gives to
# --skip-exit-code. Commands that use shell variables are skipped.
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
skipped=0
while IFS= read -r line; do
    if [[ $line == *'$'* ]]; then
        skipped=$((skipped + 1))
        continue
    fi
    # Drop comments, redirections, and pipelines, then the leading "dozr".
    cmd=$(sed -E 's/[[:space:]]#.*$//; s/[[:space:]]*[0-9]*>.*$//; s/[[:space:]]*\|.*$//' <<<"$line")
    cmd=$(sed -E 's/^[[:space:]]*dozr[[:space:]]+//' <<<"$cmd")
    read -ra args <<<"$cmd"
    [[ " ${args[*]} " == *" --dry-run "* ]] || args+=(--dry-run)
    allowed=0
    if [[ $cmd =~ --skip-exit-code[[:space:]]+([0-9]+) ]]; then
        allowed=${BASH_REMATCH[1]}
    fi
    checked=$((checked + 1))
    status=0
    output=$("$DOZR" "${args[@]}" 2>&1 >/dev/null) || status=$?
    if [[ $status -ne 0 && $status -ne $allowed ]]; then
        failed=$((failed + 1))
        echo "FAIL (exit $status): $line"
        sed 's/^/    /' <<<"$output"
    fi
done < <(extract "${files[@]}" | sort -u)

if [[ $checked -eq 0 ]]; then
    echo "No dozr commands found in: ${files[*]}" >&2
    exit 1
fi
echo "Checked $checked documented commands, $failed failed, $skipped skipped for using shell variables."
[[ $failed -eq 0 ]]
