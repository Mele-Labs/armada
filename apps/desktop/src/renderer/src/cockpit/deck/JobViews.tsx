// The Job's views on `&views=` of the `tabs` take (`flight-take.ts`), and no tab row in any of them. Thread is the thread itself; Artifacts, Checks, Subagents and Bays are the layers the tabs were
// (`session-view/layers.ts`). `composer` draws four icon buttons in the message box's row (`SessionComposer`'s `views`), `head` four in the Job's title row before its own buttons (`HeadViews`),
// and `left` a panel under the Jobs list with a row for each (`LeftViews`, set in `ViewsSlot`). A press opens the layer, a press on the lit one shuts it, as Esc does. A view with nothing in it stays
// in its place, off, with its reason as the tooltip (`offeredOf`), so nothing moves. The facts the tiles carried stay with each: Subagents the number running, Checks a pulsing dot while any runs,
// Bays the clash, and Bays tinted while a bay is wrong. **A layer starts under the head and stops over the composer's dock** (`LayerBounds`), so the head, the composer and the views stay on
// show while a layer is read. The keys press them as they pressed the tabs (`flight-keys.ts`). The owner's ask of 25 Oct 2026.

import { useLayoutEffect, useRef, useSyncExternalStore } from "react";
import { createPortal } from "react-dom";
import { Swords } from "lucide-react";
import { Button, Kbd, Separator, Tooltip, type ComposerView } from "@armada/components";
import { attachmentsOf, type Session } from "@armada/screens/src/draft/sessions";

import { useSessionsDraft } from "../../sessions-draft";
import { toggleLayer, useLayer } from "../../session-view/layers";
import { baysLit, type SessionBays } from "../../session-view/slot-trouble";
import { tilesOf, useFlightFile } from "./flight-controls";
import { glyphOf } from "./flight-glyphs";
import { useCollapse } from "./collapse";
import { TAB_LAYERS, takeKey, viewsAt } from "./flight-take";
import { VIEWS_KEY } from "./keys-model";
import { offeredOf } from "./session-tiles";
import "./JobTabs.css";

const GLYPH: Record<(typeof TAB_LAYERS)[number], string> = { artifacts: "library", checks: "shield-check", subagents: "drone", slot: "rows-3" };

/** The Job's four views, or none where the page does not ask for them. */
export function useJobViews(session: Session, bays: SessionBays | undefined): readonly ComposerView[] | undefined {
  const open = useLayer(session.id);
  const draft = useSessionsDraft();
  const tiles = tilesOf("job", useFlightFile());
  if (viewsAt() === undefined || draft === undefined) return undefined;
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
    // Running reads on the glyph itself, in the running hue and pulsing (`JobTabs.css`), and the number running is in the tooltip. Only a clash is a mark.
    const running = (layer === "checks" && checksRunning) || (layer === "subagents" && subagentsRunning > 0);
    const mark =
      layer === "slot" && clashSaid !== undefined ? (
        <span className="sv-tab__clash" role="img" aria-label={clashSaid}>
          <Swords size={12} strokeWidth={2} aria-hidden="true" />
        </span>
      ) : undefined;
    const said = layer === "slot" && wrongs.length > 0 ? wrongs.join(", ") : layer === "subagents" && subagentsRunning > 0 ? `${tile?.label ?? name}, ${subagentsRunning} running` : (tile?.label ?? name);
    return {
      id: layer,
      label: name,
      tip: why ?? said,
      Glyph: glyphOf(GLYPH[layer], "zap"),
      ...(key === undefined ? {} : { key }),
      pressed: open === layer,
      ...(why === undefined ? {} : { disabled: true }),
      ...(layer === "slot" && bays !== undefined && baysLit(bays) ? { lit: true } : {}),
      ...(running ? { running: true } : {}),
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

/** A view's button, as in the Job's head: ghost, small, its glyph and its fact on the corner. */
function ViewButton({ view }: { view: ComposerView }) {
  const { id, label, tip, Glyph, key, pressed, disabled = false, lit = false, running = false, mark, onPress } = view;
  return (
    <Tooltip label={tip} {...(key === undefined ? {} : { shortcut: key })}>
      <Button type="button" variant="ghost" size="sm" iconOnly aria-label={label} aria-pressed={pressed} disabled={disabled} data-key={key} data-lit={lit || undefined} data-running={running || undefined} data-view={id} onClick={onPress}>
        <Glyph size={16} strokeWidth={2} aria-hidden />
        {mark === undefined ? null : <span className="sv-view__mark">{mark}</span>}
      </Button>
    </Tooltip>
  );
}

/** `views=head`: the buttons, and a rule after them, for the head's row before its own buttons. */
export function HeadViews({ views }: { views: readonly ComposerView[] }) {
  return (
    <>
      {views.map((view) => (
        <ViewButton key={view.id} view={view} />
      ))}
      <Separator orientation="vertical" className="sv-views__rule" />
    </>
  );
}

// `views=left`: where the Cockpit's left column holds the panel. The Job's own view draws into it, as the Job is where the facts are held.
let slot: HTMLElement | null = null;
const watching = new Set<() => void>();
const hold = (node: HTMLElement | null) => {
  slot = node;
  watching.forEach((one) => one());
};
const watch = (one: () => void) => (watching.add(one), () => void watching.delete(one));

/** The place under the Jobs list, drawn in the left column in this take. It takes no room of its own: what is drawn into it is the column's. */
export function ViewsSlot() {
  return viewsAt() === "left" ? <div ref={hold} className="deck-views-slot" /> : null;
}

/** `views=left`: the panel, drawn into the column. Not drawn with no Job open, as there is no Job to read views of. */
export function LeftViews({ views }: { views: readonly ComposerView[] }) {
  const host = useSyncExternalStore(watch, () => slot);
  const folded = useCollapse("views");
  if (host === null) return null;
  return createPortal(
    <section className="deck-panel deck-views" aria-label="Views" tabIndex={-1} data-collapsed={folded.collapsed || undefined}>
      <header className="deck-band" {...folded.band}>
        Views
        <Tooltip label="Go to Views" shortcut={VIEWS_KEY}>
          <Kbd>{VIEWS_KEY}</Kbd>
        </Tooltip>
      </header>
      <ul className="deck-list__rows">
        {views.map(({ id, label, tip, Glyph, key, pressed, disabled = false, lit = false, running = false, mark, onPress }) => (
          <li key={id} className="deck-list__item">
            <Tooltip label={tip} {...(key === undefined ? {} : { shortcut: key })} asChild>
              {/* Off is `aria-disabled` and not `disabled`, so the row keeps focus and its tooltip says why. */}
              <button type="button" className="deck-row" aria-label={label} aria-pressed={pressed} aria-disabled={disabled || undefined} data-selected={pressed || undefined} data-off={disabled || undefined} data-lit={lit || undefined} data-running={running || undefined} data-key={key} data-view={id} onClick={disabled ? undefined : onPress}>
                <span className="deck-row__title">
                  <Glyph size={16} strokeWidth={2} aria-hidden />
                  {label}
                </span>
                <span className="deck-row__marks">{mark}</span>
              </button>
            </Tooltip>
          </li>
        ))}
      </ul>
    </section>,
    host,
  );
}
