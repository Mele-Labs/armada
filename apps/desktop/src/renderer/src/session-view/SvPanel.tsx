// One panel of a Session's view: a header band over a mono body, the state carried in the frame as the pool's bays are.
// `asks` is what the person is owed, `live` is work going on, `done` is work finished, and none is a panel at rest.
// `place` is where it stands (the Cockpit's left column, its right column, its foot), which decides how its body is laid out.

import type { ReactNode } from "react";

export type PanelState = "asks" | "live" | "done";
export type Place = "left" | "side" | "foot" | "thread" | "calls";

export function SvPanel({ label, state, acts, place, area, children }: { label: string; state?: PanelState | undefined; acts?: ReactNode; place: Place; area: string; children?: ReactNode }) {
  return (
    <section className="sv-panel" data-area={area} data-place={place} data-state={state} aria-label={label}>
      <header className="sv-panel__band">
        <h3 className="sv-panel__label">{label}</h3>
        {acts === undefined ? null : <span className="sv-panel__acts">{acts}</span>}
      </header>
      <div className="sv-panel__body">{children}</div>
    </section>
  );
}
