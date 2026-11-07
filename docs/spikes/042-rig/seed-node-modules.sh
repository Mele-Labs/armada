#!/usr/bin/env bash
# The clone's `bootstrap` Command, run by Fleet in a fresh slot: copy-on-write
# copies of a template tree's node_modules, so a slot can run vitest without a
# network install. pnpm's links are relative, so each copy resolves inside the slot.
#   seed-node-modules.sh <template-tree>
set -euo pipefail
template=$(cd "${1:?usage: seed-node-modules.sh <template-tree>}" && pwd -P)
cd "$(git rev-parse --show-toplevel)"
find "$template" -maxdepth 4 -name node_modules -prune -print | while read -r from; do
  rel=${from#"$template"/}
  if [ ! -e "$rel" ]; then
    mkdir -p "$(dirname "$rel")"
    cp -Rc "$from" "$rel"
  fi
done
echo "seeded node_modules from $template"
