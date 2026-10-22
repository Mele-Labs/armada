import { useState } from "react";

import { Button } from "../../primitives/Button/Button";
import { Dialog } from "../../primitives/Dialog/Dialog";
import { SplitButton, type SplitButtonItem } from "../../primitives/SplitButton/SplitButton";
import { loadoutKeyOf, type LoadoutRef } from "../../loadout";

/** What a skill row can be asked to do. */
export type LoadoutSkillAct = "edit" | "remove" | "remove_plugin" | "restore";

/** What Kit's inventory lets one skill row do, as the host matched the row against it. */
export type LoadoutSkillActs = {
  /** The SKILL.md Edit opens in the file panel. */
  edits?: string;
  /** One of the person's own: Remove puts its folder in the Trash. */
  removes?: boolean;
  /** The plugin it came with, by its name without the marketplace: Remove plugin runs `uninstall` and takes every skill in `takes`. */
  plugin?: { name: string; uninstall?: string; takes: readonly string[] };
  /** Gone from this Mac. `restorable` while the Trash holds it and Restore can bring it back. */
  removed?: { restorable: boolean };
  /** The act sent and not answered yet. */
  acting?: Exclude<LoadoutSkillAct, "edit">;
  /** Fleet's words for the last act refused on this row. */
  refused?: string;
};

/** Kit's acts on the Loadout's skills, by `loadoutKeyOf`. A row the host did not match offers none. */
export type SkillGroup = { rows: Readonly<Record<string, LoadoutSkillActs>>; onAct: (item: LoadoutRef, act: LoadoutSkillAct) => void };

/** Kit holds the person's own skills and the plugins'. A repository's, a built in or a managed one is not Kit's to change. */
const KIT_HOLDS: ReadonlySet<LoadoutRef["source"]> = new Set(["kit", "plugin"]);

/** What one row offers, or nothing where it is not a skill Kit holds. */
export function skillActsOf(item: LoadoutRef, skills: SkillGroup | undefined): LoadoutSkillActs | undefined {
  if (skills === undefined || item.kind !== "skill" || !KIT_HOLDS.has(item.source)) return undefined;
  return skills.rows[loadoutKeyOf(item)];
}

/** Whether a row has anything at its trailing edge: a removed skill the Trash no longer holds has only its mark. */
export const skillDraws = (acts: LoadoutSkillActs): boolean =>
  acts.acting !== undefined || acts.refused !== undefined || (acts.removed === undefined ? acts.edits !== undefined : acts.removed.restorable);

const UNDERWAY: Record<NonNullable<LoadoutSkillActs["acting"]>, string> = { remove: "Removing", remove_plugin: "Removing", restore: "Restoring" };

/**
 * A skill row's acts at its trailing edge. Edit is on the face, and Remove or Remove plugin behind the caret, each
 * behind a confirm; once removed the row offers Restore alone, where the Trash holds it. An act out takes the face.
 */
export function SkillActs({ item, acts, onAct }: { item: LoadoutRef; acts: LoadoutSkillActs; onAct: SkillGroup["onAct"] }) {
  const [confirming, setConfirming] = useState<"remove" | "remove_plugin" | undefined>(undefined);
  const refused = acts.refused === undefined ? {} : { answer: "refused" as const };
  const { plugin } = acts;
  const menu: SplitButtonItem[] =
    acts.removed !== undefined
      ? []
      : [
          ...(acts.removes === true ? [{ label: "Remove", danger: true, onSelect: () => setConfirming("remove") }] : []),
          ...(plugin === undefined ? [] : [{ label: `Remove plugin ${plugin.name}`, danger: true, onSelect: () => setConfirming("remove_plugin") }]),
        ];
  const confirmed = (act: "remove" | "remove_plugin") => () => {
    setConfirming(undefined);
    onAct(item, act);
  };
  const face = acts.acting !== undefined ? (
    <Button variant="secondary" size="sm" aria-label={`${UNDERWAY[acts.acting]} ${item.name}`} pending>
      {UNDERWAY[acts.acting]}
    </Button>
  ) : acts.removed !== undefined ? (
    acts.removed.restorable ? (
      <Button variant="secondary" size="sm" aria-label={`Restore ${item.name}`} {...refused} onClick={() => onAct(item, "restore")}>
        Restore
      </Button>
    ) : null
  ) : acts.edits === undefined ? null : menu.length === 0 ? (
    <Button variant="secondary" size="sm" aria-label={`Edit ${item.name}`} onClick={() => onAct(item, "edit")}>
      Edit
    </Button>
  ) : (
    <SplitButton size="sm" faceLabel={`Edit ${item.name}`} menuLabel={`More for ${item.name}`} items={menu} onAction={() => onAct(item, "edit")}>
      Edit
    </SplitButton>
  );
  return (
    <>
      {face}
      {acts.refused === undefined ? null : <p className="armada-loadout__refused">{acts.refused}</p>}
      {acts.removes !== true ? null : (
        <Dialog open={confirming === "remove"} title={`Remove ${item.name} from this Mac`} confirmLabel="Remove" onConfirm={confirmed("remove")} onCancel={() => setConfirming(undefined)}>
          <p>Every Job on this Mac loses it from its next start.</p>
          <p>Its folder goes to the Trash. Restore on its row brings it back.</p>
        </Dialog>
      )}
      {plugin === undefined ? null : (
        <Dialog
          open={confirming === "remove_plugin"}
          title={`Remove plugin ${plugin.name} from this Mac`}
          confirmLabel="Remove plugin"
          onConfirm={confirmed("remove_plugin")}
          onCancel={() => setConfirming(undefined)}
        >
          {plugin.uninstall === undefined ? null : <pre className="armada-loadout__commands">{plugin.uninstall}</pre>}
          {plugin.takes.length === 0 ? null : <p>Removes its skills: {plugin.takes.join(", ")}.</p>}
          <p>Every Job on this Mac loses them from its next start.</p>
          <p>There is no Undo.</p>
        </Dialog>
      )}
    </>
  );
}
