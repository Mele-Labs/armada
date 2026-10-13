// Jobs, on the rail: every Job there is, under the Sessions page's own frame (a header band over a
// sunken glass, Active, Done and All as tabs), and a press opens the Job where it opens anywhere. The
// rows and their sections are Overview's own `OverviewLists`, the Board that went; j and k, Enter,
// x and n are its keys. A tab narrows the sections, and a tab with nothing in it draws nothing.

import { useMemo, useState } from "react";
import { Button, Tabs } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";
import type { BoardSection, PauseAct } from "@armada/screens";
import { OverviewLists, overviewListsOf } from "@armada/overview";
import { Boundary } from "@armada/shell";

import type { BridgeState } from "../../shared/bridge";
import "./jobs-surface.css";

const VIEWS = [
  { id: "active", label: "Active", sections: ["needs-you", "running", "queued", "other"] },
  { id: "done", label: "Done", sections: ["recently-ended", "done"] },
  { id: "all", label: "All", sections: ["needs-you", "running", "queued", "other", "recently-ended", "done"] },
] as const satisfies readonly { id: string; label: string; sections: readonly BoardSection[] }[];

type ViewId = (typeof VIEWS)[number]["id"];

export function JobsSurface({
  state,
  now,
  live,
  repositories,
  disconnected,
  selected,
  onOpen,
  onKill,
  onRedispatch,
  onClear,
  onPausing,
  onCompose,
  onCopied,
  onCursor,
}: {
  state: BridgeState;
  now: number;
  live: boolean;
  repositories: readonly RepositorySummary[];
  disconnected: string | null;
  /** The Job whose detail is open, where one is. */
  selected: string | null;
  onOpen: (jobId: string) => void;
  onKill: (jobId: string) => void;
  onRedispatch: (jobId: string) => void;
  onClear: (jobId: string) => void;
  onPausing: (act: PauseAct, jobId: string) => void;
  onCompose: () => void;
  onCopied: (value: string) => void;
  onCursor: (jobId: string | null) => void;
}) {
  const [view, setView] = useState<ViewId>("active");
  const picked = repositories.find((one) => one.root === state.repository) ?? null;
  const shown = useMemo(() => {
    const wanted = new Set<string>(VIEWS.find((one) => one.id === view)!.sections);
    const ids = new Set(overviewListsOf(state.jobs, picked).sections.filter((one) => wanted.has(one.id)).flatMap((one) => one.jobs.map((job) => job.id)));
    return state.jobs.filter((job) => ids.has(job.id));
  }, [state.jobs, picked, view]);
  // Fleet unreachable, or nothing served: the Board's own fault states draw, whatever the tab.
  const fault = disconnected !== null || state.connection.state === "starting" || repositories.length === 0;
  return (
    <Boundary region="Jobs" bridge={state.bridge} onCopied={onCopied}>
      <div className="armada-screen__overview">
        <section className="armada-jobs-page" aria-label="Jobs">
          <div className="armada-jobs-page__head">
            <h2 className="armada-jobs-page__title">Jobs</h2>
            <Button size="sm" variant="secondary" onClick={onCompose}>
              New Job
            </Button>
          </div>
          <div className="armada-jobs-page__controls">
            <Tabs items={VIEWS.map(({ id, label }) => ({ id, label }))} value={view} onChange={(id) => setView(id as ViewId)} />
          </div>
          <div className="armada-jobs-page__glass">
            {shown.length === 0 && !fault && state.jobs.length > 0 ? null : (
              <OverviewLists
                jobs={fault ? state.jobs : shown}
                stale={!live}
                now={now}
                workflows={state.holds.workflows}
                repositories={repositories}
                picked={state.repository}
                disconnected={disconnected}
                starting={state.connection.state === "starting"}
                selected={selected}
                onOpen={onOpen}
                onKill={onKill}
                onRedispatch={onRedispatch}
                onClear={onClear}
                onPausing={onPausing}
                onCompose={onCompose}
                onCopied={onCopied}
                onCursor={onCursor}
              />
            )}
          </div>
        </section>
      </div>
    </Boundary>
  );
}
