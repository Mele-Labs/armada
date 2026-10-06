// The Checks page: every Check the selected Manifest has had requested or run, each with its state,
// and one opened in the log panel with its facts over its log (owner, 6 Oct 2026). Read off what
// the Manifest surface holds, the checkout's run sheet and run list.
//
// **One panel for the log, `LogSheet`'s.** A Check still out is followed as it prints, which is the
// socket the Manifest surface opens; one that has ended is read whole off `get_checkout_run_output`.
// **Nothing stands in for lines that have not arrived**, and no count is drawn over the rows.

import { useEffect, useMemo, useState } from "react";

import { CheckDetails, CheckList, ConsoleOutput, type ConsoleRow } from "@armada/components";
import type { CheckoutRunFollowed, CheckoutRunListRead, CheckoutRunRecord, CheckoutRunSheetRead, RunOutputRead } from "@armada/protocol";

import { checkDetailsOf, checkEntriesOf, checkRowOf, type CheckEntry } from "./manifest-checks";
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
  /** The window is at `--window-floor`. */
  floor: boolean;
};

export function ManifestChecks({ sheet, followed, onObserveRun, onListRuns, onGetRunOutput, floor }: ManifestChecksProps) {
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
    return () => {
      current = false;
    };
  }, [onListRuns, runningId, verifyId, verifyEnded]);

  const entries = useMemo(() => checkEntriesOf(data, runs), [data, runs]);
  const open = entries.find((one) => one.id === openId);

  return (
    <div className="armada-screen__overview">
      <CheckList rows={entries.map(checkRowOf)} openRow={open?.id ?? null} onOpenRow={setOpenId} />
      {open === undefined ? null : (
        <CheckLogPanel
          key={open.id}
          entry={open}
          followed={followed}
          onObserveRun={onObserveRun}
          onGetRunOutput={onGetRunOutput}
          floor={floor}
          onClose={() => setOpenId(null)}
        />
      )}
    </div>
  );
}

/** The open Check: its facts as a band, and its log under them, live while it is out. */
function CheckLogPanel({
  entry,
  followed,
  onObserveRun,
  onGetRunOutput,
  floor,
  onClose,
}: {
  entry: CheckEntry;
  followed: CheckoutRunFollowed;
  onObserveRun: (runId: string | null) => void;
  onGetRunOutput: (runId: string) => Promise<RunOutputRead>;
  floor: boolean;
  onClose: () => void;
}) {
  const { runId, status } = entry;
  const running = status === "running";
  const [kept, setKept] = useState<ConsoleRow[]>([]);
  const [wrap, setWrap] = useState(false);

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
      setKept(read.output.lines.map((text, at) => ({ row: "line" as const, at: read.output.from_line + at, text })));
    });
    return () => {
      current = false;
    };
  }, [runId, running, onObserveRun, onGetRunOutput]);

  const ours = followed.state === "following" && followed.runId === runId ? followed : undefined;
  const rows: ConsoleRow[] = running
    ? (ours?.lines ?? []).map((text, at) => ({ row: "line" as const, at: ours!.fromLine + at, text }))
    : runId === undefined
      ? []
      : kept;
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
      bands={<CheckDetails details={checkDetailsOf(entry)} />}
      floor={floor}
      onClose={onClose}
    >
      <ConsoleOutput rows={rows} following={live} wrap={wrap} />
    </LogSheet>
  );
}
