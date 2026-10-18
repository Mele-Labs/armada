// The place the battlefield is drawn: the map, which is the stage when nothing is picked. It carries the picked event and the
// replay, so a dot of Activity and a replay say the same thing wherever they are drawn. #2301.

import { Orbit } from "lucide-react";

import type { CockpitProps } from "./props";
import { Battlefield } from "./Battlefield";
import { FocusChip, ReplayBar, ReplayMark } from "./ReplayBar";
import { moveCursor, select, selOfPr, selOfSubject, startReplay, stopReplay, useDeck } from "./store";
import { useField } from "./useField";
import type { Ship, Subject } from "./ship";
import { openPullRequestInPanel, openSlotKeyInPanel } from "../../shared-panel/store";

/** The picked event's caption, the replay's marker and its bar: over the field, where it is drawn. */
function Overlays({ props, field }: { props: CockpitProps; field: ReturnType<typeof useField> }) {
  const { replay, picked, ordered } = field;
  // A replay can start only where the events Bridge holds reach: from an event in the window, and never earlier.
  const reaches = picked !== undefined && ordered.some((one) => one.id === picked.id);
  return (
    <>
      {replay === undefined ? null : <ReplayMark />}
      {replay !== undefined || picked === undefined ? null : <FocusChip event={picked} now={props.now} onReplay={reaches ? () => startReplay(Date.parse(picked.at)) : undefined} />}
      {replay === undefined ? null : <ReplayBar events={ordered} replay={replay} onLive={stopReplay} />}
    </>
  );
}

export function MapView({ props, ship, listed }: { props: CockpitProps; ship: Ship; listed: readonly Subject[] }) {
  const cursor = useDeck((deck) => deck.cursor);
  const field = useField(props, ship, listed, props.picked?.root);
  const { replay, calls, repos, drones, pads, goals, crates, flightTrails, drawn, facts, clashes, focusKey, builds, outposts } = field;
  return (
    <section className="deck-pane" aria-label="Flight map">
      <header className="deck-band deck-pane__head">
        <Orbit size={16} aria-hidden="true" />
        <h2 className="deck-pane__title">Map</h2>
      </header>
      <div className="deck-map">
        <Overlays props={props} field={field} />
        <Battlefield
          repos={repos}
          drones={drones}
          pads={pads}
          goals={goals}
          prs={crates}
          trails={flightTrails}
          plays={drawn}
          facts={facts}
          calls={calls}
          clashes={clashes}
          focus={replay === undefined ? focusKey : undefined}
          builds={builds}
          outposts={outposts}
          selected={cursor}
          scope={props.picked?.root}
          onSelect={(key) => {
            const subject = ship.subjects.find((one) => one.key === key);
            if (subject !== undefined) select(selOfSubject(subject));
          }}
          onClear={() => moveCursor(undefined)}
          onPad={(key) => openSlotKeyInPanel(key) || select({ kind: "slot", key })}
          onGoal={(root) => {
            const pr = ship.prs.find((one) => one.root === root && one.stage !== "landed") ?? ship.prs.find((one) => one.root === root);
            if (pr !== undefined && !openPullRequestInPanel(pr)) select(selOfPr(pr));
          }}
          onPr={(key) => {
            const pr = ship.prs.find((one) => one.key === key);
            if (pr !== undefined && !openPullRequestInPanel(pr)) select(selOfPr(pr));
          }}
        />
      </div>
    </section>
  );
}
