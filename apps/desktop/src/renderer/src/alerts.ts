// The open Job's own alerts, for the lead of its Overview: `list_alerts`, read again whenever
// what the Board's bells draw moves — a row's alert or status, or a firing on the Job open.

import { useEffect, useState } from "react";

import { alertRowsOf, type AlertRow, type JobOpening } from "@armada/jobs";
import type { BridgeState } from "../../shared/bridge";
import { readAlerts } from "./commands";

export function useAlerts(state: Pick<BridgeState, "jobs" | "watched">, onOpenAlert: (jobId: string, to: JobOpening) => void): { alerts: AlertRow[]; onOpenAlert: typeof onOpenAlert } {
  const [rows, setRows] = useState<AlertRow[]>([]);
  const detail = state.watched.state === "read" ? state.watched.detail : undefined;
  const openId = detail?.job.id;
  const key = JSON.stringify([
    state.jobs.map((job) => [job.id, job.status, job.alert?.kind, job.alert?.trigger]),
    detail?.job.id,
    (detail?.triggers ?? []).map((one) => [one.name, one.state]),
    (detail?.additions ?? []).map((one) => [one.id, one.state]),
  ]);
  useEffect(() => {
    let alive = true;
    void readAlerts().then((answer) => {
      if (alive) setRows(answer.ok ? alertRowsOf(answer, state.jobs).filter((row) => row.job === openId) : []);
    });
    return () => {
      alive = false;
    };
    // `key` is what `state.jobs` was read for.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);
  return { alerts: rows, onOpenAlert };
}
