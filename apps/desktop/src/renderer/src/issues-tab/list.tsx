// The issues of the repository picked, or of every repository under its heading on All. **One dropdown chooses the query**:
// Open and Assigned to me for every repository, then the queries he saved, then Describe a query, which puts a box in place of
// the list. What he describes, a model writes as a query in each tracker's own language; the list shows that text, the fields
// it chose for the rows, and what it matched, and Save keeps it under a name. A saved query's text is his to edit by hand,
// which runs it again as written. The choice is remembered for each repository, and for All.

import { useEffect, useState } from "react";
import { LoaderCircle, Pencil, Puzzle, X } from "lucide-react";
import {
  Button,
  DropdownMenu,
  Input,
  Kbd,
  ListCard,
  ListCardChip,
  ListCardMark,
  ListCardPending,
  ListCardTime,
  PrMark,
  SearchField,
  Skeleton,
  Textarea,
  Tooltip,
  forgetKept,
  sendsOn,
  useKept,
} from "@armada/components";
import type { DropdownMenuEntry } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";
import { jobIdOf } from "@armada/shell";

import { useSessions } from "../sessions-draft";
import {
  askedOf,
  bandsIn,
  issueHoldings,
  readable,
  savedNames,
  type Asked,
  type Band,
} from "./bands";
import { ISSUES_KEYS, capOf } from "./key-table";
import {
  ClosedMark,
  FixedMark,
  JobMark,
  KeptMark,
  LinkedPullRequestMark,
  UnreadMark,
  jobTitle,
  loudestHolder,
  toneOfHolders,
  useHolders,
} from "./marks";
import { ASSIGNED, CLOSED, DESCRIBE, OPEN, savedChoice, savedName } from "./model";
import type { ReadTracker, ReportedBug, Ticket } from "./model";
import { jobOfReport, reportsShown } from "./reports";
import { issueFieldOf, issueViewEntries, useIssueView } from "./row-view";
import { useIssues, useIssuesStore } from "./store";

const BUILT_INS = [
  {
    id: OPEN,
    label: "Open",
  },
  {
    id: ASSIGNED,
    label: "Assigned to me",
  },
  {
    id: CLOSED,
    label: "Closed",
  },
];

const DESCRIBE_KEEP = "issues:describe";
const SEARCH_KEEP = "issues:search";

/** Whether a ticket answers the words in the bar: its title, number or labels, in any case. */
const matches = (ticket: Ticket, words: string): boolean => {
  const wanted = words.trim().toLowerCase();
  if (wanted === "") return true;
  const labels = ticket.fields
    .filter((one) => /^labels?$/i.test(one.name))
    .flatMap((one) => one.values);
  return [
    ticket.title,
    ticket.reference,
    ...labels,
  ].some((one) => one.toLowerCase().includes(wanted));
};

/** The query chosen for a scope, kept across leaving the tab: a repository's Manifest id, or All. */
export function useChoice(picked: RepositorySummary | null): [
  string,
  (next: string) => void,
] {
  const [kept, choose] = useKept<string>(
    `issues:query:${picked === null ? "all" : (jobIdOf(picked) ?? picked.root)}`,
    OPEN,
  );
  return [
    kept,
    choose,
  ];
}

export function IssuesList({
  picked,
  repositories,
  onTeach,
}: {
  picked: RepositorySummary | null;
  repositories: readonly RepositorySummary[];
  onTeach: (band: Band) => void;
}) {
  const store = useIssuesStore()!;
  const snap = useIssues();
  const [kept, choose] = useChoice(picked);
  const [narrow, setNarrow] = useKept<string>(SEARCH_KEEP, "");
  const scope = (
    picked === null
      ? repositories
      : [
          picked,
        ]
  ).flatMap((one) => jobIdOf(one) ?? []);
  const scopeKey = scope.join("\n");
  useEffect(
    () => store.readTrackers(scope),
    [
      store,
      scopeKey,
    ],
  ); // eslint-disable-line react-hooks/exhaustive-deps

  const bands = bandsIn(picked, repositories, snap.trackers ?? []);
  // Until the trackers have answered or been refused, the list is the shape of what is coming. Trackers read with nothing in them are not this.
  const pending = snap.trackers === undefined && snap.failed === undefined;
  const names = savedNames(bands);
  // A saved query removed from every repository in scope falls back to Open.
  const choice = savedName(kept) !== undefined && !names.includes(savedName(kept)!) ? OPEN : kept;
  const describing = snap.describing;
  const boxed = describing?.phase === "box";
  const asked = bands.map((band) => ({
    band,
    asked: readable(band) ? askedOf(band, choice, describing) : undefined,
  }));

  // Each query runs when a list first needs it. Nothing reads it again until it changes.
  const runs = asked.flatMap(({ band, asked: one }) =>
    one?.state === "query"
      ? [
          [
            band.manifestId,
            one.query,
          ] as const,
        ]
      : [],
  );
  const runsKey = runs.map((one) => one.join("\n")).join("\n\n");
  useEffect(() => {
    for (const [manifestId, query] of runs) store.run(manifestId, query);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    store,
    runsKey,
  ]);

  // A Job that takes or gives back an issue changes the card in place: the lists drawn are read again for that repository.
  const sessions = useSessions();
  const holdingsKey = [
    ...new Set(runs.map(([manifestId]) => manifestId)),
  ]
    .map((id) => `${id}\n${issueHoldings(sessions, id)}`)
    .join("\n\n");
  useEffect(() => {
    for (const manifestId of new Set(runs.map(([id]) => id))) {
      store.refreshHolders(
        manifestId,
        runs.filter(([id]) => id === manifestId).map(([, query]) => query),
        issueHoldings(sessions, manifestId),
      );
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    store,
    holdingsKey,
  ]);

  const shown = reportsShown(picked, repositories);
  const only = picked === null ? undefined : bands[0];
  const here = boxed || describing?.phase === "written" ? DESCRIBE : choice;
  const entries: DropdownMenuEntry[] = [
    ...BUILT_INS.map(
      (one): DropdownMenuEntry => ({
        kind: "item",
        id: one.id,
        label: one.label,
        selected: here === one.id,
      }),
    ),
    ...(names.length === 0
      ? []
      : [
          {
            kind: "separator",
            id: "saved-rule",
          } as const,
          {
            kind: "label",
            id: "saved-label",
            label: "Saved",
          } as const,
        ]),
    ...names.map(
      (name): DropdownMenuEntry => ({
        kind: "item",
        id: savedChoice(name),
        label: name,
        selected: here === savedChoice(name),
      }),
    ),
    {
      kind: "separator",
      id: "describe-rule",
    },
    {
      kind: "item",
      id: DESCRIBE,
      label: "Describe a query…",
      selected: here === DESCRIBE,
      shortcut: capOf(ISSUES_KEYS.describe),
    },
  ];
  // A right press on the list, or the menu key on a row, opens the View menu (`row-view.ts`) where it was pressed.
  const [menu, setMenu] = useState<{
    x: number;
    y: number;
    row: HTMLElement | null;
  }>();
  const [view, toggleField] = useIssueView();
  const triggerLabel =
    here === DESCRIBE
      ? "Describe a query"
      : (BUILT_INS.find((one) => one.id === choice)?.label ?? savedName(choice) ?? "");

  return (
    <section
      className="deck-panel deck-list issues-list"
      aria-label="Issues"
      tabIndex={-1}
      onContextMenu={(event) => {
        const target = event.target as Element;
        if (target.closest("input, textarea, [role='menu']") !== null) return;
        event.preventDefault();
        setMenu({
          x: event.clientX,
          y: event.clientY,
          row: target.closest<HTMLElement>(".issues-row"),
        });
      }}
      onKeyDown={(event) => {
        if (event.key !== "ContextMenu" && !(event.key === "F10" && event.shiftKey)) return;
        const row = (event.target as Element).closest<HTMLElement>(".issues-row");
        if (row === null) return;
        event.preventDefault();
        const box = row.getBoundingClientRect();
        setMenu({
          x: box.left + box.width / 2,
          y: box.bottom,
          row,
        });
      }}
    >
      {only !== undefined && only.tracker.state === "refused" ? (
        <Refused band={only} onTeach={onTeach} />
      ) : (
        <>
          <div className="issues-list__search">
            <SearchField
              label="Search the issues"
              value={narrow}
              onValue={setNarrow}
              cap={ISSUES_KEYS.search}
            />
          </div>
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
            {only !== undefined &&
            only.tracker.state === "read" &&
            only.tracker.mod !== undefined ? (
              <ModMark mod={only.tracker.mod} />
            ) : null}
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
              {describing?.phase === "written" ? (
                <SaveWritten
                  key={describing.words}
                  words={describing.words}
                  onSaved={(name) => choose(savedChoice(name))}
                />
              ) : null}
              {only !== undefined ? (
                <>
                  {shown ? <Reported narrow={narrow} choice={choice} alone /> : null}
                  {asked[0]?.asked === undefined ? null : (
                    <QueryHead band={only} asked={asked[0].asked} />
                  )}
                  <Rows band={only} asked={asked[0]?.asked} narrow={narrow} />
                </>
              ) : (
                <ul className="deck-list__rows issues-list__rows">
                  {shown ? <Reported narrow={narrow} choice={choice} /> : null}
                  {pending ? <PendingBands bands={picked === null ? scope.length : 0} /> : null}
                  {asked.map(({ band, asked: one }) =>
                    band.tracker.state === "read" && one === undefined ? null : (
                      <li key={band.manifestId} className="deck-list__band" aria-label={band.label}>
                        <h3 className="issues-list__repository">
                          {band.label}
                          {band.tracker.state === "read" && band.tracker.mod !== undefined ? (
                            <ModMark mod={band.tracker.mod} />
                          ) : null}
                        </h3>
                        {band.tracker.state === "refused" ? (
                          <Refused band={band} onTeach={onTeach} />
                        ) : (
                          <>
                            {one === undefined ? null : <QueryHead band={band} asked={one} />}
                            <Rows band={band} asked={one} narrow={narrow} nested />
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
      {menu === undefined ? null : (
        <DropdownMenu
          triggerLabel="Issues"
          at={{
            x: menu.x,
            y: menu.y,
          }}
          open
          entries={issueViewEntries(view)}
          onOpenChange={(open) => {
            if (open) return;
            setMenu(undefined);
            menu.row?.focus();
          }}
          onSelect={(chosen) => {
            const field = issueFieldOf(chosen);
            if (field === undefined) return;
            toggleField(field);
            setMenu({
              ...menu,
            });
          }}
        />
      )}
    </section>
  );
}

/** Whether a report answers the words in the bar: its title or the Job it came from. */
const reportMatches = (report: ReportedBug, words: string): boolean => {
  const wanted = words.trim().toLowerCase();
  return (
    wanted === "" ||
    [
      report.title,
      report.job_title,
    ].some((one) => one.toLowerCase().includes(wanted))
  );
};

/** The bugs agents reported in Armada, in a band over the repositories' issues. A fixed report is under Closed and nowhere else. With no report, nothing is drawn. */
function Reported({
  narrow,
  choice,
  alone = false,
}: {
  narrow: string;
  choice: string;
  alone?: boolean;
}) {
  const store = useIssuesStore()!;
  const snap = useIssues();
  useEffect(
    () => store.readReports(),
    [
      store,
    ],
  );
  const reports = snap.reports.filter(
    (one) => (one.fixed !== undefined) === (choice === CLOSED) && reportMatches(one, narrow),
  );
  if (reports.length === 0) return null;
  const band = (
    <li className="deck-list__band" aria-label="Reported bugs">
      <h3 className="issues-list__repository">Reported bugs</h3>
      <ul className="deck-list__group">
        {reports.map((report) => (
          <ReportRow
            key={report.id}
            report={report}
            unread={!report.kept && !snap.seen.includes(report.id)}
            open={snap.openReport === report.id}
          />
        ))}
      </ul>
    </li>
  );
  return alone ? <ul className="deck-list__rows issues-list__rows">{band}</ul> : band;
}

function ReportRow({
  report,
  unread,
  open,
}: {
  report: ReportedBug;
  unread: boolean;
  open: boolean;
}) {
  const store = useIssuesStore()!;
  const job = jobOfReport(useSessions(), report.id);
  return (
    <li className="deck-list__item">
      <ListCard
        className="issues-row"
        data-report={report.id}
        selected={open}
        aria-label={`${report.title}, from ${report.job_title}`}
        onClick={() => store.openReport(report.id)}
        title={report.title}
        wrap
        marks={
          <>
            <ListCardMark>
              {report.fixed === undefined ? null : (
                <FixedMark pullRequest={report.fixed.pull_request} />
              )}
            </ListCardMark>
            <ListCardMark>{job === undefined ? null : <JobMark session={job} />}</ListCardMark>
            <ListCardMark>
              {report.kept ? <KeptMark /> : unread ? <UnreadMark /> : null}
            </ListCardMark>
          </>
        }
        tag={<span className="issues-row__from">{report.job_title}</span>}
        at={<ListCardTime at={report.at} said="Reported" />}
      />
    </li>
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
      <Button
        variant="secondary"
        size="sm"
        className="issues-refused__teach"
        onClick={() => onTeach(band)}
      >
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
  if (asked.state === "failed")
    return (
      <p className="issues-list__said" role="alert">
        {asked.said}
      </p>
    );
  // A built-in's text is the tracker's and never changes, so it is not drawn.
  if (asked.saved === undefined && asked.written === undefined) return null;
  const saved = asked.saved;
  if (saved !== undefined && editing?.manifestId === band.manifestId && editing.id === saved.id)
    return <EditQuery band={band} saved={saved} />;
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
              <Button
                variant="ghost"
                size="sm"
                iconOnly
                aria-label="Edit the query"
                onClick={() => store.edit(band.manifestId, saved.id)}
              >
                <Pencil size={16} strokeWidth={2} aria-hidden />
              </Button>
            </Tooltip>
            <Tooltip label="Remove this query">
              <Button
                variant="ghost"
                size="sm"
                iconOnly
                aria-label="Remove this query"
                onClick={() => store.remove(band.manifestId, saved.id)}
              >
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
    store.saveEdited(band.manifestId, {
      ...saved,
      query: text.trim(),
    });
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
          if (
            (sendsOn(event) || (event.key === "Enter" && !event.shiftKey)) &&
            !event.nativeEvent.isComposing
          ) {
            event.preventDefault();
            save();
          }
        }}
      />
      <span className="issues-query__acts">
        <Button
          variant="ghost"
          size="sm"
          type="button"
          onClick={() => (forgetKept(keep), store.edit(undefined))}
        >
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
          if (
            (sendsOn(event) || (event.key === "Enter" && !event.shiftKey)) &&
            !event.nativeEvent.isComposing
          ) {
            event.preventDefault();
            send();
          }
        }}
      />
      <Tooltip label="A model writes the query once, on send" asChild>
        <Button
          variant="secondary"
          size="sm"
          type="submit"
          disabled={words.trim() === "" || manifestIds.length === 0}
        >
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
  const ready =
    describing?.phase === "written" &&
    Object.values(describing.written).some((one) => one.state === "read");
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
        <Button
          variant="ghost"
          size="sm"
          iconOnly
          type="button"
          aria-label="Discard"
          onClick={() => store.stopDescribing()}
        >
          <X size={16} strokeWidth={2} aria-hidden />
        </Button>
      </Tooltip>
    </form>
  );
}

function Rows({
  band,
  asked,
  narrow,
  nested = false,
}: {
  band: Band;
  asked: Asked | undefined;
  narrow: string;
  nested?: boolean;
}) {
  const store = useIssuesStore()!;
  const snap = useIssues();
  if (band.tracker.state !== "read" || asked?.state !== "query") return null;
  const run = store.runOf(band.manifestId, asked.query);
  // A search not yet run, or running, is the shape of the rows it will draw. A search that came back empty is not this.
  if (run === undefined || run.state === "running") {
    return (
      <ul className={nested ? "deck-list__group" : "deck-list__rows issues-list__rows"} aria-hidden>
        <PendingRows count={nested ? 3 : 4} />
      </ul>
    );
  }
  if (run?.state === "failed") {
    const tool = run.missing ?? run.needsSetup;
    return tool === undefined ? (
      <p className="issues-list__said" role="alert">
        {run.said}
      </p>
    ) : (
      <MissingTool
        manifestId={band.manifestId}
        query={asked.query}
        said={run.said}
        tool={tool}
        setup={run.missing === undefined}
      />
    );
  }
  const tickets = run?.state === "read" ? run.tickets.filter((one) => matches(one, narrow)) : [];
  return (
    <ul className={nested ? "deck-list__group" : "deck-list__rows issues-list__rows"}>
      {tickets.map((ticket) => (
        <Row
          key={ticket.reference}
          band={band}
          ticket={ticket}
          fields={asked.fields}
          open={
            snap.open?.manifestId === band.manifestId && snap.open.reference === ticket.reference
          }
        />
      ))}
    </ul>
  );
}

/** Rows as the list will draw them, in bars: a title, and the reference under it. The marks are the tracker's to say, so they wait. */
function PendingRows({ count }: { count: number }) {
  return (
    <>
      {Array.from(
        {
          length: count,
        },
        (_, at) => (
          <li key={at} className="deck-list__item">
            <ListCardPending width={`${72 - ((at * 11) % 24)}%`} />
          </li>
        ),
      )}
    </>
  );
}

/** Before any tracker has answered: a heading and rows for each repository on All, or the rows alone for one picked. */
function PendingBands({ bands }: { bands: number }) {
  if (bands === 0) return <PendingRows count={4} />;
  return (
    <>
      {Array.from(
        {
          length: bands,
        },
        (_, at) => (
          <li key={at} className="deck-list__band" aria-hidden>
            <h3 className="issues-list__repository" data-pending>
              <Skeleton width="30%" />
            </h3>
            <ul className="deck-list__group">
              <PendingRows count={3} />
            </ul>
          </li>
        ),
      )}
    </>
  );
}

/** What a person calls a tool by: `bd` is beads. */
const called = (tool: string) => (tool === "bd" ? "beads" : tool);

/** A search that could not run for want of a program, or of its project (`setup`). Nothing installs or sets up until the button is pressed; a failure keeps its words and offers no second press. */
export function MissingTool({
  manifestId,
  query,
  said,
  tool,
  setup = false,
}: {
  manifestId: string;
  query: string;
  said: string;
  tool: string;
  setup?: boolean;
}) {
  const store = useIssuesStore()!;
  useIssues();
  const install = store.installOf(manifestId, tool);
  if (install?.state === "failed")
    return (
      <p className="issues-list__said" role="alert">
        {install.said}
      </p>
    );
  const installing = install?.state === "installing";
  return (
    <div className="issues-list__missing">
      <p className="issues-list__said" role="alert">
        {said}
      </p>
      <Button
        variant="secondary"
        size="sm"
        pending={installing}
        onClick={() => store.install(manifestId, tool, query)}
      >
        {setup
          ? installing
            ? `Setting up ${called(tool)}`
            : `Set up ${called(tool)}`
          : installing
            ? `Installing ${tool}`
            : `Install ${tool}`}
      </Button>
    </div>
  );
}

function Row({
  band,
  ticket,
  fields,
  open,
}: {
  band: Band;
  ticket: Ticket;
  fields: readonly string[];
  open: boolean;
}) {
  const store = useIssuesStore()!;
  const [view] = useIssueView();
  const holders = useHolders(ticket.holders);
  const values = fields.flatMap((name) =>
    (ticket.fields.find((one) => one.name === name)?.values ?? []).map((value) => ({
      name,
      value,
    })),
  );
  const tone = toneOfHolders(holders);
  const lines = [
    !view.job || holders.length === 0 ? null : (
      <span key="held" className="issues-row__held">
        {holders.map((one) => (
          <span key={one.id} className="issues-row__job">
            <JobMark session={one} />
            <span className="issues-row__job-title">{jobTitle(one)}</span>
          </span>
        ))}
      </span>
    ),
    !view.labels || values.length === 0 ? null : (
      <span key="fields" className="issues-row__fields">
        {values.map(({ name, value }) => (
          <Tooltip key={`${name}:${value}`} label={name}>
            <span className="issues-row__value">{value}</span>
          </Tooltip>
        ))}
      </span>
    ),
  ].filter((one) => one !== null);
  return (
    <li className="deck-list__item">
      <ListCard
        className="issues-row"
        selected={open}
        data-tone={tone}
        needs={view.state && tone === "ask" && !open}
        aria-label={`${ticket.reference} ${ticket.title}`}
        onClick={() => store.open(band.manifestId, ticket.reference)}
        title={ticket.title}
        wrap
        marks={
          <>
            <ListCardMark>
              {ticket.linked_pull_request ? <LinkedPullRequestMark /> : null}
            </ListCardMark>
            <ListCardMark>
              {!view.state ? null : ticket.state === "closed" ? (
                <ClosedMark />
              ) : loudestHolder(holders) === undefined ? null : (
                <JobMark session={loudestHolder(holders)!} />
              )}
            </ListCardMark>
          </>
        }
        {...(view.number
          ? {
              tag: <ListCardChip>{ticket.reference}</ListCardChip>,
            }
          : {})}
        at={<ListCardTime at={ticket.updated_at} said="Last updated" />}
        {...(lines.length === 0
          ? {}
          : {
              lines,
            })}
      />
    </li>
  );
}
