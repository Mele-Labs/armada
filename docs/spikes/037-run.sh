#!/bin/zsh
# Two machines, one remote: the same red claimed on each, then a dropped run fact. What does Dolt's merge do?
set -u
DOLT="$1"
WORK="$2"
export DOLT_ROOT_PATH="$WORK/root"
rm -rf "$WORK/sim"; mkdir -p "$WORK/sim" "$DOLT_ROOT_PATH"
cd "$WORK/sim"
$DOLT config --global --add user.name spike >/dev/null
$DOLT config --global --add user.email spike@example.invalid >/dev/null
mkdir remote seed && cd seed && $DOLT init >/dev/null
$DOLT sql -q "CREATE TABLE claims (repo VARCHAR(128), red VARCHAR(128), machine VARCHAR(64), since BIGINT, PRIMARY KEY (repo, red));
CREATE TABLE runs (repo VARCHAR(128), workspace VARCHAR(200), kind VARCHAR(16), command VARCHAR(400), PRIMARY KEY (repo, workspace, kind, command));
INSERT INTO runs VALUES ('r1', '.', 'tests', 'cargo test'), ('r1', '.', 'lint', 'cargo clippy');"
$DOLT add -A && $DOLT commit -m base >/dev/null
$DOLT remote add origin "file://$WORK/sim/remote" && $DOLT push origin main 2>&1 | tail -1
cd .. && $DOLT clone "file://$WORK/sim/remote" a >/dev/null 2>&1 && $DOLT clone "file://$WORK/sim/remote" b >/dev/null 2>&1

echo "== claim: each machine claims red h1 offline"
(cd a && $DOLT sql -q "INSERT INTO claims VALUES ('r1','h1','mac-a',100)" && $DOLT commit -am "a claims")
(cd b && $DOLT sql -q "INSERT INTO claims VALUES ('r1','h1','mac-b',101)" && $DOLT commit -am "b claims")
echo "-- a pushes:"; (cd a && $DOLT push origin main 2>&1 | tail -2)
echo "-- b pushes:"; (cd b && $DOLT push origin main 2>&1 | tail -2)
echo "-- b pulls:"; (cd b && $DOLT pull origin main 2>&1 | tail -3)
echo "-- b conflicts:"; (cd b && $DOLT sql -r csv -q "SELECT our_machine, their_machine FROM dolt_conflicts_claims")
(cd b && $DOLT merge --abort >/dev/null 2>&1; $DOLT reset --hard origin/main >/dev/null 2>&1)

echo "== runs: a drops 'cargo clippy', b adds 'pnpm dev'"
(cd a && $DOLT sql -q "DELETE FROM runs WHERE command='cargo clippy'" && $DOLT commit -am "a drops" && $DOLT push origin main 2>&1 | tail -1)
(cd b && $DOLT sql -q "INSERT INTO runs VALUES ('r1','.','dev_server','pnpm dev')" && $DOLT commit -am "b adds" && $DOLT pull origin main 2>&1 | tail -2)
echo "-- b after merge:"; (cd b && $DOLT sql -r csv -q "SELECT command FROM runs ORDER BY command")

echo "== history: who changed claims, cell by cell"
(cd a && $DOLT pull origin main >/dev/null 2>&1; $DOLT sql -r csv -q "SELECT to_machine, from_machine, diff_type, to_commit FROM dolt_diff_claims" | head -5)
echo "== size on disk"
du -sh "$WORK/sim/a/.dolt" "$WORK/sim/remote"
