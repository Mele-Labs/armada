// The Job's views as icon buttons in the composer's row, on `&views=composer` of the `tabs` take (`flight-take.ts`): no tab row, Thread is the thread itself, and Artifacts, Checks,
// Subagents and Bays are the layers the tabs were (`session-view/layers.ts`). A press opens the layer, a press on the lit one shuts it, as Esc does. A view with nothing in it stays in its
// place, off, with its reason as the tooltip (`offeredOf`), so the buttons never move. The facts the tiles carried stay on the button's corner: Subagents the number running, Checks a pulsing
// dot while any runs, Bays the clash, and Bays tinted while a bay is wrong. **A layer starts under the head and stops over the composer's dock** (`LayerBounds`), so the composer and its
// buttons stay on show while a layer is read. The keys press the buttons as they pressed the tabs (`flight-keys.ts`). The owner's ask of 25 Oct 2026.

import { useLayoutEffect, useRef } from "react";
import { CircleDot, Swords } from "lucide-react";
import { type ComposerView } from "@armada/components";
import { attachmentsOf, type Session } from "@armada/screens/src/draft/sessions";

import { useSessionsDraft } from "../../sessions-draft";
import { toggleLayer, useLayer } from "../../session-view/layers";
import { baysLit, type SessionBays } from "../../session-view/slot-trouble";
import { tilesOf, useFlightFile } from "./flight-controls";
import { glyphOf } from "./flight-glyphs";
import { TAB_LAYERS, takeKey, viewsInComposer } from "./flight-take";
import { offeredOf } from "./session-tiles";
import "./JobTabs.css";

const GLYPH: Record<(typeof TAB_LAYERS)[number], string> = { artifacts: "files", checks: "shield-check", subagents: "split", slot: "rows-3" };

/** The buttons for the composer's row, or none where the page does not ask for them. */
export function useJobViews(session: Session, bays: SessionBays | undefined): readonly ComposerView[] | undefined {
  const open = useLayer(session.id);
  const draft = useSessionsDraft();
  const tiles = tilesOf("job", useFlightFile());
  if (!viewsInComposer() || draft === undefined) return undefined;
  const { off } = offeredOf(session, draft, undefined, (bays?.bays.length ?? 0) > 0);
  const subagentsRunning = attachmentsOf(session, "subagent").filter((one) => one.state === "running").length;
  const checksRunning = session.rows.some((one) => one.kind === "check" && one.state === "running");
  const clashing = (bays?.clashes ?? []).map((one) => (one.one.kind === "session" && one.one.id === session.id ? one.other : one.one).label);
  const clashSaid = clashing.length === 0 ? undefined : `Write clash with ${clashing.join(", ")}`;
  const wrongs = [...(bays?.bays ?? []).flatMap((one) => (one.said === undefined ? [] : [one.said])), ...(clashSaid === undefined ? [] : [clashSaid])];
  return TAB_LAYERS.map((layer): ComposerView => {
    const tile = tiles.find((one) => one.builtIn !== undefined && one.id === layer);
    const key = tile === undefined ? undefined : takeKey("tabs", tile);
    const why = off[layer];
    const name = tile?.text ?? layer;
    const mark =
      layer === "subagents" && subagentsRunning > 0 ? (
        // The number running: the owner asked for it by name, the one place a figure is drawn beside what it counts.
        <span className="sv-tab__count" role="img" aria-label={`${subagentsRunning} subagents running`}>
          {subagentsRunning}
        </span>
      ) : layer === "checks" && checksRunning ? (
        <span className="sv-tab__running" role="img" aria-label="Running">
          <CircleDot size={12} strokeWidth={2} aria-hidden="true" />
        </span>
      ) : layer === "slot" && clashSaid !== undefined ? (
        <span className="sv-tab__clash" role="img" aria-label={clashSaid}>
          <Swords size={12} strokeWidth={2} aria-hidden="true" />
        </span>
      ) : undefined;
    return {
      id: layer,
      label: name,
      tip: why ?? (layer === "slot" && wrongs.length > 0 ? wrongs.join(", ") : (tile?.label ?? name)),
      Glyph: glyphOf(GLYPH[layer], "zap"),
      ...(key === undefined ? {} : { key }),
      pressed: open === layer,
      ...(why === undefined ? {} : { disabled: true }),
      ...(layer === "slot" && bays !== undefined && baysLit(bays) ? { lit: true } : {}),
      ...(mark === undefined ? {} : { mark }),
      onPress: () => toggleLayer(session.id, layer),
    };
  });
}

/** Drawn in the composer's dock. Sets where a layer starts and stops on the pane: under the head, and over the dock. */
export function LayerBounds() {
  const mark = useRef<HTMLSpanElement>(null);
  useLayoutEffect(() => {
    const dock = mark.current?.closest<HTMLElement>(".sv-dock");
    const pane = dock?.closest<HTMLElement>(".deck-pane");
    const head = pane?.querySelector<HTMLElement>(".armada-session-frame__head") ?? null;
    if (dock == null || pane == null || head == null) return;
    const place = () => {
      const edge = pane.getBoundingClientRect().top + pane.clientTop;
      pane.style.setProperty("--sv-tabs-bottom", `${head.getBoundingClientRect().bottom - edge}px`);
      pane.style.setProperty("--sv-tabs-foot", `${edge + pane.clientHeight - dock.getBoundingClientRect().top}px`);
    };
    place();
    const watch = new ResizeObserver(place);
    watch.observe(head);
    watch.observe(dock);
    watch.observe(pane);
    return () => {
      watch.disconnect();
      pane.style.removeProperty("--sv-tabs-bottom");
      pane.style.removeProperty("--sv-tabs-foot");
    };
  }, []);
  return <span ref={mark} hidden />;
}
