// The reader: one issue whole. Its title and reference, the way out to the tracker, its state, who reported it and who has it,
// **every field the tracker returned** as names and values (Bridge draws them alike, whichever tracker sent them), the
// description, the pull requests that name it, and the Jobs working it or the one press that starts one.

import { Button, Kbd, Prose, PullRequestChip, RepositoryChip, Stamp, Tooltip } from "@armada/components";

import { PullRequestMenuOn } from "../pull-request-menu";
import { IssueState, Open } from "../shared-panel/panels";
import type { Band } from "./bands";
import { ISSUES_KEYS } from "./key-table";
import { IssueJobCard, toneOfHolders, useHolders } from "./marks";
import type { ReportedBug, TicketRead } from "./model";
import { useIssues, useIssuesStore } from "./store";

type Hosts = {
  bands: readonly Band[];
  onOpenLink: (address: string) => void;
  onOpenSession: (id: string) => void;
  onStart: (band: Band, ticket: TicketRead) => void;
  /** The armada repository, where a kept report starts a Job. Absent where none is served. */
  armada: { root: string; label: string } | undefined;
  onStartReport: (root: string, report: ReportedBug) => void;
};

/** With nothing open the pane is empty: an empty slot stays empty. */
export function IssuesReader(hosts: Hosts) {
  const store = useIssuesStore()!;
  const { open, openReport, reports } = useIssues();
  const report = reports.find((one) => one.id === openReport);
  if (report !== undefined) return <ReportReader key={report.id} report={report} {...hosts} />;
  const band = hosts.bands.find((one) => one.manifestId === open?.manifestId);
  const read = open === undefined ? undefined : store.readOf(open.manifestId, open.reference);
  if (open === undefined || band === undefined || band.tracker.state !== "read" || read === undefined) {
    return <section className="deck-pane issues-reader" aria-label="Issue" tabIndex={-1} />;
  }
  if (read.state === "failed") {
    return (
      <section className="deck-pane issues-reader" aria-label="Issue" tabIndex={-1}>
        <p className="issues-list__said" role="alert">
          {read.said}
        </p>
      </section>
    );
  }
  return <Reader key={`${band.manifestId}\n${open.reference}`} band={band} ticket={read.ticket} {...hosts} />;
}

function Reader({ band, ticket, onOpenLink, onOpenSession, onStart }: Hosts & { band: Band; ticket: TicketRead }) {
  const holders = useHolders(ticket.holders);
  return (
    <section className="deck-pane issues-reader" data-tone={toneOfHolders(holders)} aria-label={`Issue ${ticket.reference}`} tabIndex={-1}>
      <header className="deck-band deck-pane__head">
        <h2 className="deck-pane__title">{ticket.title}</h2>
        <RepositoryChip name={band.label} />
        <span className="deck-pane__acts">
          <Tooltip label="Go to the issue" shortcut={ISSUES_KEYS.reader}>
            <Kbd>{ISSUES_KEYS.reader}</Kbd>
          </Tooltip>
          {holders.length > 0 ? null : (
            <Tooltip label="Start a Job" shortcut={ISSUES_KEYS.start}>
              <Button variant="secondary" size="sm" data-start onClick={() => onStart(band, ticket)}>
                Start a Job
              </Button>
            </Tooltip>
          )}
          <Open address={ticket.url} onOpenLink={onOpenLink} />
        </span>
      </header>
      <div className="issues-reader__meta">
        <Tooltip label="Reference">
          <span className="issues-reader__reference mono">{ticket.reference}</span>
        </Tooltip>
        <IssueState state={ticket.state} />
        {ticket.author === undefined ? null : (
          <Tooltip label="Reported by">
            <span className="issues-reader__who">{ticket.author}</span>
          </Tooltip>
        )}
        {ticket.assignees.map((one) => (
          <Tooltip key={one} label="Assigned to">
            <span className="issues-reader__who">{one}</span>
          </Tooltip>
        ))}
      </div>
      <div className="issues-reader__body">
        {holders.length === 0 ? null : (
          <section className="issues-reader__part" aria-label="Jobs">
            <h3 className="issues-reader__eyebrow">Jobs</h3>
            <div className="issues-reader__jobs">
              {holders.map((one) => (
                <IssueJobCard key={one.id} session={one} onOpen={onOpenSession} />
              ))}
            </div>
          </section>
        )}
        {ticket.fields.length === 0 ? null : (
          <dl className="issues-fields" aria-label="Fields">
            {ticket.fields.map((field) => (
              <div key={field.name} className="issues-fields__row">
                <dt className="issues-fields__name">{field.name}</dt>
                <dd className="issues-fields__values">
                  {field.values.map((value) => (
                    <span key={value} className="issues-fields__value">
                      {value}
                    </span>
                  ))}
                </dd>
              </div>
            ))}
          </dl>
        )}
        {ticket.body === "" ? null : (
          <section className="issues-reader__part" aria-label="Description">
            <h3 className="issues-reader__eyebrow">Description</h3>
            <Prose text={ticket.body} />
          </section>
        )}
        {ticket.linked_pull_requests.length === 0 ? null : (
          <section className="issues-reader__part" aria-label="Linked pull requests">
            <h3 className="issues-reader__eyebrow">Pull requests</h3>
            <div className="issues-reader__pulls">
              {ticket.linked_pull_requests.map((one) => (
                <PullRequestMenuOn key={one.number} manifestId={band.manifestId} number={one.number} title={one.title} address={one.address}>
                  {(on) => <PullRequestChip framed number={one.number} title={one.title} state={one.state} {...(one.checks === undefined ? {} : { checks: one.checks })} onOpen={() => onOpenLink(one.address)} {...on} />}
                </PullRequestMenuOn>
              ))}
            </div>
          </section>
        )}
      </div>
    </section>
  );
}

/** One reported bug whole. Keep sets it aside; once kept, Start a Job puts it in a sheet as the brief. Drop removes it. */
function ReportReader({ report, armada, onOpenSession, onStartReport }: Hosts & { report: ReportedBug }) {
  const store = useIssuesStore()!;
  return (
    <section className="deck-pane issues-reader" aria-label="Reported bug" tabIndex={-1}>
      <header className="deck-band deck-pane__head">
        <h2 className="deck-pane__title">{report.title}</h2>
        {armada === undefined ? null : <RepositoryChip name={armada.label} />}
        <span className="deck-pane__acts">
          <Tooltip label="Go to the report" shortcut={ISSUES_KEYS.reader}>
            <Kbd>{ISSUES_KEYS.reader}</Kbd>
          </Tooltip>
          {report.kept ? (
            armada === undefined ? null : (
              <Tooltip label="Start a Job" shortcut={ISSUES_KEYS.start}>
                <Button variant="secondary" size="sm" data-start onClick={() => onStartReport(armada.root, report)}>
                  Start a Job
                </Button>
              </Tooltip>
            )
          ) : (
            <Tooltip label="Keep" shortcut={ISSUES_KEYS.keep}>
              <Button variant="secondary" size="sm" data-keep onClick={() => store.keepReport(report.id)}>
                Keep
              </Button>
            </Tooltip>
          )}
          <Tooltip label="Drop" shortcut={ISSUES_KEYS.drop}>
            <Button variant="ghost" size="sm" data-drop onClick={() => store.dropReport(report.id)}>
              Drop
            </Button>
          </Tooltip>
        </span>
      </header>
      <div className="issues-reader__meta">
        <span className="issues-reader__from">
          <span className="issues-reader__from-label">From</span>
          <Tooltip label="Open the Job">
            <Button variant="ghost" size="sm" onClick={() => onOpenSession(report.session_id)}>
              {report.job_title}
            </Button>
          </Tooltip>
        </span>
        <span className="issues-reader__who">
          <Stamp at={report.at} />
        </span>
      </div>
      <div className="issues-reader__body">
        <section className="issues-reader__part issues-report__part" aria-label="What went wrong">
          <h3 className="issues-reader__eyebrow">What went wrong</h3>
          <Prose text={report.what} />
        </section>
        {report.steps.length === 0 ? null : (
          <section className="issues-reader__part issues-report__part" aria-label="Steps">
            <h3 className="issues-reader__eyebrow">Steps</h3>
            <ol className="issues-report__steps">
              {report.steps.map((one) => (
                <li key={one}>{one}</li>
              ))}
            </ol>
          </section>
        )}
        {report.errors.length === 0 ? null : <Lines name="Errors" lines={report.errors} />}
        {report.logs.length === 0 ? null : <Lines name="Logs" lines={report.logs} />}
      </div>
    </section>
  );
}

/** Exact lines, as the agent gave them, in mono. */
function Lines({ name, lines }: { name: string; lines: readonly string[] }) {
  return (
    <section className="issues-reader__part issues-report__part" aria-label={name}>
      <h3 className="issues-reader__eyebrow">{name}</h3>
      <ul className="issues-report__lines">
        {lines.map((one) => (
          <li key={one}>
            <code className="issues-report__line mono">{one}</code>
          </li>
        ))}
      </ul>
    </section>
  );
}
