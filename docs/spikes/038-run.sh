#!/bin/zsh
# Spike 38: beads sync over a GitHub remote. Two databases on one Mac stand for two machines.
# Usage: 038-run.sh <seed database folder> <work folder> <git remote url>
set -u
SEED="$1"; WORK="$2"; URL="$3"
export BD_NON_INTERACTIVE=1 BD_NO_REMOTE_ADOPT=1
rm -rf "$WORK"; mkdir -p "$WORK"
now() { python3 -c 'import time; print(time.time())'; }
took() { python3 -c "print(f'{$(now) - $1:.1f}s')"; }
mirror() {
  rm -rf "$WORK/mirror"; git init -q --bare "$WORK/mirror"
  git -C "$WORK/mirror" fetch -q "$URL" refs/dolt/data:refs/dolt/data 2>/dev/null
  echo "remote ref, fetched whole: $(du -sh "$WORK/mirror" | cut -f1)"
}

echo "== seed: machine a is a copy of the 955-issue database"
cp -R "$SEED" "$WORK/a"
(cd "$WORK/a" && bd dolt remote add origin "$URL" 2>&1 | tail -1)
t=$(now); (cd "$WORK/a" && bd dolt push 2>&1 | tail -1); echo "first push: $(took $t)"
mirror

echo "== machine b clones"
mkdir "$WORK/b" && git -C "$WORK/b" init -q
t=$(now); (cd "$WORK/b" && bd init --remote "$URL" --prefix armada --non-interactive --stealth 2>&1 | tail -1); echo "clone: $(took $t)"
git -C "$WORK/b" config beads.role maintainer; git -C "$WORK/a" config beads.role maintainer
echo "b holds: $(cd "$WORK/b" && bd count 2>/dev/null | tail -1)"

echo "== a writes five issues; does a write push by itself?"
before=$(git ls-remote "$URL" refs/dolt/data | cut -c1-12)
for n in 1 2 3 4 5; do
  t=$(now); id=$(cd "$WORK/a" && bd create "spike write $n" --silent 2>/dev/null); echo "create $n ($id): $(took $t)"
done
after=$(git ls-remote "$URL" refs/dolt/data | cut -c1-12)
echo "remote ref before $before, after $after"
echo "b sees spike write 5 before syncing: $(cd "$WORK/b" && bd query 'title="spike write 5"' --json 2>/dev/null | jq length)"
t=$(now); (cd "$WORK/a" && bd sync 2>&1 | tail -1); echo "a sync (push five): $(took $t)"
t=$(now); (cd "$WORK/b" && bd sync 2>&1 | tail -1); echo "b sync (pull five): $(took $t)"
echo "b sees spike write 5 after syncing: $(cd "$WORK/b" && bd query 'title="spike write 5"' --json 2>/dev/null | jq length)"
mirror

echo "== two writers at once: a and b each create, then both sync at once"
(cd "$WORK/a" && bd create "from a" --silent >/dev/null 2>&1)
(cd "$WORK/b" && bd create "from b" --silent >/dev/null 2>&1)
t=$(now)
(cd "$WORK/a" && bd sync > "$WORK/sync-a.log" 2>&1; echo "a exit $?") &
(cd "$WORK/b" && bd sync > "$WORK/sync-b.log" 2>&1; echo "b exit $?") &
wait; echo "both synced: $(took $t)"
tail -2 "$WORK/sync-a.log" "$WORK/sync-b.log"
(cd "$WORK/a" && bd sync >/dev/null 2>&1); (cd "$WORK/b" && bd sync >/dev/null 2>&1)
for m in a b; do echo "$m holds from-a and from-b: $(cd "$WORK/$m" && bd query 'title="from a" OR title="from b"' --json 2>/dev/null | jq length)"; done

echo "== claim race: a and b claim the same open issue, then both sync at once"
X=$(cd "$WORK/a" && bd query 'status=open AND title="spike write 1"' --json 2>/dev/null | jq -r '.[0].id')
(cd "$WORK/a" && bd update "$X" --claim --actor mac-a 2>&1 | tail -1)
(cd "$WORK/b" && bd update "$X" --claim --actor mac-b 2>&1 | tail -1)
(cd "$WORK/a" && bd sync > "$WORK/claim-a.log" 2>&1; echo "a exit $?") &
(cd "$WORK/b" && bd sync > "$WORK/claim-b.log" 2>&1; echo "b exit $?") &
wait
tail -3 "$WORK/claim-a.log" "$WORK/claim-b.log"
for m in a b; do (cd "$WORK/$m" && bd sync >/dev/null 2>&1; echo "$m says $X is held by: $(bd show "$X" --json 2>/dev/null | jq -r 'if type=="array" then .[0] else . end | .assignee')"); done

echo "== size on disk"
du -sh "$WORK/a/.beads" "$WORK/b/.beads"
mirror
