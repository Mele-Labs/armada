// The Field's drones and paths from what a real Fleet sends: touches as `FileTouch` spells them (`crates/ipc/src/session_plan.rs`), the
// ledger's `subagent` row, and burst entries as `crates/fleet/src/timeline/tools.rs` writes them, absolute and cut at 120 characters.

import type { FileTouch, TimelineEvent } from "@armada/protocol";
import { fieldOf } from "@armada/screens/src/session-work";
import type { Session } from "@armada/screens/src/draft/sessions";
import { ShieldCheck } from "lucide-react";
import { expect, test } from "vitest";

import { burstsOf } from "../cockpit/deck/afterglow";
import { relativeTo } from "../cockpit/deck/areas";
import { droneTip } from "../cockpit/deck/tip";
import { workingDrones } from "./field";
import { flier, mainFileOf, rootedDrone, rootsOf, subagentTitle } from "./field-drones";

const REPO = "/Users/user/Developer/armada";
const SLOT = `${REPO}/.armada/slots/slot-3`;

/** Fleet's `bare(text, 120)`: one line, cut to 119 characters and an ellipsis. */
const bare = (text: string, most = 120): string => (text.length <= most ? text : `${text.slice(0, most - 1).trimEnd()}…`);

const job = (rest: Partial<Session> = {}): Session => ({
  id: "s1",
  title: "Rotate the audit log",
  manifestId: "armada",
  turn: { state: "working" },
  standing: "working",
  rows: [],
  attachments: [],
  ...rest,
});

let id = 0;
const edited = (at: string, items: string[]): TimelineEvent => ({ id: ++id, at, kind: "session.edited", subject: { kind: "session", id: "s1" }, tone: "run", says: `Edited ${items.length} files`, detail: items[0]!, items, live: true });

test("a subagent's card names the Job it works for, then the task the ledger gave it, where Fleet knew no type for it", () => {
  // Fleet keys a hosted subagent's touch by the call that started it and names its type `subagent` when it was not told one.
  const touches: FileTouch[] = [{ by: { agent: "subagent", id: "toolu_01" }, path: "packages/protocol/src/wire.ts", act: "edit", at: "2026-10-22T09:00:00Z" }];
  const session = job({ attachments: [{ kind: "subagent", id: "agent-a1", call: "toolu_01", task: "Check the wire types", state: "running" }] });
  const drones = workingDrones(session, fieldOf(touches).field)!;
  const sub = flier(session, drones.find((one) => one.agent !== undefined)!, REPO, "live");
  const tip = droneTip(sub, undefined, ShieldCheck);
  expect(tip.label).toBe("Rotate the audit log");
  expect(tip.title).toBe("Check the wire types");
  // The Job's own drone is still a Job.
  expect(droneTip(flier(session, { id: "main", path: "", doing: "read" }, REPO, "live"), undefined, ShieldCheck).label).toBe("Job");
});

test("a subagent with no task recorded is named by its type, and never by the bare word Fleet uses for none", () => {
  expect(subagentTitle({ agent: "Explore" })).toBe("Explore");
  expect(subagentTitle({ agent: "subagent" })).toBe("No task recorded");
  expect(subagentTitle({ agent: "subagent", task: "Keep the stamp" })).toBe("Keep the stamp");
});

test("a terminal Job at the repository's root names files under its slot or worktree, and the Field draws them at the repository's path", () => {
  // `SessionFact::Touched` is relative to the session's cwd, so a Job standing at the root names the slot it works in.
  const { field } = fieldOf([{ path: ".armada/slots/slot-3/apps/desktop/src/a.ts", act: "edit", at: "2026-10-22T09:00:00Z" }]);
  const roots = rootsOf(job({ cwd: REPO }), REPO);
  expect(rootedDrone(field[0]!, roots)).toMatchObject({ file: "apps/desktop/src/a.ts", path: "apps/desktop" });
  expect(relativeTo(".claude/worktrees/fix-a/crates/fleet/src/b.rs", roots)).toBe("crates/fleet/src/b.rs");
  // Absolute, under a Job directory inside the repository: the deepest checkout wins.
  expect(relativeTo(`${REPO}/.claude/worktrees/fix-a/crates/fleet/src/b.rs`, rootsOf(job({ cwd: `${REPO}/.claude/worktrees/fix-a` }), REPO))).toBe("crates/fleet/src/b.rs");
  // Somebody else's file stays absolute, which no area claims.
  expect(relativeTo("/etc/hosts", roots)).toBe("/etc/hosts");
});

test("a hosted Job's absolute path that Fleet cut at its line limit glows on the one file it can be, and on none where it is unknown", () => {
  const file = "apps/desktop/src/renderer/src/cockpit/deck/pixi/callouts-over-the-battlefield.test.tsx";
  const cut = bare(`${SLOT}/${file}`);
  expect(cut.endsWith("…")).toBe(true);
  const roots = rootsOf(job({ attachments: [{ kind: "branch", name: "armada/audit", slot: 3 }] }), REPO);
  const events = [edited("2026-10-22T09:00:00Z", [cut, `${SLOT}/docs/a.md`])];
  expect(burstsOf(events, "s1", roots, [file, "docs/a.md"])[0]!.paths).toEqual([file, "docs/a.md"]);
  expect(burstsOf(events, "s1", roots, [])[0]!.paths).toEqual(["docs/a.md"]);
});

test("the Job's drone stands at the newer of its last touch and its last burst, and the file it writes is never one it only read", () => {
  const touch = fieldOf([{ path: "crates/store/src/rotate.rs", act: "edit", at: "2026-10-22T09:05:00Z" }]).field[0]!;
  const bursts = burstsOf(
    [edited("2026-10-22T09:00:00Z", ["crates/store/src/audit.rs"]), { ...edited("2026-10-22T09:06:00Z", ["crates/store/src/lib.rs"]), kind: "session.read" }],
    "s1",
    [REPO],
  );
  expect(mainFileOf(touch, bursts)).toEqual({ at: "crates/store/src/lib.rs", written: "crates/store/src/rotate.rs" });
  expect(mainFileOf(touch, bursts.slice(0, 1))).toEqual({ at: "crates/store/src/rotate.rs", written: "crates/store/src/rotate.rs" });
  expect(mainFileOf(undefined, bursts)).toEqual({ at: "crates/store/src/lib.rs", written: "crates/store/src/audit.rs" });
});
