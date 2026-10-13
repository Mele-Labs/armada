// A subagent's own thread, as the Session's reading sheet draws it. Apart from `sessions.tsx`,
// which is at its length.

import { useEffect, useState } from "react";
import { Prose, SessionThread } from "@armada/components";
import type { SessionThreadRow } from "@armada/components";
import type { SessionAttachment, SessionsDraft, SubagentThread as Subagent } from "@armada/screens/src/draft/sessions";

import { useBackfilled } from "./backfilled";
import { threadRowsFrom } from "./session-thread-rows";

/** How often a running subagent's transcript is read again. */
const SUBAGENT_EVERY_MS = 1500;

/** How many subagent threads are kept past the sheet that read them. */
const SUBAGENTS_KEPT = 20;

/** The last thread read for each subagent, so a sheet opened again draws it while the next read is out. */
const subagentsRead = new Map<string, Subagent>();

const keepSubagent = (key: string, thread: Subagent): void => {
  subagentsRead.delete(key);
  subagentsRead.set(key, thread);
  for (const old of [...subagentsRead.keys()].slice(0, -SUBAGENTS_KEPT)) subagentsRead.delete(old);
};

const NO_ROWS: SessionThreadRow[] = [];

/**
 * A subagent's own thread, drawn as the Session's is. Read again while it runs; the report it ends
 * on is the last message of the thread, and stands alone only where the thread could not be read.
 */
export function SubagentThread({
  sessionId,
  one,
  read,
}: {
  sessionId: string;
  one: Extract<SessionAttachment, { kind: "subagent" }>;
  read: SessionsDraft["subagent"];
}) {
  const key = `${sessionId}/${one.id}`;
  const [thread, setThread] = useState<Subagent | undefined>(() => subagentsRead.get(key));
  const running = one.state === "running";
  useEffect(() => {
    setThread(subagentsRead.get(key));
    if (read === undefined) return;
    let live = true;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const look = async () => {
      const got = await read(sessionId, one.id);
      if (!live) return;
      if (got !== undefined) {
        keepSubagent(key, got);
        setThread(got);
      }
      if (running && got?.finished !== true) timer = setTimeout(() => void look(), SUBAGENT_EVERY_MS);
    };
    void look();
    return () => {
      live = false;
      if (timer !== undefined) clearTimeout(timer);
    };
  }, [read, sessionId, one.id, key, running]);
  const rows = useBackfilled(thread === undefined ? NO_ROWS : threadRowsFrom(thread.rows), key);
  const report = one.report ?? thread?.report;
  if (rows.length === 0) return report === undefined ? null : <Prose text={report} />;
  return (
    <div className="armada-session-reading" data-thread>
      <SessionThread sessionId={`${sessionId}/${one.id}`} rows={rows} onAnswer={() => undefined} onOpenSession={() => undefined} />
    </div>
  );
}
