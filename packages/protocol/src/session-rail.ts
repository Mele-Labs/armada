// A Session's workflow rail and its judges. `crates/ipc/src/session_rail.rs`, `docs/concepts/session.md`
// (*The rail*, *The judges*). Since 16 Oct 2026.
//
// The header rules in `protocol.ts` hold here. The rail is soft and the judge is advisory: Fleet reads the
// step a Session is on from what it observed and nudges the agent about a step skipped, and a judge's
// concern reaches the agent only if the person sends it. The record's own members (`workflow_step`,
// `judging`, `judge`, `settled`) are on `SessionRecord`, and a concern is a `WaitingItem` of source `judge`.

/** What produced a concern: the Judge tile, or the boundary after the step at `index` (from zero, as `workflow_step`). */
export type ConcernFrom = { kind: "tile" } | { kind: "step"; index: number; label: string };

/** How the person settled a concern: `sent` to the agent as his message, or `dismissed` with the agent told nothing. */
export type ConcernHow = "sent" | "dismissed";

/** What a `judge` waiting item carries beside its text. `settled` is absent while the concern waits. */
export type JudgeConcern = {
  from: ConcernFrom;
  settled?: { how: ConcernHow; at: string };
  /** The refusal in three short sentences, which `text` joins: what should be seen, what was seen instead, what the difference does. Absent on a concern kept before they were. */
  expected?: string;
  produced?: string;
  consequence?: string;
};

/** A place a concern points at: a file and, where the quotation lands on a line of it, the line as the change leaves the file. */
export type WaitingCite = { path: string; line?: number };

/** The outcome of the last reading. `concerns` 0 is how Bridge says "No concerns". */
export type JudgeReading = { at: string; concerns: number };

/** `POST /sessions/judge`: run a judge now. Answers the record at once with `judging` true; `judge` lands when it returns. */
export type JudgeSession = { session_id: string };

/** `POST /sessions/judge/send`: the concern (`judge:<n>`) goes to the agent as the person's message, and is settled. */
export type SendConcern = { session_id: string; item_id: string };

/** `POST /sessions/judge/dismiss`: the concern is settled and the agent is told nothing. */
export type DismissConcern = { session_id: string; item_id: string };
