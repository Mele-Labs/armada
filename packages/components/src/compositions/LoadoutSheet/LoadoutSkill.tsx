import { Button } from "../../primitives/Button/Button";
import { loadoutKeyOf, type LoadoutRef } from "../../loadout";

/** What a skill row can be asked to do. */
export type LoadoutSkillAct = "edit";

/** What Kit's inventory lets one skill row do, as the host matched the row against it. */
export type LoadoutSkillActs = {
  /** The SKILL.md Edit opens in the file panel. */
  edits?: string;
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

/** A skill row's acts at its trailing edge: Edit on the face. */
export function SkillActs({ item, acts, onAct }: { item: LoadoutRef; acts: LoadoutSkillActs; onAct: SkillGroup["onAct"] }) {
  if (acts.edits === undefined) return null;
  return (
    <Button variant="secondary" size="sm" aria-label={`Edit ${item.name}`} onClick={() => onAct(item, "edit")}>
      Edit
    </Button>
  );
}
