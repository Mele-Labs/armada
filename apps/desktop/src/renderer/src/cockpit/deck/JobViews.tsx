// The Job's views at the top of its spine: Artifacts, Checks, Subagents and Bays, a strip of icon buttons above the story, and Lanes after them on a Job that holds lifecycles, which open in the side panel. Thread is the thread itself; the five are the layers (`session-view/layers.ts`). A press
// opens the layer, a press on the lit one shuts it, as Esc does. A view with nothing in it stays in its place, off, with its reason as the tooltip (`offeredOf`), so nothing moves. The facts the tiles carried
// stay with each: Subagents the number running, Checks a pulse while any runs, Bays the clash, and Bays tinted while a bay is wrong. **A layer opens in the spine, under the strip**, so the strip stays on show while a layer is read. The keys press them (`flight-keys.ts`).

import type { ReactNode } from "react";
import { StretchHorizontal, type LucideIcon } from "lucide-react";
import { Button, Tooltip } from "@armada/components";
import { attachmentsOf, type Session } from "@armada/screens/src/draft/sessions";

import { useSessionsDraft } from "../../sessions-draft";
import { toggleLayer, useLayer } from "../../session-view/layers";
import { toggleLanesPanel, useLanesOpen } from "../../shared-panel/store";
import { ARTIFACTS_KEY, LANES_KEY } from "../../session-view/story";
import { baysLit, type SessionBays } from "../../session-view/slot-trouble";
import { tilesOf, useFlightFile } from "./flight-controls";
import { glyphOf } from "./flight-glyphs";
import { VIEW_LAYERS, flightKey } from "./flight-order";
import { offeredOf } from "./session-tiles";
import "./JobViews.css";

/**
 * A view the head opens: an icon button that is lit while its layer is open and presses it back shut. `tip` is the tooltip's line, a disabled view's reason or a lit one's fact; `key` is the key that does
 * the same, named on the tooltip and carried as `data-key` for the host's handler. `mark` is a fact on the button's corner.
 */
export type JobView = {
  id: string;
  label: string;
  tip: string;
  Glyph: LucideIcon;
  key?: string;
  /** A second key that does the same, named on the tooltip beside `key` and not the host's handler's to find. */
  alsoKey?: string;
  pressed: boolean;
  disabled?: boolean;
  /** A fact that tints the glyph, set as `data-lit` for the host's style. */
  lit?: boolean;
  /** Something is running in the view: the glyph is the running hue and pulses, set as `data-running` for the host's style. */
  running?: boolean;
  mark?: ReactNode;
  onPress: () => void;
};

const GLYPH: Record<(typeof VIEW_LAYERS)[number], string> = { artifacts: "library", checks: "shield-check", subagents: "drone", slot: "rows-3" };

/** The Job's four views, or none where there is no draft to read what is on. */
export function useJobViews(session: Session, bays: SessionBays | undefined): readonly JobView[] | undefined {
  const open = useLayer(session.id);
  const lanesOpen = useLanesOpen(session.id);
  const draft = useSessionsDraft();
  const tiles = tilesOf("job", useFlightFile());
  if (draft === undefined) return undefined;
  const { off } = offeredOf(session, draft, undefined, (bays?.bays.length ?? 0) > 0);
  const subagentsRunning = attachmentsOf(session, "subagent").filter((one) => one.state === "running").length;
  const checksRunning = session.rows.some((one) => one.kind === "check" && one.state === "running");
  const clashing = (bays?.clashes ?? []).map((one) => (one.one.kind === "session" && one.one.id === session.id ? one.other : one.one).label);
  const clashSaid = clashing.length === 0 ? undefined : `Write clash with ${clashing.join(", ")}`;
  const wrongs = [...(bays?.bays ?? []).flatMap((one) => (one.said === undefined ? [] : [one.said])), ...(clashSaid === undefined ? [] : [clashSaid])];
  const views = VIEW_LAYERS.map((layer): JobView => {
    const tile = tiles.find((one) => one.builtIn !== undefined && one.id === layer);
    const key = tile === undefined ? undefined : flightKey(tile);
    const why = off[layer];
    const name = tile?.text ?? layer;
    // Running reads on the glyph itself, in the running hue and pulsing (`JobViews.css`), and the number running is in the tooltip. Only a clash is a mark.
    const running = (layer === "checks" && checksRunning) || (layer === "subagents" && subagentsRunning > 0);
    const mark =
      layer === "slot" && clashSaid !== undefined ? (
        <span className="sv-view__clash" role="img" aria-label={clashSaid} />
      ) : undefined;
    const said = layer === "slot" && wrongs.length > 0 ? wrongs.join(", ") : layer === "subagents" && subagentsRunning > 0 ? `${tile?.label ?? name}, ${subagentsRunning} running` : (tile?.label ?? name);
    return {
      id: layer,
      label: name,
      tip: why ?? said,
      Glyph: glyphOf(GLYPH[layer], "zap"),
      ...(key === undefined ? {} : { key }),
      ...(layer === "artifacts" ? { alsoKey: ARTIFACTS_KEY.cap } : {}),
      pressed: open === layer,
      ...(why === undefined ? {} : { disabled: true }),
      ...(layer === "slot" && bays !== undefined && baysLit(bays) ? { lit: true } : {}),
      ...(running ? { running: true } : {}),
      ...(mark === undefined ? {} : { mark }),
      onPress: () => toggleLayer(session.id, layer),
    };
  });
  // Lanes is a Job's own when the Workspace gives it lifecycles. It has no flight key: its ⌥⇧L is heard by the workspace, so the host's `data-key` handler does not toggle it twice.
  if (session.lanes === undefined) return views;
  return [...views, { id: "lanes", label: "Lanes", tip: "Lanes", Glyph: StretchHorizontal, alsoKey: LANES_KEY.cap, pressed: lanesOpen, onPress: () => toggleLanesPanel(session.id) }];
}

/** A view's button, as in the Job's head: ghost, small, its glyph and its fact on the corner. */
function ViewButton({ view }: { view: JobView }) {
  const { id, label, tip, Glyph, key, alsoKey, pressed, disabled = false, lit = false, running = false, mark, onPress } = view;
  return (
    <Tooltip label={tip} {...(key === undefined && alsoKey === undefined ? {} : { shortcut: [key, alsoKey].filter((one) => one !== undefined).join(" ") })}>
      <Button type="button" variant="ghost" size="sm" iconOnly aria-label={label} aria-pressed={pressed} disabled={disabled} data-key={key} data-lit={lit || undefined} data-running={running || undefined} data-view={id} onClick={onPress}>
        <Glyph size={16} strokeWidth={2} aria-hidden />
        {mark === undefined ? null : <span className="sv-view__mark">{mark}</span>}
      </Button>
    </Tooltip>
  );
}

/** The four views as a strip pinned above the spine, outside what scrolls. */
export function ViewStrip({ views }: { views: readonly JobView[] }) {
  return (
    <div className="sv-views" role="toolbar" aria-label="Views">
      {views.map((view) => (
        <ViewButton key={view.id} view={view} />
      ))}
    </div>
  );
}
