import { NodeToolbar, Position, useStore } from "@xyflow/react";
import type { LucideIcon } from "lucide-react";
import { useState, type ReactNode } from "react";

import { Button } from "../../primitives/Button/Button";
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
  onPress: () => void;
};

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
  const noRoom = useStore((state) => {
    const [, y, zoom] = state.transform;
    let top = Number.POSITIVE_INFINITY;
    for (const id of nodeIds) {
      const node = state.nodeLookup.get(id);
      if (node === undefined) continue;
      top = Math.min(top, node.internals.positionAbsolute.y * zoom + y);
    }
    return Number.isFinite(top) && top < tall;
  });
  if (nodeIds.length === 0) return null;
  return (
    <NodeToolbar
      nodeId={[...nodeIds]}
      isVisible
      offset={gap}
      position={noRoom ? Position.Bottom : Position.Top}
      className="armada-graph-node-bar"
      role="group"
      aria-label={label}
    >
      {children}
    </NodeToolbar>
  );
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
