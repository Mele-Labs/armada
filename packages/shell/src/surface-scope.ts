// What the title row's picker means on each surface. No React, so a node test
// reaches it.
//
// The picker was a filter on some surfaces, a prerequisite on others and
// ignored on a few, with nothing on screen saying which. The owner could not
// tell when picking mattered (11 Oct 2026), so the picker now says it.

import { SURFACE } from "./surfaces";

/**
 * - `repository`: the surface shows what the pick names, or every repository on All.
 * - `machine`: the surface is about this installation, and a pick changes nothing on it.
 */
export type SurfaceScope = "repository" | "machine";

// Kit is not here: its second tier is a repository's, and it reads the pick.
// Nor is Worktree Slots: the owner moved it to the pick on 13 Oct 2026.
const MACHINE: ReadonlySet<string> = new Set([
  SURFACE.settings,
  SURFACE.guides,
  SURFACE.mods,
  SURFACE.doctor,
]);

export function surfaceScope(surface: string): SurfaceScope {
  return MACHINE.has(surface) ? "machine" : "repository";
}

/** What the picker's trigger reads where the pick does not apply. */
export const MACHINE_SCOPE_SAID = "This machine";
