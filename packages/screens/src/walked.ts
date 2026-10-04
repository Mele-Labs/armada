// A Job's work served for review, opened in Bridge's own window when the Job
// is opened — `docs/concepts/manifest.md`, *A server to walk the work on*.
//
// **Only what Fleet started for review**, `for_review`: a server a person or a
// Drone started is theirs to open. Read off the live list, so one that comes
// up while the Job is open opens then, and once per server while it stays open.

import { useEffect, useRef } from "react";
import type { ServerList } from "@armada/protocol";

import { whyNotWalked, type OpenCaptureWindow } from "./capturing";

export function useWalkedOnOpen(
  jobId: string,
  servers: ServerList,
  open: OpenCaptureWindow | undefined,
  onSaid: (sentence: string) => void,
): void {
  const opened = useRef(new Set<string>());
  useEffect(() => {
    if (open === undefined) return;
    for (const server of servers.servers) {
      const link = server.links[0];
      if (server.job_id !== jobId || server.for_review !== true || server.phase !== "serving") continue;
      if (link === undefined || opened.current.has(server.id)) continue;
      opened.current.add(server.id);
      void open(server.id, link.url).then((answer) => {
        const because = whyNotWalked(answer);
        if (because !== null) onSaid(because);
      });
    }
  }, [jobId, servers, open, onSaid]);
}
