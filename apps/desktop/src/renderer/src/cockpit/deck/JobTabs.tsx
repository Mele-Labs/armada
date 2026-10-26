// The tab row at the top of a Job's thread in the `tabs` take (`flight-take.ts`): Thread, then the layers the Job has something in (Artifacts, Checks, Subagents, Bays).
// A tab is a view the layers already provide (`session-view/layers.ts`), so the selected one follows `useLayer` and Thread is no layer. A layer with nothing in it
// is not drawn, for the reason its tile gives off (`offeredOf`). Each tab wears its tile's key, which presses it as the tile did (`flight-keys.ts`). The
// facts the tiles carried stay on the tab: Checks pulses while any runs, Subagents draws the number running, Bays is lit when a bay is wrong and carries the clash.
// Not the `Tabs` primitive, which draws a label and a dot and nothing else. The owner's ask of 25 Oct 2026.

import { useRef, type KeyboardEvent } from "react";
import { CircleDot, Swords } from "lucide-react";
import { Kbd, Tooltip } from "@armada/components";
import { attachmentsOf, type Session } from "@armada/screens/src/draft/sessions";

import { useSessionsDraft } from "../../sessions-draft";
import { closeLayer, showLayer, toggleLayer, useLayer, type LayerId } from "../../session-view/layers";
import { baysLit, type SessionBays } from "../../session-view/slot-trouble";
import { tilesOf, useFlightFile } from "./flight-controls";
import { TAB_LAYERS } from "./flight-take";
import { offeredOf, tileCap } from "./session-tiles";
import "./JobTabs.css";

const THREAD = "Thread";

export function JobTabs({ session, bays }: { session: Session; bays: SessionBays | undefined }) {
  const open = useLayer(session.id);
  const draft = useSessionsDraft();
  const tiles = tilesOf("job", useFlightFile());
  const strip = useRef<HTMLDivElement>(null);
  if (draft === undefined) return null;
  const { off } = offeredOf(session, draft, undefined, (bays?.bays.length ?? 0) > 0);
  const drawn = TAB_LAYERS.filter((layer) => off[layer] === undefined);
  const subagentsRunning = attachmentsOf(session, "subagent").filter((one) => one.state === "running").length;
  const checksRunning = session.rows.some((one) => one.kind === "check" && one.state === "running");
  const clashing = (bays?.clashes ?? []).map((one) => (one.one.kind === "session" && one.one.id === session.id ? one.other : one.one).label);
  const clashSaid = clashing.length === 0 ? undefined : `Write clash with ${clashing.join(", ")}`;
  const wrongs = [...(bays?.bays ?? []).flatMap((one) => (one.said === undefined ? [] : [one.said])), ...(clashSaid === undefined ? [] : [clashSaid])];
  const go = (layer: LayerId | undefined) => (layer === undefined ? closeLayer() : toggleLayer(session.id, layer));
  // The arrows move between tabs and select as they go; a layer takes focus when it opens, so the tab takes it back.
  const move = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key !== "ArrowRight" && event.key !== "ArrowLeft") return;
    const order: (LayerId | undefined)[] = [undefined, ...drawn];
    const at = Math.max(0, order.indexOf(open));
    const to = (at + (event.key === "ArrowRight" ? 1 : -1) + order.length) % order.length;
    event.preventDefault();
    if (order[to] === undefined) closeLayer();
    else showLayer(session.id, order[to]);
    window.setTimeout(() => strip.current?.querySelectorAll<HTMLElement>('[role="tab"]')[to]?.focus(), 0);
  };
  const tab = (layer: LayerId | undefined) => {
    const tile = layer === undefined ? undefined : tiles.find((one) => one.builtIn !== undefined && one.id === layer);
    const on = open === layer;
    const key = tile?.key;
    const lit = layer === "slot" && bays !== undefined && baysLit(bays);
    const pulsing = layer === "checks" && checksRunning;
    const button = (
      <button
        key={layer ?? "thread"}
        type="button"
        role="tab"
        aria-selected={on}
        tabIndex={on ? 0 : -1}
        className={on ? "armada-tabs__tab armada-tabs__tab--active sv-tab" : "armada-tabs__tab sv-tab"}
        data-key={key}
        data-lit={lit || undefined}
        data-running={pulsing || undefined}
        onClick={() => go(layer)}
      >
        {layer === undefined ? THREAD : (tile?.text ?? layer)}
        {layer !== "subagents" || subagentsRunning === 0 ? null : (
          // The number running: the owner asked for it by name, the one place a figure is drawn beside what it counts.
          <span className="sv-tab__count" role="img" aria-label={`${subagentsRunning} subagents running`}>
            {subagentsRunning}
          </span>
        )}
        {!pulsing ? null : (
          <span className="sv-tab__running" role="img" aria-label="Running">
            <CircleDot size={12} strokeWidth={2} aria-hidden="true" />
          </span>
        )}
        {layer !== "slot" || clashSaid === undefined ? null : (
          <span className="sv-tab__clash" role="img" aria-label={clashSaid}>
            <Swords size={12} strokeWidth={2} aria-hidden="true" />
          </span>
        )}
        {key === undefined ? null : (
          <Kbd className="sv-tab__key" aria-hidden="true">
            {tileCap(key)}
          </Kbd>
        )}
      </button>
    );
    // What is wrong with a bay, or the clash, is the Bays tab's tooltip, as it was the tile's.
    return layer === "slot" && wrongs.length > 0 ? (
      <Tooltip key="slot" label={wrongs.join(", ")} asChild>
        {button}
      </Tooltip>
    ) : (
      button
    );
  };
  return (
    <div className="sv-tabs">
      <div ref={strip} className="armada-tabs" role="tablist" onKeyDown={move}>
        {[undefined, ...drawn].map(tab)}
      </div>
    </div>
  );
}
