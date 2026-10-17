// The Checks page: every Check the selected Manifest has had requested or run, each with its state
// and who asked for it, and one opened in the log panel with its facts over its log (owner,
// 6 Oct 2026). Read off what the app already holds: the checkout's run sheet and run list, and the
// merge line's Checks.
//
// **One panel for the log, `LogSheet`'s.** A Check still out is followed as it prints, which is the
// socket the Manifest surface opens; one that has ended is read whole off `get_checkout_run_output`.
// A merge line Check is `LandCheckLogSheet`'s own socket. **Nothing stands in for lines that have not
// arrived**, and no count is drawn over the rows.

import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";

import { Button, CheckDetails, CheckList, ConsoleOutput, Tabs, type ConsoleRow } from "@armada/components";
import type {
  CheckoutRunFollowed,
  CheckoutRunListRead,
  CheckoutRunRecord,
  CheckoutRunSheetRead,
  FollowedLandLog,
  LandCheckAt,
  MergeLine,
  RunOutputRead,
} from "@armada/protocol";

import { LandCheckLogSheet } from "./check-log-sheet";
import {
  askerOf,
  CHECK_FILTER_LABEL,
  CHECK_FILTERS,
  checkDetailsOf,
  checkEntriesOf,
  checkRowOf,
  heldBy,
  type Asker,
  type CheckEntry,
  type CheckFilter,
} from "./manifest-checks";
import { LogSheet } from "./log-sheet";

export type ManifestChecksProps = {
  /** Draw only the focused run's panel, over whatever surface asked; the list and filters are left out. */
  only?: boolean;
  /** The panel closed. */
  onClosed?: () => void;
  /** `GET /manifest/run_sheet`, held open while this page is showing. */
  sheet: CheckoutRunSheetRead;
  /** The run being read, as it prints. */
  followed: CheckoutRunFollowed;
  /** One run's output, or `null` to stop. */
  onObserveRun: (runId: string | null) => void;
  onListRuns: () => Promise<CheckoutRunListRead>;
  onGetRunOutput: (runId: string) => Promise<RunOutputRead>;
  /** The merge line of each served repository, narrowed to this one by the host. */
  lines: readonly MergeLine[];
  /** The merge line Check being followed, and the ask to follow one. */
  landFollowed: FollowedLandLog;
  onFollowLand: (at: LandCheckAt | null) => void;
  /** Where a requester is opened: the merge line at a branch. */
  onOpenMergeLine: (branch?: string) => void;
  /** The window is at `--window-floor`. */
  floor: boolean;
  /** A Check to have open on arriving. */
  focus?: string;
};

/** The filter row, a panel's filters as filled tabs. All on opening, no figure on any. */
function CheckFilters({ value, onChange }: { value: CheckFilter; onChange: (filter: CheckFilter) => void }) {
  return (
    <Tabs
      items={CHECK_FILTERS.map((id) => ({ id, label: CHECK_FILTER_LABEL[id] }))}
      value={value}
      onChange={(id) => onChange(id as CheckFilter)}
    />
  );
}

export function ManifestChecks(props: ManifestChecksProps) {
  const { sheet, onListRuns, lines, floor } = props;
  const [runs, setRuns] = useState<readonly CheckoutRunRecord[]>([]);
  const [openId, setOpenId] = useState<string | null>(null);
  const [filter, setFilter] = useState<CheckFilter>("all");
  const data = sheet.state === "read" ? sheet.sheet : undefined;

  // Read on opening, and again the moment a run ends: `checkout_run.finished` moves the sheet's
  // `running`, which is the signal the list has one more row.
  const runningId = data?.running?.id;
  const verifyId = data?.verify?.id;
  const verifyEnded = data?.verify?.ended_at;
  useEffect(() => {
    let current = true;
    void onListRuns().then((read) => {
      if (current && read.ok) setRuns(read.runs.runs);
    });
    return () => {
      current = false;
    };
  }, [onListRuns, runningId, verifyId, verifyEnded]);

  const { focus } = props;
  const entries = useMemo(() => checkEntriesOf(data, runs, lines), [data, runs, lines]);
  useEffect(() => {
    if (focus === undefined) return;
    setFilter("all");
    setOpenId(focus);
  }, [focus]);
  const shown = useMemo(() => entries.filter((one) => heldBy(filter, one)), [entries, filter]);
  // **Only the panel**, over the surface it was pressed from: a Session's Check opens here and
  // closing it leaves the person where they were (owner, 8 Oct 2026).
  const only = props.only === true;
  const open = (only ? entries : shown).find((one) => one.id === openId);
  const close = () => {
    setOpenId(null);
    props.onClosed?.();
  };
  const bands = open === undefined ? undefined : <FactsOf entry={open} {...props} />;

  // Panel-only takes no room of its own: the sheet floats over the surface that asked, and the
  // page around it would be laid out beside that surface and squeeze it.
  if (only) return <>{open === undefined ? null : open.land !== undefined ? (
        <LandCheckLogSheet
          key={open.id}
          at={open.land}
          followed={props.landFollowed}
          onFollow={props.onFollowLand}
          bands={bands}
          floor={floor}
          onClose={close}
        />
      ) : (
        <CheckLogPanel key={open.id} entry={open} bands={bands} {...props} onClose={close} />
      )}</>;
  return (
    <div className="armada-screen__overview">
      <CheckList
        rows={shown.map((one) => checkRowOf(one))}
        openRow={open?.id ?? null}
        onOpenRow={setOpenId}
        controls={
          <CheckFilters
            value={filter}
            onChange={(next) => {
              setFilter(next);
              setOpenId(null);
            }}
          />
        }
      />
      {open === undefined ? null : open.land !== undefined ? (
        <LandCheckLogSheet
          key={open.id}
          at={open.land}
          followed={props.landFollowed}
          onFollow={props.onFollowLand}
          bands={bands}
          floor={floor}
          onClose={close}
        />
      ) : (
        <CheckLogPanel key={open.id} entry={open} bands={bands} {...props} onClose={close} />
      )}
    </div>
  );
}

/** The Check's facts, with who asked as a link where a route to them exists. */
function FactsOf({ entry, onOpenMergeLine }: ManifestChecksProps & { entry: CheckEntry }) {
  const asker = askerOf(entry.requester);
  return <CheckDetails details={checkDetailsOf(entry, <RequestedBy asker={asker} onOpenMergeLine={onOpenMergeLine} />)} />;
}

/** Who asked, pressable where the requester is the merge line, which has a row to open. */
function RequestedBy({ asker, onOpenMergeLine }: { asker: Asker; onOpenMergeLine: ManifestChecksProps["onOpenMergeLine"] }): ReactNode {
  const { opens, label } = asker;
  if (opens === undefined || opens.to !== "merge-line") return label;
  return (
    <Button variant="ghost" size="sm" onClick={() => onOpenMergeLine(opens.branch)}>
      {label}
    </Button>
  );
}

/** The open checkout Check: its log under its facts, live while it is out. */
function CheckLogPanel({
  entry,
  bands,
  followed,
  onObserveRun,
  onGetRunOutput,
  floor,
  onClose,
}: {
  entry: CheckEntry;
  bands: ReactNode;
  followed: CheckoutRunFollowed;
  onObserveRun: (runId: string | null) => void;
  onGetRunOutput: (runId: string) => Promise<RunOutputRead>;
  floor: boolean;
  onClose: () => void;
}) {
  const { runId, status } = entry;
  const running = status === "running";
  const [kept, setKept] = useState<{ runId: string; rows: ConsoleRow[] } | null>(null);
  const [wrap, setWrap] = useState(false);
  // What was last drawn, so a Check that ends with its panel open keeps its lines until the whole
  // log has been read, rather than emptying in between.
  const drawn = useRef<ConsoleRow[]>([]);

  // Opening the panel is the ask, once; closing it lets a followed one go.
  useEffect(() => {
    if (runId === undefined) return;
    if (running) {
      onObserveRun(runId);
      return () => onObserveRun(null);
    }
    let current = true;
    void onGetRunOutput(runId).then((read) => {
      if (!current || !read.ok) return;
      const { output } = read;
      setKept({ runId, rows: output.lines.map((text, at) => ({ row: "line" as const, at: output.from_line + at, text })) });
    });
    return () => {
      current = false;
    };
  }, [runId, running, onObserveRun, onGetRunOutput]);

  const ours = followed.state === "following" && followed.runId === runId ? followed : undefined;
  let rows: ConsoleRow[];
  if (running) rows = (ours?.lines ?? []).map((text, at) => ({ row: "line" as const, at: ours!.fromLine + at, text }));
  else if (kept !== null && kept.runId === runId) rows = kept.rows;
  else rows = drawn.current;
  drawn.current = rows;
  const live = running && (ours === undefined || ours.ended === undefined);
  return (
    <LogSheet
      kind="check-log"
      placement="floating"
      title="Check"
      about={<span className="mono">{entry.name}</span>}
      live={live}
      grows={rows.length}
      bleed
      wrap={{ wrap, onToggle: () => setWrap((was) => !was) }}
      bands={bands}
      floor={floor}
      onClose={onClose}
    >
      <ConsoleOutput rows={rows} following={live} wrap={wrap} />
    </LogSheet>
  );
}
