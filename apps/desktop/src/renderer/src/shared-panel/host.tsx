// The side panel's host: whatever `openInPanel` was last given, drawn over the app in the one panel. **Mounted once per window**
// beside the app, where the window serves the reads (`reads.tsx`). Closing hands focus back to what opened it.

import { useMemo } from "react";
import { ProseRefs } from "@armada/components";
import type { ProseReferences } from "@armada/components";
import { refScanner } from "@armada/helm";

import { openLink } from "../commands";
import { OverTheApp } from "../session-view/OverTheApp";
import { BayPanel, CheckPanel, IssuePanel } from "./panels";
import { FileSide, useOpenFileKey } from "./file";
import { PullRequestSide } from "./pull-request";
import { RequestsPanel } from "./requests";
import { usePanelReads } from "./reads";
import { openInPanel, usePanel } from "./store";
import "./panel.css";

export function SharedPanelHost() {
  const reads = usePanelReads();
  const { open } = usePanel();
  useOpenFileKey();
  // A `#number` in what the panel reads opens in the panel, as it does in a thread.
  const refs = useMemo((): ProseReferences | null => {
    if (reads === undefined) return null;
    return {
      scan: refScanner({ sessions: [], numbered: reads.numbered }),
      open: (ref) => {
        const hit = reads.numbered.find((one) => one.number === ref.id && one.kind === ref.kind);
        if (ref.kind === "issue") openInPanel({ kind: "issue", number: ref.id, ...(hit?.url === undefined ? {} : { address: hit.url }) });
        else if (ref.kind === "pull_request") openInPanel({ kind: "pr", number: Number(ref.id), address: hit?.url ?? "" });
      },
    };
  }, [reads]);
  if (reads === undefined) return null;
  const onOpenLink = (address: string) => void openLink(address);
  return (
    <ProseRefs.Provider value={refs}>
    <OverTheApp open={open !== undefined}>
      {open === undefined ? null : open.kind === "pr" ? (
        <PullRequestSide key={`pr:${open.number}`} item={open} />
      ) : open.kind === "issue" ? (
        <IssuePanel key={`issue:${open.number}`} item={open} onOpenLink={onOpenLink} />
      ) : open.kind === "file" ? (
        <FileSide key={`file:${open.path}`} item={open} />
      ) : open.kind === "bay" ? (
        <BayPanel key={`bay:${open.manifestId}/${open.slot}`} item={open} />
      ) : open.kind === "requests" ? (
        <RequestsPanel key={`requests:${open.sessionId ?? ""}:${open.from ?? ""}`} item={open} />
      ) : (
        <CheckPanel key={`check:${open.id}`} item={open} />
      )}
    </OverTheApp>
    </ProseRefs.Provider>
  );
}
