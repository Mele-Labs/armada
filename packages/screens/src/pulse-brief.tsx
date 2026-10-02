// A kept brief, read once into Pulse's log panel: what a Judge or a gaming
// check was asked.
//
// **Fetched, not streamed.** Fleet writes a brief whole and closes it before
// the call goes out, so there is nothing to follow; the panel asks for it once
// when it opens (`get_brief`, protocol 21.11) and draws it the way a Check's
// recorded output is drawn — numbered as the file is, because a Judge's
// citation names lines of the brief it read.

import { useEffect, useState } from "react";

import { ConsoleOutput, type ConsoleRegion, type ConsoleRow } from "@armada/components";
import type { BriefContents, BriefRead } from "@armada/protocol";

import { weighs } from "./outputs";

/** Reading one kept brief, as the screen's caller hands it in. `ReadCheckOutput`'s shape. */
export type ReadBrief = (jobId: string, name: string) => Promise<BriefRead>;

/** What a brief panel holds: nothing yet, the brief, or why there is none. */
type Held = { state: "reading" } | { state: "got"; brief: BriefContents } | { state: "absent"; note: string };

export function BriefPane({ jobId, path, read }: { jobId: string; path: string; read: ReadBrief }) {
  const [held, setHeld] = useState<Held>({ state: "reading" });
  useEffect(() => {
    let current = true;
    setHeld({ state: "reading" });
    read(jobId, nameOf(path)).then(
      (answer) => current && setHeld(settled(answer)),
      () => current && setHeld({ state: "absent", note: NOT_ANSWERED }),
    );
    return () => {
      current = false;
    };
  }, [jobId, path, read]);
  // Nothing while the one read is out: an empty slot stays empty.
  if (held.state === "reading") return null;
  if (held.state === "absent") return <ConsoleOutput rows={[]} emptyNote={held.note} />;
  return <ConsoleOutput rows={rowsOf(held.brief)} region={regionOf(held.brief)} emptyNote={EMPTY} />;
}

/** The file's own name, which is all `get_brief` takes. Fleet resolves it inside the Job's briefs. */
export function nameOf(path: string): string {
  return path.slice(path.lastIndexOf("/") + 1);
}

function settled(answer: BriefRead): Held {
  if (answer.ok) return { state: "got", brief: answer.brief };
  const refused = !answer.outcome.ok && answer.outcome.why === "refused";
  return { state: "absent", note: refused ? NOT_KEPT : NOT_ANSWERED };
}

/** The lines, numbered as the file numbers them. */
export function rowsOf(brief: BriefContents): ConsoleRow[] {
  return brief.lines.map((text, at) => ({ row: "line" as const, at: brief.from_line + at, text }));
}

/** Where the window sits in the file, and the file's own path and weight. */
export function regionOf(brief: BriefContents): ConsoleRegion {
  const last = brief.from_line + Math.max(brief.lines.length - 1, 0);
  const says = brief.whole
    ? `${brief.total_lines.toLocaleString()} lines`
    : `lines ${brief.from_line.toLocaleString()}–${last.toLocaleString()} of ${brief.total_lines.toLocaleString()}`;
  return { says, path: brief.path, size: weighs(brief.bytes) };
}

/** The 422: the Job is there and kept no brief under that name. */
const NOT_KEPT = "This brief is no longer in the record.";

/** Fleet did not answer. The sentence a Check's output uses. */
const NOT_ANSWERED = "Fleet did not answer";

/** A brief Fleet kept with nothing in it. */
const EMPTY = "This brief is empty.";
