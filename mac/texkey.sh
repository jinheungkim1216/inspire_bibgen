#!/bin/zsh
# INSPIRE texkey — turn arXiv IDs, DOIs, INSPIRE record IDs or URLs into
# INSPIRE texkeys. Runs inside an Automator Quick Action (see install.sh).
#
# MODE (env var, or first argument when run by hand):
#   replace    stdin = selected text, stdout = replacement text
#   clipboard  input = clipboard, result copied back to the clipboard
#
# The action must never exit non-zero: Automator would pop an error dialog.
# On failure the selection is returned unchanged and a notification says why.

MODE="${MODE:-${1:-replace}}"
BIN="$HOME/.cargo/bin/inspire_bibgen"
[[ -x "$BIN" ]] || BIN="$(command -v inspire_bibgen 2>/dev/null)"

notify() { # $1 = subtitle, $2 = body
  local sub="${1//\"/\\\"}" body="${2//\"/\\\"}"
  osascript -e "display notification \"$body\" with title \"INSPIRE texkey\" subtitle \"$sub\"" >/dev/null 2>&1
}
give_up() { # $1 = subtitle, $2 = body — notify and leave the selection as it was
  notify "$1" "$2"
  [[ "$MODE" == replace ]] && printf '%s' "$input"
  exit 0
}

if [[ "$MODE" == clipboard ]]; then input="$(pbpaste)"; else input="$(cat)"; fi

# Newlines and semicolons also separate identifiers; whitespace never matters.
ids="$(printf '%s' "$input" | tr '\n;' ',,' | tr -d '[:space:]' | sed -E 's/,+/,/g; s/^,//; s/,$//')"

[[ -n "$BIN" ]] || give_up "inspire_bibgen not found" "Install it with: cargo install inspire_bibgen"
[[ -n "$ids" ]] || give_up "Nothing to look up" "Select or copy an arXiv ID, DOI or URL first"

err="$(mktemp)" || give_up "Could not create a temp file" "mktemp failed"
keys="$("$BIN" --key "$ids" 2>"$err")"
# IDs INSPIRE could not resolve, taken from the CLI's "left out" summary.
failed_raw="$(sed -nE 's/^[[:space:]]+- ([^[:space:]]+)[[:space:]]+\(.*$/\1/p' "$err")"
rm -f "$err"
failed=()
[[ -n "$failed_raw" ]] && failed=("${(@f)failed_raw}")

[[ -n "$keys" ]] || give_up "Not found on INSPIRE" "$ids"

# Rebuild the list in the original order: resolved IDs become texkeys,
# unresolved ones stay as they were so nothing is lost from the document.
key_list=("${(@f)keys}")
out=()
n=1
for id in "${(@s:,:)ids}"; do
  if (( ${failed[(Ie)$id]} )); then
    out+=("$id")
  else
    out+=("${key_list[$n]}")
    (( n++ ))
  fi
done
result="${(j:,:)out}"

if [[ "$MODE" == clipboard ]]; then
  printf '%s' "$result" | pbcopy
  notify "Copied to clipboard" "$result"
else
  printf '%s' "$result"
fi
if (( ${#failed} )); then
  notify "${#failed} not found, left unchanged" "${(j:, :)failed}"
fi
exit 0
