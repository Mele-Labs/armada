// Under the Bays metro while an Autopilot is on: Next up, a numbered list of what it is choosing from
// (the owner's call: a list, never tags), and Set aside, a siding of each pick it was unsure of with
// its reason, Run it and Dismiss. #2092.

import { Button } from "@armada/components";
import type { AutopilotPick } from "@armada/protocol";

import type { NextUp } from "./autopilot";

export type SidingPress = { pickId: string; act: "run" | "dismiss" };

export type Siding = {
  manifest: string;
  /** Named where there is more than one: on All. */
  name: string | undefined;
  nextUp: readonly NextUp[];
  asides: readonly AutopilotPick[];
};

/** Run it and Dismiss on one set-aside pick. Run it goes to the approval gate, as any dispatch does. */
export function AsideActs({ pick, pending, onRun, onDismiss }: { pick: AutopilotPick; pending: SidingPress | undefined; onRun: () => void; onDismiss: () => void }) {
  const mine = pending?.pickId === pick.pick_id ? pending.act : undefined;
  return (
    <span className="armada-autopilot-aside__acts">
      <Button variant="secondary" size="sm" onClick={onRun} pending={mine === "run"} disabled={pending !== undefined && mine !== "run"}>
        Run it
      </Button>
      <Button variant="ghost" size="sm" onClick={onDismiss} pending={mine === "dismiss"} disabled={pending !== undefined && mine !== "dismiss"}>
        Dismiss
      </Button>
    </span>
  );
}

export function AutopilotSidings({ sidings, pending, onRun, onDismiss }: {
  sidings: readonly Siding[];
  pending: SidingPress | undefined;
  onRun: (manifest: string, pick: AutopilotPick) => void;
  onDismiss: (manifest: string, pick: AutopilotPick) => void;
}) {
  return (
    <>
      {sidings.map((siding) => (
        <div key={siding.manifest} className="armada-autopilot-sidings">
          {siding.nextUp.length === 0 ? null : (
            <section className="armada-autopilot-next" aria-label={siding.name === undefined ? "Next up" : `Next up in ${siding.name}`}>
              <h3 className="armada-metro__part">Next up{siding.name === undefined ? "" : ` · ${siding.name}`} · what Autopilot is choosing from</h3>
              <ol className="armada-autopilot-next__list">
                {siding.nextUp.map((one) => (
                  <li key={one.number} className="armada-autopilot-next__row" data-passed={one.order === undefined || undefined}>
                    <span className="armada-autopilot-next__order">{one.order ?? "–"}</span>
                    <span className="armada-autopilot-next__number">#{one.number}</span>
                    <span className="armada-autopilot-next__title" title={one.title}>
                      {one.title}
                    </span>
                    <span className="armada-autopilot-next__note">{one.note}</span>
                  </li>
                ))}
              </ol>
            </section>
          )}
          <section className="armada-autopilot-aside" aria-label={siding.name === undefined ? "Set aside" : `Set aside in ${siding.name}`}>
            <h3 className="armada-metro__part">Set aside{siding.name === undefined ? "" : ` · ${siding.name}`} · Autopilot was unsure</h3>
            {siding.asides.length === 0 ? (
              <p className="armada-autopilot-aside__empty">Nothing set aside.</p>
            ) : (
              <ul className="armada-autopilot-aside__list">
                {siding.asides.map((pick) => (
                  <li key={pick.pick_id} className="armada-autopilot-aside__row">
                    <span className="armada-autopilot-aside__number">#{pick.issue_number}</span>
                    <span className="armada-autopilot-aside__title">{pick.issue_title}</span>
                    <span className="armada-autopilot-aside__why">{pick.reason}</span>
                    <AsideActs pick={pick} pending={pending} onRun={() => onRun(siding.manifest, pick)} onDismiss={() => onDismiss(siding.manifest, pick)} />
                  </li>
                ))}
              </ul>
            )}
          </section>
        </div>
      ))}
    </>
  );
}
