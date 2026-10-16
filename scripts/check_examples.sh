#!/usr/bin/env bash
# Checks that every dozr command shown in the documentation parses and
# validates.
#
# Commands are taken from fenced ```bash and ```sh blocks (with or without a
# leading "$ " prompt) and from backticked `dozr …` spans on any line. A
# backticked span needs at least two words after "dozr", so a bare mention
# such as `dozr d` in prose is not treated as a command.
#
# Each command runs exactly as written plus `--dry-run`, so dozr parses and
# validates every argument and prints the duration it picked without
# waiting. A command passes when it exits 0, or when it exits with its own
# documented --skip-exit-code AND printed "0s", which is what a skipped wait
# prints. A usage error that happens to share that status therefore fails.
# Commands that use shell variables are skipped.
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
        /^```(bash|sh)[[:space:]]*$/ { in_block = 1; next }
        /^```/                       { in_block = 0; next }
        in_block {
            line = $0
            sub(/^[[:space:]]*\$[[:space:]]+/, "", line)
            if (line ~ /^[[:space:]]*dozr /) print line
            next
        }
        {
            line = $0
            while (match(line, /`dozr [^`]*`/)) {
                span = substr(line, RSTART + 1, RLENGTH - 2)
                n = split(span, words, /[[:space:]]+/)
                if (n >= 3) print span
                line = substr(line, RSTART + RLENGTH)
            }
        }
    ' "$@"
}

checked=0
failed=0
skipped=0
shell_variable='\$[A-Za-z_{(]'
while IFS= read -r line; do
    if [[ $line =~ $shell_variable ]]; then
        skipped=$((skipped + 1))
        continue
    fi
    # Drop comments, redirections, and pipelines, then the leading "dozr".
    cmd=$(sed -E 's/[[:space:]]#.*$//; s/[[:space:]]*[0-9]*>.*$//; s/[[:space:]]*\|.*$//' <<<"$line")
    cmd=$(sed -E 's/^[[:space:]]*dozr[[:space:]]+//' <<<"$cmd")
    read -ra args <<<"$cmd"
    [[ " ${args[*]} " == *" --dry-run "* ]] || args+=(--dry-run)
    allowed=""
    if [[ $cmd =~ --skip-exit-code[[:space:]]+([0-9]+) ]]; then
        allowed=${BASH_REMATCH[1]}
    fi
    checked=$((checked + 1))
    status=0
    stdout=$("$DOZR" "${args[@]}" 2>/tmp/dozr-check-stderr.$$) || status=$?
    stderr=$(cat /tmp/dozr-check-stderr.$$)
    ok=0
    if [[ $status -eq 0 ]]; then
        ok=1
    elif [[ -n $allowed && $status -eq $allowed && $stdout == "0s" ]]; then
        ok=1
    fi
    if [[ $ok -eq 0 ]]; then
        failed=$((failed + 1))
        echo "FAIL (exit $status): $line"
        sed 's/^/    /' <<<"$stderr"
    fi
done < <(extract "${files[@]}" | sort -u)
rm -f /tmp/dozr-check-stderr.$$

if [[ $checked -eq 0 ]]; then
    echo "No dozr commands found in: ${files[*]}" >&2
    exit 1
fi
echo "Checked $checked documented commands, $failed failed, $skipped skipped for using shell variables."
[[ $failed -eq 0 ]]
