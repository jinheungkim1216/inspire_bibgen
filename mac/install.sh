#!/bin/zsh
# Build and install the two "INSPIRE texkey" Quick Actions into
# ~/Library/Services and bind their keyboard shortcuts.
#
#   INSPIRE texkey                  select text anywhere → replaced by texkeys   (⌃⌘K)
#   INSPIRE texkey from clipboard   clipboard → texkeys back on the clipboard    (⌃⇧⌘K)
#
# Usage:  mac/install.sh            install / update
#         mac/install.sh --uninstall
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
[[ -n "${HOME:-}" && -d "$HOME" ]] || { echo "HOME is not set" >&2; exit 1; }
services="$HOME/Library/Services"
replace_name="INSPIRE texkey"
clip_name="INSPIRE texkey from clipboard"
pbs=/System/Library/CoreServices/pbs

# Delete one of OUR bundles and nothing else. Every check must pass:
# a plain name, the exact expected path, and a directory (or nothing) there.
remove_bundle() { # $1 = service name
  local name="$1" dest="$services/$1.workflow"
  [[ "$name" != */* && -n "$name" ]] || { echo "refusing to remove: bad name '$name'" >&2; exit 1; }
  [[ "$dest" == "$HOME/Library/Services/"*.workflow ]] || { echo "refusing to remove: $dest" >&2; exit 1; }
  [[ ! -e "$dest" || -d "$dest" ]] || { echo "refusing to remove non-directory: $dest" >&2; exit 1; }
  [[ -e "$dest" ]] && rm -r "$dest"
  return 0
}

if [[ "${1:-}" == "--uninstall" ]]; then
  remove_bundle "$replace_name"
  remove_bundle "$clip_name"
  "$pbs" -flush
  echo "Removed both Quick Actions. Their two shortcut entries in the Services"
  echo "settings are left in place; they are inert without the bundles."
  exit 0
fi

[[ -x "$HOME/.cargo/bin/inspire_bibgen" ]] || command -v inspire_bibgen >/dev/null \
  || { echo "inspire_bibgen is not installed (cargo install --path .)" >&2; exit 1; }

build() { # $1 = mode, $2 = service name
  local mode="$1" name="$2" dest="$services/$2.workflow"
  remove_bundle "$name"
  cp -R "$here/template.workflow" "$dest"
  local info="$dest/Contents/Info.plist" doc="$dest/Contents/document.wflow"
  local script
  script="MODE=$mode"$'\n'"$(cat "$here/texkey.sh")"
  plutil -replace actions.0.action.ActionParameters.COMMAND_STRING -string "$script" "$doc"
  plutil -replace NSServices.0.NSMenuItem.default -string "$name" "$info"
  if [[ "$mode" == clipboard ]]; then
    # No input, no output: a plain menu item that works with nothing selected.
    plutil -remove NSServices.0.NSSendTypes "$info"
    plutil -remove NSServices.0.NSReturnTypes "$info"
    for k in inputTypeIdentifier outputTypeIdentifier serviceInputTypeIdentifier serviceOutputTypeIdentifier; do
      plutil -replace "workflowMetaData.$k" -string com.apple.Automator.nothing "$doc"
    done
    plutil -replace workflowMetaData.processesInput -bool false "$doc"
    plutil -replace workflowMetaData.serviceProcessesInput -bool false "$doc"
  fi
  plutil -lint "$info" "$doc" >/dev/null
  echo "Installed: $dest"
}

mkdir -p "$services"
build replace   "$replace_name"
build clipboard "$clip_name"

# Keyboard shortcuts (System Settings › Keyboard › Shortcuts › Services).
# -dict-add touches only our own entry; every other service keeps its settings.
# Modifier letters: ^ control, ~ option, $ shift, @ command.
# ⌃⌘ is used because window managers such as Rectangle grab most ⌃⌥+letter
# combinations system-wide before any app's Services menu can see them.
bind() { # $1 = service name, $2 = key equivalent
  # The key holds spaces and parentheses, so it must be a quoted plist string.
  defaults write pbs NSServicesStatus -dict-add "\"(null) - $1 - runWorkflowAsService\"" \
    "{ enabled_context_menu = 1; enabled_services_menu = 1; key_equivalent = \"$2\"; \
       presentation_modes = { ContextMenu = 1; ServicesMenu = 1; }; }"
}
bind "$replace_name" '^@k'
bind "$clip_name"    '^$@k'
"$pbs" -flush
"$pbs" -update 2>/dev/null || true

cat <<EOF

Done. Shortcuts: ⌃⌘K  = replace selected IDs with texkeys
                 ⌃⇧⌘K = look up the clipboard, copy texkeys back
If a shortcut does not fire, log out and back in once, or set it by hand in
System Settings › Keyboard › Keyboard Shortcuts › Services › Text.
EOF
