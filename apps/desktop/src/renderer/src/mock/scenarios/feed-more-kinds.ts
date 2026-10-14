// The Cockpit's timeline with what a Session does between its turns: reads, edits and commands Fleet
// counted into one row each, a subagent that starts and finishes, a waiting item and its resolution, a
// Session started by another, and a pull request that opens, goes red and goes green. Three Sessions
// work through the last fifteen minutes over `fleet-timeline`'s two hours. Mock only: the kinds are
// spelled as Fleet's `fleet/timeline-tool-events` will list them, and a row carries the `items` it opens to.
// The walk `cockpit-feed-more-kinds` plays it with each burst its own row.

import type { TimelineEvent, TimelineSubject } from "@armada/protocol";

import type { ActivityKind } from "../../cockpit/fleet-events";
import type { Scenario } from "../moment";
import { s207FleetTimeline as base } from "./fleet-timeline";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();
const FORGE = "https://git.example/armada/pull/";

type Who = { subject: TimelineSubject; title: string };
const session = (id: string, title: string): Who => ({ subject: { kind: "session", id }, title });
const pull = (number: number, title: string): Who => ({ subject: { kind: "pull_request", number, url: `${FORGE}${number}` }, title });

const s9 = session("s9", "Flaky store test");
const s10 = session("s10", "Migration notes");
const s12 = session("s12", "Theme token audit");
const s13 = session("s13", "Changelog draft");
const PULL = pull(1896, "Wait on the channel, not the sleep");

// Ids above every entry the base numbers.
let count = 2000;
function entry(minutes: number, who: Who, kind: TimelineEvent["kind"] | ActivityKind, tone: TimelineEvent["tone"], says: string, rest: { detail?: string; live?: boolean; items?: readonly string[] } = {}) {
  count += 1;
  return { id: count, at: ago(minutes), kind, subject: who.subject, title: who.title, tone, says, manifest_id: "armada", ...rest };
}

const files = (n: number) => `${n} ${n === 1 ? "file" : "files"}`;
const read = (minutes: number, who: Who, items: readonly string[]) => entry(minutes, who, "session.read", "run", `Read ${files(items.length)}`, { detail: items[0]!, live: true, items });
const edited = (minutes: number, who: Who, items: readonly string[]) => entry(minutes, who, "session.edited", "run", `Edited ${files(items.length)}`, { detail: items[0]!, live: true, items });
const ran = (minutes: number, who: Who, items: readonly string[]) =>
  entry(minutes, who, "session.ran", "run", `Ran ${items.length} ${items.length === 1 ? "command" : "commands"}`, { detail: items[0]!, live: true, items });

const under = (dir: string, names: string) => names.split(" ").map((name) => `${dir}/${name}`);

const MORE = [
  // Flaky store test: reads the channel, edits it, runs the tests, opens a pull request that goes red and then green.
  read(11.2, s9, ["crates/store/src/channel.rs", ...under("crates/store/src", "lib.rs reads.rs writes.rs clock.rs snapshot.rs retry.rs"), ...under("crates/store/tests", "flaky.rs reads.rs writes.rs"), "crates/store/Cargo.toml", "crates/store/tests/snapshots/store.snap", "crates/store/tests/common.rs", "Cargo.lock"]),
  edited(9.8, s9, ["crates/store/src/channel.rs", "crates/store/src/clock.rs", "crates/store/src/retry.rs"]),
  ran(8.4, s9, ["cargo test -p store flaky -- --nocapture", "cargo test -p store flaky -- --test-threads=1", "cargo check -p store", "git diff --stat", "cargo test -p store flaky"]),
  edited(7.1, s9, ["crates/store/tests/flaky.rs", "crates/store/src/channel.rs"]),
  ran(6, s9, ["cargo test -p store", "cargo clippy -p store"]),
  entry(5.2, PULL, "pr.opened", "idle", "Opened", { detail: "fix/flaky-store" }),
  entry(4.6, PULL, "pr.checks", "run", "Checks running", { live: true }),
  entry(3.1, PULL, "pr.checks", "bad", "Check store failed", { detail: "store: 1 failed" }),
  read(2.6, s9, ["crates/store/tests/flaky.rs", "crates/store/tests/snapshots/store.snap", "crates/store/src/clock.rs"]),
  edited(1.9, s9, ["crates/store/tests/flaky.rs"]),
  entry(1.4, PULL, "pr.checks", "run", "Checks running", { live: true }),
  entry(0.5, PULL, "pr.checks", "ok", "Checks passed"),
  entry(0.3, s9, "session.waiting", "ask", "Waiting: approve #1896", { detail: "Pull request" }),

  // Migration notes: reads, starts a subagent and a Session, runs git while the subagent works.
  read(6.6, s10, ["migrations/0042_add_index.sql", ...under("migrations", "0043_split_runs.sql 0044_backfill.sql 0045_rename_ledger.sql 0046_drop_legacy.sql 0047_add_slots.sql"), "docs/migrations.md", "crates/store/src/migrate.rs"]),
  entry(6, s13, "session.spawned", "run", "Started by Migration notes", { live: true }),
  entry(5.6, s10, "session.subagent", "run", "Subagent started", { detail: "Check the order of migrations 0042 to 0047", live: true }),
  edited(5, s10, ["docs/migrations.md"]),
  ran(4.3, s10, ["git log --oneline 0042..HEAD -- migrations", "git diff 0042..HEAD --stat -- migrations", 'rg -n "CREATE INDEX" migrations']),
  entry(2.2, s10, "session.subagent", "ok", "Subagent finished", { detail: "Check the order of migrations 0042 to 0047", live: true }),
  read(1.7, s10, ["migrations/0046_drop_legacy.sql", ...under("migrations", "0047_add_slots.sql 0045_rename_ledger.sql"), "crates/store/src/migrate.rs"]),

  // Changelog draft, the Session Migration notes started.
  read(5.4, s13, ["CHANGELOG.md", "docs/migrations.md", "crates/store/Cargo.toml", "package.json", "Cargo.toml", "README.md"]),
  edited(3.9, s13, ["CHANGELOG.md"]),
  ran(0.9, s13, ["git diff --stat main"]),
  entry(0.7, s13, "session.subagent", "run", "Subagent started", { detail: "Check the headings against the style guide", live: true }),

  // Theme token audit: reads a great many files, waits on a decision, and is answered.
  read(13.4, s12, [
    "packages/tokens/src/colors.css",
    ...under("packages/tokens/src", "spacing.css semantic.css light.css base.css motion.css type.css"),
    ...under("packages/components/src/primitives", "Badge/Badge.css Button/Button.css Tabs/Tabs.css Tooltip/Tooltip.css Sheet/Sheet.css Table/Table.css Card/Card.css Dialog/Dialog.css"),
    ...under("apps/desktop/src/renderer/src/cockpit", "timeline.css cockpit.css autopilot.css"),
    "apps/desktop/src/renderer/src/mock/mock.css",
    "packages/screens/src/screens.css",
    "packages/tokens/src/index.css",
    "docs/contracts/design-system.md",
  ]),
  ran(12.1, s12, ['rg -l "status-" packages apps', 'rg -l "fg-subtle" packages apps', 'rg -c "--space-" packages', "git log -3 --format=%s -- packages/tokens"]),
  entry(9.5, s12, "session.waiting", "ask", "Waiting: delete the unused tokens?", { detail: "Decision" }),
  entry(3.2, s12, "session.resolved", "ok", "Resolved: delete the unused tokens?", { detail: "Decision", live: true }),
  edited(2.6, s12, [...under("packages/tokens/src", "colors.css light.css semantic.css spacing.css"), "packages/tokens/README.md", "docs/contracts/design-system.md"]),
  ran(1.3, s12, ["pnpm -C packages/tokens test", "pnpm -C packages/tokens typecheck"]),
];

export const s243FeedMoreKinds: Scenario = {
  ...base,
  name: "feed-more-kinds",
  says: "The Cockpit's timeline with reads, edits and commands, a subagent, a waiting item and its resolution, and a Session started by another",
  // Cast: the wire's kind list does not hold the seven activity kinds until Fleet's branch lands.
  draft: { ...base.draft, events: [...(base.draft?.events ?? []), ...(MORE as unknown as TimelineEvent[])] },
};
