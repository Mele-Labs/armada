// The bodies of the tabs that stand on their own. A tab that needs the Job's thread or its story is drawn by `SessionWorkspace`, which holds both.

import { useEffect, useRef } from "react";
import { Button, UnifiedDiff, PlanRevisionView, useUnchanged, type LedgerEntry, type PlanRevision, ShellPane } from "@armada/components";
import type { Session } from "@armada/screens/src/draft/sessions";
import { drawnOf } from "@armada/screens/src/drawn-patch";

import { JobBrowserPane } from "../job-browser/JobBrowserPane";
import { useJobBrowser } from "../job-browser/store";
import { shellsOf, useJobShell } from "../job-shell";
import { useDeckShell } from "../job-shell-dock";
import { useSessionsDraft } from "../sessions-draft";
import { openRequestsInPanel } from "../shared-panel/store";
import { historyOf } from "../shared-panel/requests-model";
import { MOCK_PATCH } from "./mock-diff";
import { jobOpens, useJobPanes } from "./store";
import { hasKind } from "./tree";

export function BrowserBody({ jobId }: { jobId: string }) {
  const browser = useJobBrowser(jobId);
  return browser === undefined ? null : <JobBrowserPane sessionId={jobId} browser={browser} placed="pane" />;
}

/** The Job's shell. The Cockpit's Terminal tile shows the same shell, which is why it asks for it shown while a Terminal tab is open. */
export function TerminalBody({ jobId }: { jobId: string }) {
  const draft = useSessionsDraft();
  const served = draft?.terminalRuns !== undefined;
  const pane = useDeckShell(jobId, served);
  const shells = served ? shellsOf(window.armada) : null;
  const { shown } = useJobShell(shells, "page", jobId);
  useEffect(() => {
    if (shells !== null && !shown) shells.want("page", jobId);
    // Once, as the tab opens: a shell put away from its own head stays away.
  }, []);
  return pane === null ? null : <ShellPane {...pane} />;
}

const cutSaid = (drawn: number, total: number) => `The first ${drawn.toLocaleString()} of ${total.toLocaleString()} lines.`;

export function DiffBody() {
  const drawn = drawnOf(MOCK_PATCH, cutSaid);
  return (
    <div className="job-pane__scroll">
      <UnifiedDiff files={drawn.files} />
    </div>
  );
}

export function PlanBody({ session }: { session: Session }) {
  const groups = session.work?.plan?.groups ?? [];
  const revision: PlanRevision = { groups: groups.map((group) => ({ id: group.id, agent: group.agent, title: group.title, tasks: group.tasks.map((task) => ({ id: task.id, text: task.text, state: task.state })), gone: [] })) };
  const unchanged = useUnchanged(revision);
  return (
    <div className="job-pane__scroll">
      <PlanRevisionView revision={revision} unchanged={unchanged} />
    </div>
  );
}

/** The files the Job has made or shown, from its ledger. */
export function FilesBody({ entries }: { entries: readonly LedgerEntry[] }) {
  const files = entries.filter((one) => one.kind === "artifact");
  return (
    <ul className="job-pane__list job-pane__scroll" aria-label="Files">
      {files.map((one) => (
        <li key={one.key} className="job-pane__item">
          {one.name}
        </li>
      ))}
    </ul>
  );
}

/** The requests the Job sent, newest first, and the Requests panel to send another. */
export function RequestBody({ session }: { session: Session }) {
  const sent = historyOf(session);
  return (
    <div className="job-pane__scroll">
      <ul className="job-pane__list" aria-label="Requests">
        {sent.map((one) => (
          <li key={one.id} className="job-pane__item">
            <span className="job-pane__mono">{one.method}</span> {one.url}
          </li>
        ))}
      </ul>
      <Button variant="secondary" size="sm" onClick={() => openRequestsInPanel(session.id)}>
        Open Requests
      </Button>
    </div>
  );
}

/**
 * The Cockpit's Terminal tile and a Terminal tab are one shell. A shell shown with no tab for it opens one, as a tab the Job opens itself does; the last tab closed puts the shell away.
 * Draws nothing.
 */
export function TerminalSync({ jobId }: { jobId: string }) {
  const draft = useSessionsDraft();
  const shells = draft?.terminalRuns !== undefined ? shellsOf(window.armada) : null;
  const { shown } = useJobShell(shells, "page", jobId);
  const has = hasKind(useJobPanes(jobId).arrangement, "terminal");
  const was = useRef(has);
  useEffect(() => {
    const closed = was.current && !has;
    was.current = has;
    if (closed) return void shells?.show("page", jobId, false);
    if (shown && !has) jobOpens(jobId, "terminal");
  }, [shown, has]);
  return null;
}
