import type { LucideIcon } from "lucide-react";

import { GraphCanvasNodeAct } from "../GraphCanvas/GraphCanvasRail";

/**
 * The acts on what is picked on a Studio's whiteboard — #1399.
 *
 * **A row of buttons, each one press.** It was one control opening a menu
 * until the owner read it on 28 Sep 2026: *when I said a toolbar that hovers, I
 * didn't mean a popup with the dropdown. I meant something like a toolbar that
 * has icon buttons for the actions that I can take on this node.* A menu asks
 * for a press to see what is there and a second to use it; a toolbar asks one.
 *
 * **The node's own title is gone with it**, at his word in the same note: the
 * bar hovers over the card it is about, so a line naming that card says what
 * the person is already looking at.
 */

/**
 * One act on what is picked. `id` is the caller's own word for it.
 *
 * **What decides between a glyph and a word is the registry, never the room** —
 * `docs/contracts/iconography.md`, `[node-bar-glyphs]`.
 */
export type StudioPickedAct = {
  id: string;
  /** Sentence case, naming what happens. The button's word, or its tooltip. */
  label: string;
  /** The glyph, where `iconography.md` sanctions one. Absent draws `label`. */
  icon?: LucideIcon;
  /** Deleting. Drawn last, in the failure colour. */
  danger?: boolean;
};

export type StudioPickedProps = {
  /** What the selection offers. None draws nothing at all. */
  acts: readonly StudioPickedAct[];
  onAct: (id: string) => void;
  /** Something is already out to Fleet: the bar sends no second press. */
  disabled?: boolean;
};

/** Destructive acts last, which is the rule the menu kept and a row keeps too. */
function ordered(acts: readonly StudioPickedAct[]): StudioPickedAct[] {
  return [...acts.filter((act) => act.danger !== true), ...acts.filter((act) => act.danger === true)];
}

/** **No surface of its own**: `GraphCanvasNodeBar` is the frame it sits in. */
export function StudioPicked({ acts, onAct, disabled = false }: StudioPickedProps) {
  if (acts.length === 0) return null;
  return (
    <>
      {ordered(acts).map((act) => (
        <GraphCanvasNodeAct
          key={act.id}
          name={act.label}
          {...(act.icon === undefined ? {} : { icon: act.icon })}
          {...(act.danger === undefined ? {} : { danger: act.danger })}
          disabled={disabled}
          onPress={() => onAct(act.id)}
        />
      ))}
    </>
  );
}
