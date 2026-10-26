// The Job's views as tabs in the `tabs` take (`flight-take.ts`), in one of two looks on `&tabs=`. `nav` is a second row of the head, under the title, or with `&tabpos=foot` the last row of the panel, under
// the composer, in the type and ground of the Cockpit's own places in the title bar. `segmented` is a segmented control in the head's own row, before the checks and the context tank, so the head
// stays one row. Thread comes first, then the layers the Job has something in (Artifacts, Checks, Subagents, Bays). A tab is a view the layers already provide
// (`session-view/layers.ts`), so the selected one follows `useLayer` and Thread is no layer. A layer with nothing in it is not drawn, for the reason its tile gives off
// (`offeredOf`). A tab draws no key cap, which reads as a count beside a word; its key is in its tooltip, and presses it as the tile did (`flight-keys.ts`). The facts
// the tiles carried stay on the tab: Checks pulses while any runs, Subagents draws the number running, Bays is lit when a bay is wrong and carries the clash.
// **A layer starts under the head**, and under the second row where there is one, and stops over the tabs where they are the panel's footer, so the Job's title and
// the tabs stay on show while it is read (`--sv-tabs-bottom` and `--sv-tabs-foot`, set here on the pane). Not the `Tabs` primitive, which draws a label and a dot and nothing else. The owner's ask of 25 Oct 2026.

import { useLayoutEffect, useRef, type KeyboardEvent } from "react";
import { CircleDot, Swords } from "lucide-react";
import { Tooltip } from "@armada/components";
import { attachmentsOf, type Session } from "@armada/screens/src/draft/sessions";

import { useSessionsDraft } from "../../sessions-draft";
import { closeLayer, showLayer, toggleLayer, useLayer, type LayerId } from "../../session-view/layers";
import { baysLit, type SessionBays } from "../../session-view/slot-trouble";
import { tilesOf, useFlightFile } from "./flight-controls";
import { TAB_LAYERS, tabsLook, tabsPos, takeKey } from "./flight-take";
import { offeredOf } from "./session-tiles";
import "./JobTabs.css";

const THREAD = "Thread";

export function JobTabs({ session, bays }: { session: Session; bays: SessionBays | undefined }) {
  const open = useLayer(session.id);
  const draft = useSessionsDraft();
  const tiles = tilesOf("job", useFlightFile());
  const strip = useRef<HTMLDivElement>(null);
  const row = useRef<HTMLDivElement>(null);
  const look = tabsLook();
  const pos = tabsPos();
  // The pane's layer starts where the head ends, or under this row where it is the second row of the head, and stops over this row where it is the panel's last.
  useLayoutEffect(() => {
    const el = row.current;
    const pane = el?.closest<HTMLElement>(".deck-pane");
    const head = pane?.querySelector<HTMLElement>(".armada-session-frame__head") ?? null;
    const top = look === "nav" && pos === "head" ? el : head;
    if (el == null || pane == null || top == null) return;
    const place = () => {
      const edge = pane.getBoundingClientRect().top + pane.clientTop;
      pane.style.setProperty("--sv-tabs-bottom", `${top.getBoundingClientRect().bottom - edge}px`);
      if (pos === "foot") pane.style.setProperty("--sv-tabs-foot", `${edge + pane.clientHeight - el.getBoundingClientRect().top}px`);
    };
    place();
    const watch = new ResizeObserver(place);
    watch.observe(top);
    watch.observe(el);
    watch.observe(pane);
    return () => {
      watch.disconnect();
      pane.style.removeProperty("--sv-tabs-bottom");
      pane.style.removeProperty("--sv-tabs-foot");
    };
  }, [draft === undefined]);
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
    const key = tile === undefined ? undefined : takeKey("tabs", tile);
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
      </button>
    );
    // A tab is named and keyed in its tooltip; what is wrong with a bay, or the clash, is the Bays tab's, as it was the tile's.
    return layer === undefined ? (
      button
    ) : (
      <Tooltip key={layer} label={layer === "slot" && wrongs.length > 0 ? wrongs.join(", ") : (tile?.label ?? layer)} {...(key === undefined ? {} : { shortcut: key })} asChild>
        {button}
      </Tooltip>
    );
  };
  return (
    <div ref={row} className="sv-tabs" data-look={look} data-pos={pos}>
      <div ref={strip} className="sv-tabs__list" role="tablist" onKeyDown={move}>
        {[undefined, ...drawn].map(tab)}
      </div>
    </div>
  );
}
