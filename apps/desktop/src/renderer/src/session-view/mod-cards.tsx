// The mods a Mod Job made, drawn in its thread where they were made: the same card System › Mods draws, with its switch.

import { Card, CardContent, ModRow } from "@armada/components";
import type { ThreadCard } from "@armada/components";

import { useModsMade } from "../mods-made";

/** One card per mod the Job made, placed by when the mod's files were last written. Empty for a Job that made none. */
export function useModCards(id: string): readonly ThreadCard[] {
  const made = useModsMade(id);
  return made.mods.map((one) => ({
    key: `mod:${one.name}`,
    ...(one.changedAt === undefined ? {} : { at: one.changedAt }),
    node: (
      <Card>
        <CardContent>
          <ModRow title={one.title} kind={one.kind} enabled={one.enabled} onEnabled={(on) => made.setEnabled(one.name, on)} />
        </CardContent>
      </Card>
    ),
  }));
}
