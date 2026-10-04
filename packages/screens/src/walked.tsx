// A Job's work served for review, opened in Bridge's own window when the Job
// is opened — `docs/concepts/manifest.md`, *A server to walk the work on*.
//
// **Only what Fleet started for review**, `for_review`: a server a person or a
// Drone started is theirs to open. Read off the live list, so one that comes
// up while the Job is open opens then, and once per server while it stays open.
//
// **And it opens again from the Job**, as often as wanted: the owner, 4 Oct
// 2026, closing the window and finding no way back to it — *"I need to be able
// to freely close and reopen it."* The lead carries the press while it serves.

import { useEffect, useRef, type ReactNode } from "react";
import { Button } from "@armada/components";
import type { ServerList, ServerState } from "@armada/protocol";

import { whyNotWalked, type OpenCaptureWindow } from "./capturing";

/** This Job's server up for review, with a link to open, or `undefined`. */
export function walkable(jobId: string, servers: ServerList): ServerState | undefined {
  return servers.servers.find(
    (server) =>
      server.job_id === jobId && server.for_review === true && server.phase === "serving" && server.links.length > 0,
  );
}

/** Open it, saying why where it did not. */
function walkIn(server: ServerState, open: OpenCaptureWindow, onSaid: (sentence: string) => void): void {
  void open(server.id, server.links[0]!.url).then((answer) => {
    const because = whyNotWalked(answer);
    if (because !== null) onSaid(because);
  });
}

/**
 * **Walk in Bridge**, for the lead, while this Job's review server serves —
 * and how many notes from the walk wait to be sent, whether or not it still
 * serves, so a person sees they are on the Job before opening its review.
 */
export function walkAct(
  jobId: string,
  servers: ServerList,
  open: OpenCaptureWindow | undefined,
  onSaid: (sentence: string) => void,
  waiting = 0,
): ReactNode {
  const server = open === undefined ? undefined : walkable(jobId, servers);
  if (server === undefined && waiting === 0) return null;
  return (
    <>
      {waiting === 0 ? null : (
        <span className="text-fg-muted">
          {waiting === 1 ? "1 note from your walk" : `${waiting} notes from your walk`}, sent when you request changes
        </span>
      )}
      {server === undefined || open === undefined ? null : (
        <Button variant="secondary" onClick={() => walkIn(server, open, onSaid)}>
          Walk in Bridge
        </Button>
      )}
    </>
  );
}

export function useWalkedOnOpen(
  jobId: string,
  servers: ServerList,
  open: OpenCaptureWindow | undefined,
  onSaid: (sentence: string) => void,
): void {
  const opened = useRef(new Set<string>());
  useEffect(() => {
    if (open === undefined) return;
    const server = walkable(jobId, servers);
    if (server === undefined || opened.current.has(server.id)) return;
    opened.current.add(server.id);
    walkIn(server, open, onSaid);
  }, [jobId, servers, open, onSaid]);
}
