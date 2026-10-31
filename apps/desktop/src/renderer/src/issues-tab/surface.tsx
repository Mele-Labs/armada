// The Issues page: a repository's issues from its tracker, beside Reviews, in the Cockpit's grid. The list is the left column
// and the reader the stage. Starting a Job on an issue, and teaching Armada a tracker it cannot read, both open the one
// dispatch sheet, prefilled; the sheet's own act starts the Job.

import { useRef } from "react";
import { DockHandle } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";

import { useColumns } from "../cockpit/deck/columns";
import { useDispatching } from "../dispatch-panel/dispatching";
import { issueHit } from "../numbered";
import { SpendDialog } from "../pr-review/flight";
import { bandsIn, type Band } from "./bands";
import { IssuesKeys } from "./keys";
import { IssuesList } from "./list";
import type { ReportedBug, TicketRead } from "./model";
import { armadaIn, briefOf } from "./reports";
import { IssuesReader } from "./reader";
import { useIssues, useIssuesStore } from "./store";
import "../cockpit/cockpit.css";
import "../cockpit/deck/deck.css";
import "./issues-tab.css";

type Props = {
  picked: RepositorySummary | null;
  repositories: readonly RepositorySummary[];
  onOpenLink: (address: string) => void;
  onOpenSession: (id: string) => void;
};

export function IssuesSurface({ picked, repositories, onOpenLink, onOpenSession }: Props) {
  const store = useIssuesStore()!;
  const snap = useIssues();
  const dispatching = useDispatching();
  const grid = useRef<HTMLDivElement>(null);
  const { columns, resize, reset } = useColumns(grid, "issues");
  const bands = bandsIn(picked, repositories, snap.trackers ?? []);

  // The issue rides as a tag, as a `#` chosen in the request would: its reference and title in the words, and the tag with them.
  const start = (band: Band, ticket: TicketRead) => {
    if (dispatching === undefined) return;
    const { tag } = issueHit({ reference: ticket.reference, title: ticket.title, url: ticket.url });
    dispatching.startDraft(`${tag.title} `, band.root, [tag]);
  };
  // A kept report is the brief of a Job in the armada repository, started as an issue's Start a Job starts it. The tag tells Fleet
  // which report the Job holds; the brief opens with the title, so the tag rides with the words.
  const startReport = (root: string, report: ReportedBug) => {
    if (dispatching === undefined) return;
    dispatching.startDraft(briefOf(report), root, [{ kind: "report", id: report.id, title: report.title }]);
  };
  const teach = (band: Band) => {
    if (dispatching === undefined || band.tracker.state !== "refused") return;
    const named = band.tracker.named === undefined ? "" : `${band.tracker.named.charAt(0).toUpperCase()}${band.tracker.named.slice(1)} `;
    dispatching.startDraft(`Teach Armada to read this repository's ${named}tracker as a tracker mod`, band.root);
  };

  return (
    <div className="armada-cockpit deck issues-surface">
      <div ref={grid} className="deck-grid" data-issues style={columns === undefined ? undefined : ({ "--deck-left": `${columns.left}px`, "--deck-right": `${columns.right}px` } as React.CSSProperties)}>
        <div className="deck-left">
          <IssuesList picked={picked} repositories={repositories} onTeach={teach} />
          {columns === undefined ? null : <DockHandle quiet className="deck-handle" grows="right" label="the left column" width={columns.left} {...columns.range.left} onResize={(width) => resize("left", width)} onReset={() => reset("left")} />}
        </div>
        <div className="deck-centre">
          <div className="deck-stage">
            <IssuesReader bands={bands} armada={armadaIn(repositories)} onOpenLink={onOpenLink} onOpenSession={onOpenSession} onStart={start} onStartReport={startReport} />
          </div>
        </div>
      </div>
      <SpendDialog spending={snap.spending} onConfirm={(again) => store.confirmSpend(again)} onCancel={() => store.cancelSpend()} />
      <IssuesKeys />
    </div>
  );
}
