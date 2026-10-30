// What End lists first on a Job that still holds a ticket, a pull request or a deploy not in an end state, each as `Deploy: Rolling out`; one not read yet is its label alone. Ending a Job ends it and its instances, and
// nothing here cancels anything: the list is the states as they stand. End job goes on as it does with nothing in flight, and Keep going leaves the Job as it is.

import { Dialog } from "@armada/components";

import "./lanes.css";

export type InFlightRow = { id: string; label: string; state: string };

export function InFlightDialog({ rows, onEnd, onKeep }: { rows: readonly InFlightRow[]; onEnd: () => void; onKeep: () => void }) {
  return (
    <Dialog open tone="destructive" title="In flight" confirmLabel="End job" cancelLabel="Keep going" onConfirm={onEnd} onCancel={onKeep}>
      <ul className="sv-inflight" aria-label="In flight">
        {rows.map((one) => (
          <li key={one.id} className="sv-inflight__row">{one.state === "" ? one.label : `${one.label}: ${one.state}`}</li>
        ))}
      </ul>
    </Dialog>
  );
}
