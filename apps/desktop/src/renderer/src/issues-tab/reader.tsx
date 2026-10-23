// The reader: one issue whole. Its title and reference, the way out to the tracker, its state, who reported it and who has it,
// **every field the tracker returned** as names and values (Bridge draws them alike, whichever tracker sent them), the
// description, the pull requests that name it, and the Jobs working it or the one press that starts one.

import { Button, Kbd, Prose, Tooltip } from "@armada/components";

import { IssueState, LinkedPullRequests, Open } from "../shared-panel/panels";
import type { Band } from "./bands";
import { ISSUES_KEYS } from "./key-table";
import { JobMark, jobTitle, useHolders } from "./marks";
import type { ReadTracker, TicketRead } from "./model";
import { useIssues, useIssuesStore } from "./store";

type Hosts = {
  bands: readonly Band[];
  onOpenLink: (address: string) => void;
  onOpenSession: (id: string) => void;
  onStart: (band: Band, ticket: TicketRead) => void;
};

/** With nothing open the pane is empty: an empty slot stays empty. */
export function IssuesReader(hosts: Hosts) {
  const store = useIssuesStore()!;
  const { open } = useIssues();
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
  return <Reader key={`${band.manifestId}\n${open.reference}`} band={band} tracker={band.tracker} ticket={read.ticket} {...hosts} />;
}

function Reader({ band, tracker, ticket, onOpenLink, onOpenSession, onStart }: Hosts & { band: Band; tracker: ReadTracker; ticket: TicketRead }) {
  const holders = useHolders(tracker.held[ticket.reference]);
  return (
    <section className="deck-pane issues-reader" aria-label={`Issue ${ticket.reference}`} tabIndex={-1}>
      <header className="deck-band deck-pane__head">
        <h2 className="deck-pane__title">{ticket.title}</h2>
        <span className="deck-pane__where">{band.label}</span>
        <span className="deck-pane__acts">
          <Tooltip label="Go to the issue" shortcut={ISSUES_KEYS.reader}>
            <Kbd>{ISSUES_KEYS.reader}</Kbd>
          </Tooltip>
          {holders.map((one) => (
            <Button key={one.id} variant="ghost" size="sm" className="issues-reader__job" aria-label={`Job: ${jobTitle(one)}`} onClick={() => onOpenSession(one.id)}>
              <JobMark session={one} />
              {jobTitle(one)}
            </Button>
          ))}
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
            <LinkedPullRequests linked={ticket.linked_pull_requests} onPress={(one) => onOpenLink(one.address)} />
          </section>
        )}
      </div>
    </section>
  );
}
