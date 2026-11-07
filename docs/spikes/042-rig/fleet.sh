#!/usr/bin/env bash
#
# A throwaway Fleet whose Jobs run a REAL Claude Code agent (sonnet), hosted the
# way Bridge hosts one. docs/spikes/042-workflow-mechanisms-rig.md.
#
#   fleet.sh <scratch-dir>      sets everything up, then ends by exec-ing `armada serve`
#
# Start it with run_in_background and leave it up; run.sh talks to it.
#
# **Why the scratch HOME is the isolation.** Fleet's store, its runtime file
# (`fleet.json`) and the mod's copy all live under $HOME/Library/Application
# Support/Armada, and the mod finds Fleet by reading that one file. So an agent
# whose HOME is the scratch home reports to this Fleet and cannot see the
# owner's. The two things the scratch home needs from the real one are the login
# keychain (a symlink, so `claude` is signed in) and nothing else.
#
# Environment (all optional):
#   RIG_TREE     the armada tree whose plugins/ is the mod under test (default: this checkout)
#   RIG_BASE     the commit the clone starts from (default 3fee6c4c46)
#   RIG_ARMADA   the armada binary to serve with (default: newest of the owner's kept builds, copied)

set -euo pipefail

dir=${1:?usage: fleet.sh <scratch-dir>}
rig=$(cd "$(dirname "$0")" && pwd -P)
tree=${RIG_TREE:-$(git -C "$rig" rev-parse --show-toplevel)}
tree=$(cd "$tree" && pwd -P)
base=${RIG_BASE:-3fee6c4c46}
real_home=$HOME

mkdir -p "$dir"
dir=$(cd "$dir" && pwd -P)
case "$dir/" in
  "$tree"/* | "$real_home/Development/armada"/*)
    echo "fleet.sh: $dir is inside the repository; use a scratch directory outside it" >&2
    exit 1
    ;;
esac

home="$dir/home/user"
machine="$home/Library/Application Support/Armada"
runtime="$machine/fleet.json"

if [ -f "$runtime" ]; then
  pid=$(python3 -I -c 'import json,sys; print(json.load(open(sys.argv[1])).get("pid", ""))' "$runtime")
  if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
    echo "fleet.sh: already running as pid $pid, $runtime"
    exit 0
  fi
fi

mkdir -p "$machine" "$dir/bin" "$dir/shims" "$home/.claude" "$home/Library"

# --- sign-in: the login keychain, nothing else of the real home --------------
ln -sfn "$real_home/Library/Keychains" "$home/Library/Keychains"

# --- git identity for the agent; no credentials, no helpers ------------------
cat >"$home/.gitconfig" <<GIT
[user]
	name = Rig Agent
	email = rig-agent@example.invalid
[init]
	defaultBranch = main
GIT

# --- the repository clone, its local bare origin, and a Manifest of its own --
if [ ! -d "$dir/repo" ]; then
  git clone --local --no-checkout --quiet "$tree" "$dir/repo"
  git -C "$dir/repo" checkout --quiet -b main "$base"
  # No remote of the owner's survives: origin becomes a bare repository in the scratch.
  git -C "$dir/repo" remote remove origin
  git -C "$dir/repo" config user.name "Rig Agent"
  git -C "$dir/repo" config user.email "rig-agent@example.invalid"
  # One Manifest, no workspace ones, no tracker, no setup that builds anything.
  find "$dir/repo" -name armada.yml -not -path '*/node_modules/*' -not -path "$dir/repo/.git/*" -delete
  rm -f "$dir/repo/.mcp.json"
  cat >"$dir/repo/armada.yml" <<YML
version: 1
id: armada
base: main
connectors: false
checks:
  scenario_test:
    run: cd apps/desktop && node_modules/.bin/vitest run src/renderer/src/mock/scenario.test.ts
commands:
  bootstrap:
    run: bash $rig/seed-node-modules.sh $tree
setup:
  requires:
    - bootstrap
YML
  git -C "$dir/repo" add -A
  git -C "$dir/repo" commit --quiet -m "rig: one small Manifest, no .mcp.json, no tracker"
  git init --bare --quiet "$dir/origin.git"
  git -C "$dir/repo" remote add origin "$dir/origin.git"
  git -C "$dir/repo" push --quiet origin main
  git -C "$dir/repo" branch --set-upstream-to=origin/main main >/dev/null
  # The workflows Fleet reads from a repository's .armada/
  if [ -d "$tree/.armada/workflows" ]; then
    mkdir -p "$dir/repo/.armada"
    cp -R "$tree/.armada/workflows" "$dir/repo/.armada/"
  fi
fi

# --- the mod under test, as Fleet's own copy (what a hosted Job loads) -------
HOME="$home" "$tree/scripts/sync-mod" "$tree"

# --- the agent: the real claude; permission and trust held in the scratch home
claude_bin=$(command -v claude)
cat >"$dir/bin/agent" <<SH
#!/bin/sh
# ARMADA_AGENT_BINARY: Fleet runs this as the hosted Job's agent.
exec "$claude_bin" "\$@"
SH
chmod +x "$dir/bin/agent"

cat >"$home/.claude/settings.json" <<'JSON'
{
  "permissions": {
    "allow": [
      "Read", "Glob", "Grep", "Edit", "Write", "TodoWrite", "TaskCreate", "TaskUpdate", "TaskList", "TaskGet",
      "Bash(git *)", "Bash(ls*)", "Bash(cat *)", "Bash(rg *)", "Bash(grep *)", "Bash(find *)",
      "Bash(head *)", "Bash(tail *)", "Bash(wc *)", "Bash(sed -n*)", "Bash(diff *)", "Bash(pwd)",
      "Bash(cd *)", "Bash(node_modules/.bin/vitest *)", "Bash(cd apps/desktop && node_modules/.bin/vitest *)",
      "Bash(armada *)"
    ],
    "deny": [
      "Bash(gh *)", "Bash(bd *)", "Bash(curl *)", "Bash(wget *)", "Bash(ssh *)", "Bash(sudo *)",
      "Bash(cargo *)", "Bash(pnpm install*)", "Bash(pnpm add*)", "Bash(pnpm dlx*)", "Bash(npm *)", "Bash(npx *)",
      "WebFetch", "WebSearch", "mcp__claude_ai_*", "mcp__plugin_productivity_*"
    ]
  }
}
JSON
python3 -I - "$home/.claude.json" "$dir/repo" <<'PY'
import json, sys
path, repo = sys.argv[1:]
json.dump({"hasCompletedOnboarding": True, "projects": {repo: {"hasTrustDialogAccepted": True, "allowedTools": []}}}, open(path, "w"), indent=2)
PY

# --- shims first on PATH: the agent can reach neither GitHub nor beads -------
for tool in gh bd; do
  printf '#!/bin/sh\necho "%s: not available in the workflow-spike rig" >&2\nexit 1\n' "$tool" >"$dir/shims/$tool"
  chmod +x "$dir/shims/$tool"
done

# --- the Fleet binary: a copy, so a restart of the owner's build changes nothing
armada="$dir/bin/armada"
if [ ! -x "$armada" ]; then
  src=${RIG_ARMADA:-$(ls -d "$real_home/Library/Application Support/Armada/builds"/*/armada | sort | tail -1)}
  cp -c "$src" "$armada" 2>/dev/null || cp "$src" "$armada"
  echo "fleet.sh: serving with a copy of $src" >&2
fi

ln -sfn "$armada" "$dir/shims/armada"

cd "$dir/repo"
exec env HOME="$home" PATH="$dir/shims:$PATH" ARMADA_AGENT_BINARY="$dir/bin/agent" "$armada" serve "$dir/repo"
