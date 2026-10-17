// A Session thread with a mix of what one holds: his message, the agent's prose in paragraphs, tool calls, a check
// run, a sketch card, a pull request to approve and a question with options. The walk `thread-look` plays it, in
// the new look (`?look=new`).

import { featureRunning } from "@armada/jobs/fake";
import { repository } from "@armada/screens/src/fixtures/build/base";
import type { Session } from "@armada/screens/src/draft/sessions";

import { asRow, holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";
import { READS, READS_TITLE } from "../sessions/sketches";

const OWNER = repository().manifest!.id;
const { reclaimed_at: _cleared, ...job } = { ...asRow(featureRunning(), 91, "look-job", "Cache the manifest read").job, owner_manifest_id: OWNER };
const running = { ...asRow(featureRunning(), 91, "look-job", "Cache the manifest read"), job };

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const SESSION: Session = {
  id: "s21",
  title: "Land the speed fixes",
  turn: { state: "idle" },
  lastTurn: "14:20",
  lastTurnAt: ago(2),
  rows: [
    { id: "t1", at: "14:10:00", kind: "message", from: { kind: "you" }, text: "The manifest read is slow on cold start. Find out why and fix it, then get it through the merge line." },
    { id: "t2", at: "14:10:20", kind: "tool", text: "Read crates/manifest/src/read.rs" },
    { id: "t3", at: "14:10:21", kind: "tool", text: "Grep read_manifest crates/" },
    { id: "t4", at: "14:10:24", kind: "tool", text: "Read crates/manifest/src/cache.rs" },
    {
      id: "t5",
      at: "14:11:02",
      kind: "message",
      from: { kind: "agent" },
      text: "The read parses the whole manifest on every call, and `read_manifest` is called once per Check. On a cold start that is forty parses of the same file.\n\nThe fix is to read once and hand the parsed value around. I have kept the cache keyed on the file's modified time, so an edit still shows up on the next read.\n\n- **Cache** the parsed manifest by path and mtime\n- **Drop** the per-Check reads in `gate.rs`\n- **Keep** the error text as it was",
    },
    { id: "t6", at: "14:11:30", kind: "tool", text: "Edit crates/manifest/src/cache.rs" },
    { id: "t7", at: "14:11:41", kind: "tool", text: "Edit crates/gate/src/gate.rs" },
    { id: "t8", at: "14:12:10", kind: "check", name: "manifest", run: 4410, state: "passed" },
    { id: "t9", at: "14:13:00", kind: "message", from: { kind: "agent" }, text: "I drew how the read flows now." },
    { id: "t9b", at: "14:13:01", kind: "tool", text: `show_sketch ${JSON.stringify({ title: READS_TITLE, scene: READS })}` },
    { id: "t10", at: "14:20:00", kind: "message", from: { kind: "agent" }, text: "Checks pass and the pull request is open. It needs your approval before it can join the merge line." },
  ],
  attachments: [
    { kind: "pull_request", number: 2118, title: "Cache the manifest read", branch: "speed/manifest-cache", address: "https://git.example/armada/pull/2118", checks: { state: "passed" }, state: "open", auto: false },
    { kind: "sketch", id: `sketch:${READS_TITLE}`, title: READS_TITLE, by: "agent", drawing: READS },
  ],
  waitingFor: [
    { id: "e-pull", text: "Approve #2118", since: ago(7), source: "agent", act: { kind: "approve_pr", target: "2118" } },
    {
      id: "e-ask",
      text: "Which clock should the store tests use?",
      since: ago(5),
      source: "ask_card",
      act: { kind: "answer", target: "q-21" },
      options: [{ label: "A fake the test sets" }, { label: "The real one, stopped" }],
    },
  ],
};

function build(): Scenario {
  const base = holding("thread-look", "A Session thread with prose, tool calls, a check, a sketch, a pull request and a question", [running]);
  const none = { id: "x", number: 0, title: "", branch: "", slot: 0 };
  return {
    ...base,
    state: { ...base.state, repository: repository().root, jobs: [running.job] },
    draft: { sessions: (board) => sessionsStore([none, none], none, [], board, [], [SESSION]) },
  };
}

export const s211ThreadLook: Scenario = build();
