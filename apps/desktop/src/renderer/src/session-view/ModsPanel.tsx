// Mods made: the mods a Mod Job made, each with its switch. A mod applies as it is saved, and turning it off is the undo.
// It stands in the Landing panel when a Job lands in a mod (`LandingPanel.tsx`), and draws nothing until the first one is listed.

import { ModRow } from "@armada/components";

import { useModsMade } from "../mods-made";

export function ModsMade({ id }: { id: string }) {
  const made = useModsMade(id);
  if (made.mods.length === 0) return null;
  return (
    <div className="sv-mods">
      {made.mods.map((one) => (
        <ModRow key={one.name} title={one.name} kind={one.kind} enabled={one.enabled} onEnabled={(on) => made.setEnabled(one.name, on)} />
      ))}
    </div>
  );
}
