// A restart Fleet reports as failed raises the failure toast once, with the Dispatch session act every failure toast has.
// Fleet serves no event for it (`get_fleet_build` is read on a connection and every minute), so the toast hangs off the
// report. The owner's ask of 17 Oct 2026.
//
// **One toast per failure, keyed by commit and line.** The same report read again is not news, and neither is one
// Bridge was reopened on after its toast was raised: the last key raised is kept in `localStorage`. A failure present
// when Bridge starts that was never raised is news, and raises. A report with no `failed` forgets the key, so a retry that
// fails the same way is a new failure.

import { useEffect, useRef } from "react";
import type { BuildSource, FleetBuildReport } from "@armada/protocol";
import type { BuildFailed } from "@armada/shell";

const KEY = "armada.bridge.build-failure-raised";

function stored(): string | null {
  try {
    return window.localStorage.getItem(KEY);
  } catch {
    return null;
  }
}

function store(key: string | null): void {
  try {
    if (key === null) window.localStorage.removeItem(KEY);
    else window.localStorage.setItem(KEY, key);
  } catch {
    // Unremembered, so a reopened Bridge raises it once more.
  }
}

/** Raises a toast for a failed restart the report newly carries. */
export function useBuildFailed(report: FleetBuildReport | null, raise: (failed: BuildFailed) => void): void {
  /** The build the last restart under way was moving onto: by the time it fails, the report no longer says. */
  const target = useRef<BuildSource | undefined>(undefined);
  useEffect(() => {
    // Fleet offering no build, or not answering, says nothing about the last failure.
    if (report === null) return;
    if (report.restarting !== undefined) target.current = report.restarting;
    if (report.failed === undefined) {
      // Nothing failed, so the next failure is not this restart's.
      if (report.restarting === undefined) target.current = undefined;
      store(null);
      return;
    }
    const key = `${report.commit ?? ""} ${report.failed}`;
    if (stored() === key) return;
    store(key);
    raise({
      ...(target.current === undefined ? {} : { build: target.current }),
      running: report.on,
      ...(report.commit === undefined ? {} : { commit: report.commit }),
      line: report.failed,
    });
  }, [report, raise]);
}
