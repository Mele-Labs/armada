// What a Session's background rows say: a shell command the agent ran in the background, or a Monitor it
// set to watch. Apart from `sessions.tsx`, which is at its length.

import { BACKGROUND } from "@armada/components";
import type { LedgerEntry, SessionLedgerCard } from "@armada/components";
import { span } from "@armada/screens/src/duration";
import type { SessionAttachment } from "@armada/screens/src/draft/sessions";

export type Background = Extract<SessionAttachment, { kind: "background" }>;

/** The row's end mark: the pulsing dot while it runs, then how it ended. */
export function backgroundMarkOf(one: Background): NonNullable<LedgerEntry["mark"]> {
  if (one.state === "running") return { glyph: "running", said: "Running" };
  if (one.state === "stopped") return { glyph: "stopped", said: "Stopped" };
  return one.state === "done" ? { glyph: "done", said: `Exit ${one.exit ?? 0}` } : { glyph: "failed", said: one.exit === undefined ? "Failed" : `Exit ${one.exit}` };
}

/** The row, minus its card. An ended one is `finished`, so it is drawn under All only. */
export function backgroundEntryOf(one: Background, onOpen: () => void): Omit<LedgerEntry, "card"> {
  const mark = backgroundMarkOf(one);
  return {
    key: one.id,
    kind: "background",
    background: one.form,
    name: `${BACKGROUND[one.form].said} ${one.task}, ${mark.said.toLowerCase()}`,
    text: one.task,
    mark,
    ...(one.state === "running" ? {} : { finished: true }),
    onOpen,
  };
}

/** The card: the command in full, when it began, and how long it has run or how it ended. */
export function backgroundCardOf(one: Background, now: number = Date.now()): SessionLedgerCard {
  const mark = backgroundMarkOf(one);
  return {
    title: one.task,
    facts: [
      { label: "Command", value: one.command, mono: true },
      { label: "Started", value: { at: one.started } },
      one.ended === undefined ? { label: "Running for", value: span(one.started, now) ?? "" } : { label: "Ended", value: { at: one.ended } },
      ...(one.state === "running" ? [] : [{ label: "Exit", value: one.state === "stopped" ? "Stopped" : one.exit === undefined ? (one.state === "done" ? "0" : "Failed") : String(one.exit), tone: mark.glyph }]),
    ],
  };
}
