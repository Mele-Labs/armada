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
  FollowedLog,
  ManifestCheckRow,
  ManifestChecksRead,
  LandCheckAt,
  MergeLine,
  RunOutputRead,
} from "@armada/protocol";

import { JobCheckLogSheet, LandCheckLogSheet } from "./check-log-sheet";
import type { JobOpening } from "./detail-props";
import { askerOf, checkDetailsOf, checkEntriesOf, checkRowOf, type Asker, type CheckEntry } from "./manifest-checks";
import { useCheckOutputs, useFollowing, type FollowCheckOutput, type ReadCheckOutput } from "./outputs";
import { LogSheet } from "./log-sheet";

export type ManifestChecksProps = {
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
  /** `list_manifest_checks`: a Job's gate Checks and a Drone's own runs, beside the checkout's and the merge line's. */
  onReadChecks: () => Promise<ManifestChecksRead>;
  /** A Job's Check log, read whole or followed while it is written. */
  onReadCheckOutput: ReadCheckOutput;
  onFollowCheckOutput: FollowCheckOutput;
  followedLog: FollowedLog;
  /** A Job as a person names it, for the link to it. */
  jobLabel: (jobId: string) => string;
  /** Where a requester is opened: a Job at a step or a Drone, and the merge line at a branch. */
  onOpenJob: (jobId: string, to?: JobOpening) => void;
  onOpenMergeLine: (branch?: string) => void;
  /** The window is at `--window-floor`. */
  floor: boolean;
};

export function ManifestChecks(props: ManifestChecksProps) {
  const { sheet, onListRuns, onReadChecks, lines, jobLabel, floor } = props;
  const [reported, setReported] = useState<readonly ManifestCheckRow[]>([]);
  const [runs, setRuns] = useState<readonly CheckoutRunRecord[]>([]);
  const [openId, setOpenId] = useState<string | null>(null);
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
    void onReadChecks().then((read) => {
      if (current && read.ok) setReported(read.checks.rows);
    });
    return () => {
      current = false;
    };
  }, [onListRuns, onReadChecks, runningId, verifyId, verifyEnded]);

  const entries = useMemo(() => checkEntriesOf(data, runs, lines, reported), [data, runs, lines, reported]);
  const open = entries.find((one) => one.id === openId);
  const bands = open === undefined ? undefined : <FactsOf entry={open} {...props} />;

  return (
    <div className="armada-screen__overview">
      <CheckList rows={entries.map((one) => checkRowOf(one, jobLabel))} openRow={open?.id ?? null} onOpenRow={setOpenId} />
      {open === undefined ? null : open.land !== undefined ? (
        <LandCheckLogSheet
          key={open.id}
          at={open.land}
          followed={props.landFollowed}
          onFollow={props.onFollowLand}
          bands={bands}
          floor={floor}
          onClose={() => setOpenId(null)}
        />
      ) : open.job !== undefined && open.logs !== undefined ? (
        <ReportedLogPanel key={open.id} entry={open} job={open.job.id} logs={open.logs} {...props} onClose={() => setOpenId(null)} />
      ) : (
        <CheckLogPanel key={open.id} entry={open} bands={bands} {...props} onClose={() => setOpenId(null)} />
      )}
    </div>
  );
}

/**
 * A gate's or a Drone's Check: its log read under its Job, followed while an asked run goes. A run
 * of several Checks has a log each, and a tab picks which is read.
 */
function ReportedLogPanel(
  props: ManifestChecksProps & { entry: CheckEntry; job: string; logs: NonNullable<CheckEntry["logs"]>; onClose: () => void },
) {
  const { entry, job, logs, floor, onClose } = props;
  const [at, setAt] = useState(0);
  const outputs = useCheckOutputs(props.onReadCheckOutput, job);
  const following = useFollowing(props.onFollowCheckOutput, props.followedLog, job);
  const log = logs[Math.min(at, logs.length - 1)]!;
  return (
    <JobCheckLogSheet
      log={{ name: log.check, kept: log.kept, live: entry.status === "running" }}
      outputs={outputs}
      following={following}
      bands={
        <>
          {logs.length < 2 ? null : (
            <Tabs items={logs.map((one, n) => ({ id: String(n), label: one.check }))} value={String(at)} onChange={(id) => setAt(Number(id))} />
          )}
          <FactsOf {...props} />
        </>
      }
      floor={floor}
      onClose={onClose}
    />
  );
}

/** The Check's facts, with who asked as a link where a route to them exists. */
function FactsOf({ entry, jobLabel, onOpenJob, onOpenMergeLine }: ManifestChecksProps & { entry: CheckEntry }) {
  const asker = askerOf(entry.requester, jobLabel);
  return <CheckDetails details={checkDetailsOf(entry, <RequestedBy asker={asker} onOpenJob={onOpenJob} onOpenMergeLine={onOpenMergeLine} />)} />;
}

/** Who asked, pressable where there is a requester to open. */
function RequestedBy({ asker, onOpenJob, onOpenMergeLine }: { asker: Asker; onOpenJob: ManifestChecksProps["onOpenJob"]; onOpenMergeLine: ManifestChecksProps["onOpenMergeLine"] }): ReactNode {
  const { opens, label } = asker;
  if (opens === undefined) return label;
  return (
    <Button
      variant="ghost"
      size="sm"
      onClick={() =>
        opens.to === "merge-line"
          ? onOpenMergeLine(opens.branch)
          : onOpenJob(opens.jobId, opens.step === undefined && opens.drone === undefined ? undefined : { ...(opens.step === undefined ? {} : { step: opens.step }), ...(opens.drone === undefined ? {} : { drone: opens.drone }) })
      }
    >
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
      wrap={{ wrap, onToggle: () => setWrap((was) => !was) }}
      bands={bands}
      floor={floor}
      onClose={onClose}
    >
      <ConsoleOutput rows={rows} following={live} wrap={wrap} />
    </LogSheet>
  );
}
