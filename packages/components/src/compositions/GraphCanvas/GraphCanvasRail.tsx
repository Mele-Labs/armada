import { NodeToolbar, Position, useReactFlow, useStore, useStoreApi } from "@xyflow/react";
import type { LucideIcon } from "lucide-react";
import { useEffect, useState, type ReactNode } from "react";

import { Button } from "../../primitives/Button/Button";
import { DropdownMenu, type DropdownMenuEntry } from "../../primitives/DropdownMenu/DropdownMenu";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * What a canvas draws over itself — `docs/contracts/iconography.md`, *The
 * canvas rail*, which holds the rule and every glyph refused.
 *
 * **One rail and one hovering bar for every canvas.** The owner asked for both
 * on the sketch pad and on a Studio's whiteboard in one sitting, then for all
 * four canvases at once, so the surfaces supply the acts and this draws them.
 *
 * **The rail's frame and its view group are `GraphCanvas`'s**; a surface hands
 * in the groups above them. Everything is mounted inside the canvas, so a rail
 * act may read the viewport it places something into.
 */

/** A control's glyph: 16px at strokeWidth 2, and never tuned per size. */
const RAIL_ICON = 16;
const RAIL_STROKE = 2;

/**
 * One act on the rail. Icon-only, so `name` is the whole of what a reader gets.
 *
 * **`icon` or `sign`, one of the two.** Nearly every act draws a glyph; the
 * zoom pair draws `−` and `+`, which are signs rather than glyphs and stay so
 * under the default-to-no-icon rule.
 */
export type GraphCanvasRailAct = {
  id: string;
  /** Sentence case, naming what the press does. The accessible name and the tooltip. */
  name: string;
  /** From `packages/icons/icons.toml`, group `Canvas rail`. */
  icon?: LucideIcon;
  /** The character the button draws where no glyph is right for it. */
  sign?: string;
  /**
   * The binding, where the act has one. Drawn as the tooltip's trailing kbd,
   * which is the only way a person finds the key without reading a contract.
   */
  shortcut?: string;
  /**
   * A mode rather than a press that is over — the pen, or following the run.
   * Drawn held down, and said as `aria-pressed` for a reader who cannot see it.
   */
  pressed?: boolean;
  disabled?: boolean;
  /**
   * Why the act is off. It replaces the name in the tooltip: a dead control
   * with no reason reads as broken, and the name does not say why.
   */
  why?: string;
} & (
  | { onPress: () => void; menu?: never }
  | {
      /**
       * A press that opens a list to choose from, rather than one that acts —
       * Run, whose press opens the checkout's commands (the owner, 2 Oct 2026).
       * The chosen entry goes to `onSelect`.
       */
      menu: {
        entries: DropdownMenuEntry[];
        onSelect: (id: string) => void;
        /** Held by the surface, where something besides the press opens it — Run's `R`. */
        open?: boolean;
        onOpenChange?: (open: boolean) => void;
      };
      onPress?: never;
    }
);

export type GraphCanvasRailGroupProps = {
  /** What the group is, read to somebody who cannot see it. */
  label: string;
  acts: readonly GraphCanvasRailAct[];
  /** Nothing may be pressed while the connection is not live. */
  disabled?: boolean;
};

/**
 * One group of the rail — what a person places, how they look at it, or what
 * the view follows. Empty draws nothing, which is how a canvas with no tools
 * of its own carries a rail at all.
 */
export function GraphCanvasRailGroup({ label, acts, disabled = false }: GraphCanvasRailGroupProps) {
  if (acts.length === 0) return null;
  return (
    <div className="armada-graph-rail__group" role="group" aria-label={label}>
      {acts.map((act) => {
        const off = disabled || act.disabled === true;
        const Glyph = act.icon;
        return (
          <Tooltip
            key={act.id}
            label={off && act.why !== undefined ? act.why : act.name}
            {...(act.shortcut === undefined ? {} : { shortcut: act.shortcut })}
          >
            {act.menu !== undefined ? (
              <DropdownMenu
                {...(Glyph === undefined ? {} : { icon: Glyph })}
                triggerLabel={act.name}
                align="start"
                entries={act.menu.entries}
                disabled={off}
                onSelect={act.menu.onSelect}
                {...(act.menu.open === undefined ? {} : { open: act.menu.open })}
                {...(act.menu.onOpenChange === undefined ? {} : { onOpenChange: act.menu.onOpenChange })}
              />
            ) : (
              <Button
                variant="ghost"
                size="sm"
                iconOnly
                aria-label={act.name}
                {...(act.pressed === undefined ? {} : { "aria-pressed": act.pressed })}
                disabled={off}
                onClick={act.onPress}
              >
                {Glyph === undefined ? act.sign : <Glyph size={RAIL_ICON} strokeWidth={RAIL_STROKE} aria-hidden />}
              </Button>
            )}
          </Tooltip>
        );
      })}
    </div>
  );
}

export type GraphCanvasNodeBarProps = {
  /** What the bar is, read to somebody who cannot see where it is drawn. */
  label: string;
  /**
   * What it is about. **Every picked node, not the first**: the bar hangs over
   * the whole selection's bounds, which is what puts Join above the two boxes
   * it joins rather than above one of them.
   *
   * Empty draws nothing — a bar over the middle of a canvas is the fixed panel
   * this replaced.
   */
  nodeIds: readonly string[];
  /** The acts. The canvas decides nothing about what they are. */
  children: ReactNode;
};

/**
 * One act in the bar hovering over a node. **A press that acts, never a press
 * that opens something** — the owner's correction of 28 Sep 2026: *a toolbar
 * that has icon buttons for the actions that I can take on this node*.
 *
 * **A glyph where the registry sanctions one, the word where it does not.**
 * `docs/contracts/iconography.md`, *The node bar*, holds what was minted and
 * what was refused; nothing here reaches for a glyph that row does not give.
 */
export type GraphCanvasNodeActProps = {
  /** Sentence case, naming what the press does. The accessible name. */
  name: string;
  /** From `packages/icons/icons.toml`. Absent draws the name. */
  icon?: LucideIcon;
  /** Deleting. Drawn in the failure colour, and the caller draws it last. */
  danger?: boolean;
  disabled?: boolean;
  /** Why the act is off. A dead control with no reason reads as broken. */
  why?: string;
  onPress: () => void;
};

/**
 * One button in the hovering bar.
 *
 * **The tooltip is the icon-only button's only name.** A button drawing its
 * word already says what it does, and a bubble restating it is the one thing
 * `docs/contracts/design-system.md` says a tooltip never does — so a word takes
 * one only where the act is off and the reason is not on the face of it.
 */
export function GraphCanvasNodeAct({
  name,
  icon: Glyph,
  danger = false,
  disabled = false,
  why,
  onPress,
}: GraphCanvasNodeActProps) {
  const said = disabled && why !== undefined ? why : name;
  const button = (
    <Button
      variant={danger ? "destructive" : "ghost"}
      size="sm"
      iconOnly={Glyph !== undefined}
      aria-label={name}
      disabled={disabled}
      onClick={onPress}
    >
      {Glyph === undefined ? name : <Glyph size={RAIL_ICON} strokeWidth={RAIL_STROKE} aria-hidden />}
    </Button>
  );
  if (Glyph === undefined && said === name) return button;
  return <Tooltip label={said}>{button}</Tooltip>;
}

/**
 * What hovers over the node a person has selected.
 *
 * **React Flow's own `NodeToolbar`, so no new layer.** It portals into the
 * canvas's wrapper and holds position against pan and zoom without scaling with
 * them, which is all this needed; a layer of Armada's own would be a second
 * answer to a question hard rule 6 settles.
 *
 * **`isVisible` is set rather than left to read the selection**, whose default
 * hides the bar past one node — and two boxes picked is when Join has work.
 */
export function GraphCanvasNodeBar({ label, nodeIds, children }: GraphCanvasNodeBarProps) {
  // Read once, when the bar first appears: `NodeToolbar` takes the gap as a
  // number, so the token is read as one rather than restated here.
  const [gap] = useState(gapAboveTheNode);
  const [tall] = useState(() => gapAboveTheNode() + heightOfTheBar());
  // **Under the node where there is no room over it.** The canvas clips, and a
  // node dragged to the top edge — or fitted there, which is where the sketch
  // pad opens — put the bar outside the frame with nothing to say it was
  // there. `NodeToolbar` collides with nothing on its own.
  //
  // **And over the top of the picked nodes where there is room on neither
  // side**, held just inside the frame. Every node picked on a fitted Studio
  // spans the whole canvas, and the bar landed wholly below its bottom edge,
  // out of reach — the very case #1411 was built for.
  const placed = useStore((state) => {
    const [, y, zoom] = state.transform;
    let top = Number.POSITIVE_INFINITY;
    let bottom = Number.NEGATIVE_INFINITY;
    for (const id of nodeIds) {
      const node = state.nodeLookup.get(id);
      if (node === undefined) continue;
      const at = node.internals.positionAbsolute.y;
      top = Math.min(top, at * zoom + y);
      bottom = Math.max(bottom, (at + (node.measured.height ?? 0)) * zoom + y);
    }
    if (!Number.isFinite(top) || top >= tall) return "over";
    if (bottom + tall <= state.height) return "under";
    return Math.round(top - tall);
  });
  const flow = useReactFlow();
  const store = useStoreApi();
  const picked = nodeIds.join(" ");
  // **Panned to, never clamped** — the owner, 1 Oct 2026. A node picked at the
  // side of the frame centres its bar past the edge, and React Flow's own focus
  // pan moves only a node none of which shows. On a pick and not after it, so a
  // person who pans away is not pulled back.
  useEffect(() => {
    // Found by its class: `NodeToolbar` takes no ref, and a canvas draws one bar.
    const canvas = store.getState().domNode;
    const bar = canvas?.querySelector(".armada-graph-node-bar")?.getBoundingClientRect();
    const frame = canvas?.getBoundingClientRect();
    if (bar === undefined || frame === undefined) return;
    const bounds = flow.getNodesBounds([...nodeIds]);
    const near = flow.flowToScreenPosition({ x: bounds.x, y: bounds.y });
    const far = flow.flowToScreenPosition({ x: bounds.x + bounds.width, y: bounds.y + bounds.height });
    const dx = intoView([Math.min(near.x, bar.left), Math.max(far.x, bar.right)], [bar.left, bar.right], [frame.left, frame.right]);
    const dy = intoView([Math.min(near.y, bar.top), Math.max(far.y, bar.bottom)], [bar.top, bar.bottom], [frame.top, frame.bottom]);
    if (dx === 0 && dy === 0) return;
    const { x, y, zoom } = flow.getViewport();
    void flow.setViewport({ x: x + dx, y: y + dy, zoom }, { duration: token("--duration-travel") });
  }, [picked]);
  if (nodeIds.length === 0) return null;
  return (
    <NodeToolbar
      nodeId={[...nodeIds]}
      isVisible
      offset={typeof placed === "number" ? placed : gap}
      position={placed === "under" ? Position.Bottom : Position.Top}
      className="armada-graph-node-bar"
      role="group"
      aria-label={label}
    >
      {children}
    </NodeToolbar>
  );
}

type Span = readonly [from: number, to: number];

/**
 * How far to move `pick` so it lies inside `frame`, the least distance that
 * does. **The bar alone where the whole pick is wider than the frame**: the bar
 * is what has to be reached.
 */
function intoView(pick: Span, bar: Span, frame: Span): number {
  const room = frame[1] - frame[0];
  const [from, to] = pick[1] - pick[0] <= room ? pick : bar;
  if (to - from > room) return 0;
  if (from < frame[0]) return frame[0] - from;
  if (to > frame[1]) return frame[1] - to;
  return 0;
}

/**
 * How far the bar sits above the node, off the spacing scale. **Nothing where
 * the token cannot be read**: a bar flush to the card is a defect anybody can
 * see, and a pixel count standing in for the token is one nobody can find.
 */
function gapAboveTheNode(): number {
  return token("--space-2");
}

/**
 * How tall the bar is: a small control, the padding either side of it and its
 * own edges. Read rather than measured, so nothing observes the DOM to decide
 * which side of a node the bar goes on.
 */
function heightOfTheBar(): number {
  return token("--h-control-sm") + token("--space-1") * 2 + token("--border-width") * 2;
}

/** One length off the token set, as a number. Nothing where it cannot be read. */
function token(name: string): number {
  const read = Number.parseFloat(getComputedStyle(document.body).getPropertyValue(name));
  return Number.isFinite(read) ? read : 0;
}
