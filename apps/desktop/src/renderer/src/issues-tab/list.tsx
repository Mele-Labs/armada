// The issues of the repository picked, or of every repository under its heading on All. **One dropdown chooses the query**:
// Open and Assigned to me for every repository, then the queries he saved, then Describe a query, which puts a box in place of
// the list. What he describes, a model writes as a query in each tracker's own language; the list shows that text, the fields
// it chose for the rows, and what it matched, and Save keeps it under a name. A saved query's text is his to edit by hand,
// which runs it again as written. The choice is remembered for each repository, and for All.

import { useEffect, useState } from "react";
import { LoaderCircle, Pencil, Puzzle, X } from "lucide-react";
import { Button, DropdownMenu, Input, Kbd, PrMark, Textarea, Tooltip, forgetKept, sendsOn, useKept } from "@armada/components";
import type { DropdownMenuEntry } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";
import { jobIdOf } from "@armada/shell";

import { useSessions } from "../sessions-draft";
import { askedOf, bandsIn, issueHoldings, readable, savedNames, type Asked, type Band } from "./bands";
import { ISSUES_KEYS, capOf } from "./key-table";
import { JobMark, jobTitle, toneOfHolders, useHolders } from "./marks";
import { ASSIGNED, DESCRIBE, OPEN, savedChoice, savedName } from "./model";
import type { ReadTracker, Ticket } from "./model";
import { useIssues, useIssuesStore } from "./store";

const BUILT_INS = [
  { id: OPEN, label: "Open" },
  { id: ASSIGNED, label: "Assigned to me" },
];

const DESCRIBE_KEEP = "issues:describe";

/** The query chosen for a scope, kept across leaving the tab: a repository's Manifest id, or All. */
export function useChoice(picked: RepositorySummary | null): [string, (next: string) => void] {
  const [kept, choose] = useKept<string>(`issues:query:${picked === null ? "all" : (jobIdOf(picked) ?? picked.root)}`, OPEN);
  return [kept, choose];
}

export function IssuesList({ picked, repositories, onTeach }: { picked: RepositorySummary | null; repositories: readonly RepositorySummary[]; onTeach: (band: Band) => void }) {
  const store = useIssuesStore()!;
  const snap = useIssues();
  const [kept, choose] = useChoice(picked);
  const scope = (picked === null ? repositories : [picked]).flatMap((one) => jobIdOf(one) ?? []);
  const scopeKey = scope.join("\n");
  useEffect(() => store.readTrackers(scope), [store, scopeKey]); // eslint-disable-line react-hooks/exhaustive-deps

  const bands = bandsIn(picked, repositories, snap.trackers ?? []);
  const names = savedNames(bands);
  // A saved query removed from every repository in scope falls back to Open.
  const choice = savedName(kept) !== undefined && !names.includes(savedName(kept)!) ? OPEN : kept;
  const describing = snap.describing;
  const boxed = describing?.phase === "box";
  const asked = bands.map((band) => ({ band, asked: readable(band) ? askedOf(band, choice, describing) : undefined }));

  // Each query runs when a list first needs it. Nothing reads it again until it changes.
  const runs = asked.flatMap(({ band, asked: one }) => (one?.state === "query" ? [[band.manifestId, one.query] as const] : []));
  const runsKey = runs.map((one) => one.join("\n")).join("\n\n");
  useEffect(() => {
    for (const [manifestId, query] of runs) store.run(manifestId, query);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [store, runsKey]);

  // A Job that takes or gives back an issue changes the card in place: the lists drawn are read again for that repository.
  const sessions = useSessions();
  const holdingsKey = [...new Set(runs.map(([manifestId]) => manifestId))].map((id) => `${id}\n${issueHoldings(sessions, id)}`).join("\n\n");
  useEffect(() => {
    for (const manifestId of new Set(runs.map(([id]) => id))) {
      store.refreshHolders(manifestId, runs.filter(([id]) => id === manifestId).map(([, query]) => query), issueHoldings(sessions, manifestId));
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [store, holdingsKey]);

  const only = picked === null ? undefined : bands[0];
  const here = boxed || describing?.phase === "written" ? DESCRIBE : choice;
  const entries: DropdownMenuEntry[] = [
    ...BUILT_INS.map((one): DropdownMenuEntry => ({ kind: "item", id: one.id, label: one.label, selected: here === one.id })),
    ...(names.length === 0 ? [] : [{ kind: "separator", id: "saved-rule" } as const, { kind: "label", id: "saved-label", label: "Saved" } as const]),
    ...names.map((name): DropdownMenuEntry => ({ kind: "item", id: savedChoice(name), label: name, selected: here === savedChoice(name) })),
    { kind: "separator", id: "describe-rule" },
    { kind: "item", id: DESCRIBE, label: "Describe a query…", selected: here === DESCRIBE, shortcut: ISSUES_KEYS.describe },
  ];
  const triggerLabel = here === DESCRIBE ? "Describe a query" : (BUILT_INS.find((one) => one.id === choice)?.label ?? savedName(choice) ?? "");

  return (
    <section className="deck-panel deck-list issues-list" aria-label="Issues" tabIndex={-1}>
      <header className="deck-band">
        Issues
        {only !== undefined && only.tracker.state === "read" && only.tracker.mod !== undefined ? <ModMark mod={only.tracker.mod} /> : null}
        <Tooltip label="Go to Issues" shortcut={ISSUES_KEYS.list}>
          <Kbd>{ISSUES_KEYS.list}</Kbd>
        </Tooltip>
      </header>
      {only !== undefined && only.tracker.state === "refused" ? (
        <Refused band={only} onTeach={onTeach} />
      ) : (
        <>
          <div className="deck-scope issues-list__pick">
            <DropdownMenu
              align="start"
              portal
              triggerLabel={triggerLabel}
              triggerAriaLabel="Query"
              entries={entries}
              onSelect={(id) => {
                if (id === DESCRIBE) return store.describe();
                store.stopDescribing();
                store.edit(undefined);
                choose(id);
              }}
            />
            <Tooltip label="Open the queries" shortcut={capOf(ISSUES_KEYS.queries)}>
              <Kbd>{capOf(ISSUES_KEYS.queries)}</Kbd>
            </Tooltip>
          </div>
          {boxed ? (
            <DescribeBox manifestIds={bands.filter(readable).map((band) => band.manifestId)} />
          ) : (
            <>
              {snap.failed === undefined ? null : (
                <p className="issues-list__said" role="alert">
                  {snap.failed}
                </p>
              )}
              {describing?.phase === "written" ? <SaveWritten key={describing.words} words={describing.words} onSaved={(name) => choose(savedChoice(name))} /> : null}
              {only !== undefined ? (
                <>
                  {asked[0]?.asked === undefined ? null : <QueryHead band={only} asked={asked[0].asked} />}
                  <Rows band={only} asked={asked[0]?.asked} />
                </>
              ) : (
                <ul className="deck-list__rows issues-list__rows">
                  {asked.map(({ band, asked: one }) =>
                    band.tracker.state === "read" && one === undefined ? null : (
                      <li key={band.manifestId} className="deck-list__band" aria-label={band.label}>
                        <h3 className="issues-list__repository">
                          {band.label}
                          {band.tracker.state === "read" && band.tracker.mod !== undefined ? <ModMark mod={band.tracker.mod} /> : null}
                        </h3>
                        {band.tracker.state === "refused" ? (
                          <Refused band={band} onTeach={onTeach} />
                        ) : (
                          <>
                            {one === undefined ? null : <QueryHead band={band} asked={one} />}
                            <Rows band={band} asked={one} nested />
                          </>
                        )}
                      </li>
                    ),
                  )}
                </ul>
              )}
            </>
          )}
        </>
      )}
    </section>
  );
}

/** A tracker read through a mod of his, rather than one Armada ships. */
function ModMark({ mod }: { mod: string }) {
  const said = `Tracker mod: ${mod}`;
  return (
    <Tooltip label={said}>
      <span className="issues-list__mod" role="img" aria-label={said}>
        <Puzzle size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

/** A tracker nothing reads: Fleet's words for why, and the one act that fixes it. */
function Refused({ band, onTeach }: { band: Band; onTeach: (band: Band) => void }) {
  if (band.tracker.state !== "refused") return null;
  return (
    <div className="issues-refused">
      <p className="issues-refused__said">{band.tracker.said}</p>
      <Button variant="secondary" size="sm" className="issues-refused__teach" onClick={() => onTeach(band)}>
        Teach Armada this tracker
      </Button>
    </div>
  );
}

/** The query a list runs, as the tracker reads it, with the fields its rows show. A saved one can be edited or removed. */
function QueryHead({ band, asked }: { band: Band; asked: Asked }) {
  const store = useIssuesStore()!;
  const { editing } = useIssues();
  if (asked.state === "writing") {
    return (
      <div className="issues-query">
        <PrMark Glyph={LoaderCircle} said="Writing the query" tone="running" />
      </div>
    );
  }
  if (asked.state === "failed") return <p className="issues-list__said" role="alert">{asked.said}</p>;
  // A built-in's text is the tracker's and never changes, so it is not drawn.
  if (asked.saved === undefined && asked.written === undefined) return null;
  const saved = asked.saved;
  if (saved !== undefined && editing?.manifestId === band.manifestId && editing.id === saved.id) return <EditQuery band={band} saved={saved} />;
  return (
    <div className="issues-query">
      <code className="issues-query__text mono" aria-label="Query">
        {asked.query}
      </code>
      <span className="issues-query__foot">
        <span className="issues-query__fields" aria-label="Row fields">
          {asked.fields.map((one) => (
            <span key={one} className="issues-query__field">
              {one}
            </span>
          ))}
        </span>
        {saved === undefined ? null : (
          <span className="issues-query__acts">
            <Tooltip label="Edit the query">
              <Button variant="ghost" size="sm" iconOnly aria-label="Edit the query" onClick={() => store.edit(band.manifestId, saved.id)}>
                <Pencil size={16} strokeWidth={2} aria-hidden />
              </Button>
            </Tooltip>
            <Tooltip label="Remove this query">
              <Button variant="ghost" size="sm" iconOnly aria-label="Remove this query" onClick={() => store.remove(band.manifestId, saved.id)}>
                <X size={16} strokeWidth={2} aria-hidden />
              </Button>
            </Tooltip>
          </span>
        )}
      </span>
    </div>
  );
}

/** The saved query's text, by hand. Save keeps it and runs it as written; no model is asked. */
function EditQuery({ band, saved }: { band: Band; saved: ReadTracker["saved"][number] }) {
  const store = useIssuesStore()!;
  const keep = `issues:edit:${band.manifestId}:${saved.id}`;
  const [text, setText] = useState(saved.query);
  const save = () => {
    if (text.trim() === "") return;
    forgetKept(keep);
    store.saveEdited(band.manifestId, { ...saved, query: text.trim() });
  };
  return (
    <form
      className="issues-query issues-query--editing"
      aria-label="Edit the query"
      onSubmit={(event) => {
        event.preventDefault();
        save();
      }}
    >
      <Textarea
        rows={3}
        autoFocus
        keep={keep}
        aria-label="Query"
        className="mono"
        value={text}
        onChange={(event) => setText(event.target.value)}
        onKeyDown={(event) => {
          if ((sendsOn(event) || (event.key === "Enter" && !event.shiftKey)) && !event.nativeEvent.isComposing) {
            event.preventDefault();
            save();
          }
        }}
      />
      <span className="issues-query__acts">
        <Button variant="ghost" size="sm" type="button" onClick={() => (forgetKept(keep), store.edit(undefined))}>
          Cancel
        </Button>
        <Button variant="secondary" size="sm" type="submit" disabled={text.trim() === ""}>
          Save
        </Button>
      </span>
    </form>
  );
}

/** His words, in place of the list. Send, or Enter as in a message box, asks before a model writes the query. */
function DescribeBox({ manifestIds }: { manifestIds: readonly string[] }) {
  const store = useIssuesStore()!;
  const [words, setWords] = useState("");
  const send = () => {
    if (words.trim() === "" || manifestIds.length === 0) return;
    const said = words.trim();
    // The words stay in the box until he has said Run, so Cancel loses nothing.
    store.spend("Describe a query", () => {
      store.write(said, manifestIds);
      forgetKept(DESCRIBE_KEEP);
      setWords("");
    });
  };
  return (
    <form
      className="issues-describe"
      aria-label="Describe a query"
      onSubmit={(event) => {
        event.preventDefault();
        send();
      }}
    >
      <Textarea
        rows={3}
        autoFocus
        keep={DESCRIBE_KEEP}
        aria-label="Describe a query"
        value={words}
        onChange={(event) => setWords(event.target.value)}
        onKeyDown={(event) => {
          if ((sendsOn(event) || (event.key === "Enter" && !event.shiftKey)) && !event.nativeEvent.isComposing) {
            event.preventDefault();
            send();
          }
        }}
      />
      <Tooltip label="A model writes the query once, on send" asChild>
        <Button variant="secondary" size="sm" type="submit" disabled={words.trim() === "" || manifestIds.length === 0}>
          Send
        </Button>
      </Tooltip>
    </form>
  );
}

/** Keeps what the model wrote, under a name that starts as his words. */
function SaveWritten({ words, onSaved }: { words: string; onSaved: (name: string) => void }) {
  const store = useIssuesStore()!;
  const { describing } = useIssues();
  const [name, setName] = useState(words);
  const ready = describing?.phase === "written" && Object.values(describing.written).some((one) => one.state === "read");
  const save = () => {
    const kept = name.trim();
    if (kept === "" || !ready) return;
    void store.saveWritten(kept).then(() => onSaved(kept));
  };
  return (
    <form
      className="issues-save"
      aria-label="Save the query"
      onSubmit={(event) => {
        event.preventDefault();
        save();
      }}
    >
      <Input aria-label="Name" value={name} onChange={(event) => setName(event.target.value)} />
      <Button variant="secondary" size="sm" type="submit" disabled={name.trim() === "" || !ready}>
        Save
      </Button>
      <Tooltip label="Discard">
        <Button variant="ghost" size="sm" iconOnly type="button" aria-label="Discard" onClick={() => store.stopDescribing()}>
          <X size={16} strokeWidth={2} aria-hidden />
        </Button>
      </Tooltip>
    </form>
  );
}

function Rows({ band, asked, nested = false }: { band: Band; asked: Asked | undefined; nested?: boolean }) {
  const store = useIssuesStore()!;
  const snap = useIssues();
  if (band.tracker.state !== "read" || asked?.state !== "query") return null;
  const run = store.runOf(band.manifestId, asked.query);
  if (run?.state === "failed") return <p className="issues-list__said" role="alert">{run.said}</p>;
  const tickets = run?.state === "read" ? run.tickets : [];
  return (
    <ul className={nested ? "deck-list__group" : "deck-list__rows issues-list__rows"}>
      {tickets.map((ticket) => (
        <Row key={ticket.reference} band={band} ticket={ticket} fields={asked.fields} open={snap.open?.manifestId === band.manifestId && snap.open.reference === ticket.reference} />
      ))}
    </ul>
  );
}

function Row({ band, ticket, fields, open }: { band: Band; ticket: Ticket; fields: readonly string[]; open: boolean }) {
  const store = useIssuesStore()!;
  const holders = useHolders(ticket.holders);
  const values = fields.flatMap((name) => (ticket.fields.find((one) => one.name === name)?.values ?? []).map((value) => ({ name, value })));
  return (
    <li className="deck-list__item">
      <button type="button" className="issues-row" data-tone={toneOfHolders(holders)} data-selected={open || undefined} aria-current={open || undefined} aria-label={`${ticket.reference} ${ticket.title}`} onClick={() => store.open(band.manifestId, ticket.reference)}>
        <span className="issues-row__reference mono">{ticket.reference}</span>
        <span className="issues-row__title">{ticket.title}</span>
        <span className="issues-row__jobs">
          {holders.map((one) => (
            <JobMark key={one.id} session={one} />
          ))}
        </span>
        {holders.length === 0 ? null : (
          <span className="issues-row__held">
            {holders.map((one) => (
              <span key={one.id} className="issues-row__job">
                <JobMark session={one} />
                <span className="issues-row__job-title">{jobTitle(one)}</span>
              </span>
            ))}
          </span>
        )}
        {values.length === 0 ? null : (
          <span className="issues-row__fields">
            {values.map(({ name, value }) => (
              <Tooltip key={`${name}:${value}`} label={name}>
                <span className="issues-row__value">{value}</span>
              </Tooltip>
            ))}
          </span>
        )}
      </button>
    </li>
  );
}
