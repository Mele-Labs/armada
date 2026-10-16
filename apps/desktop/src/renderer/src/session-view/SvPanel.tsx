// One panel of a Session's view: a header band over a mono body, the state carried in the frame as the pool's bays are.
// `asks` is what the person is owed, `live` is work going on, `done` is work finished, and none is a panel at rest.
// `place` is where it stands (the Cockpit's left column, its right column, its foot), which decides how its body is laid out.

import type { ReactNode } from "react";

import { useCollapse } from "../cockpit/deck/collapse";

export type PanelState = "asks" | "live" | "done";
export type Place = "left" | "side" | "foot" | "thread" | "calls";

export function SvPanel({ label, state, acts, place, area, children }: { label: string; state?: PanelState | undefined; acts?: ReactNode; place: Place; area: string; children?: ReactNode }) {
  // Pressing the band folds the panel to it. The card in the thread is not one of the panels that fold.
  const { collapsed, band } = useCollapse(area);
  const folds = place !== "thread";
  return (
    <section className="sv-panel" data-area={area} data-place={place} data-state={state} data-collapsed={(folds && collapsed) || undefined} aria-label={label}>
      <header className="sv-panel__band" {...(folds ? band : {})}>
        <h3 className="sv-panel__label">{label}</h3>
        {acts === undefined ? null : <span className="sv-panel__acts">{acts}</span>}
      </header>
      <div className="sv-panel__body">{children}</div>
    </section>
  );
}
