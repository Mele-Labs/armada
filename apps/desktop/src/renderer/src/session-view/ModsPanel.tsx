// Mods made: the mods a Mod Job made, in its right column, each with its switch. A mod applies as it is saved, and turning it off is the undo.
// Drawn only for a Mod Job. Empty it stands folded to its band and opens on the first mod.

import { ModRow } from "@armada/components";

import { useModsMade } from "../mods-made";
import { SvPanel, type Place } from "./SvPanel";

export function ModsPanel({ id, mod, place }: { id: string; mod: boolean; place: Place }) {
  return mod ? <Made id={id} place={place} /> : null;
}

function Made({ id, place }: { id: string; place: Place }) {
  const made = useModsMade(id);
  return (
    <SvPanel label="Mods made" area="mods" place={place} empty={made.mods.length === 0}>
      {made.mods.length === 0 ? undefined : (
        <div className="sv-mods">
          {made.mods.map((one) => (
            <ModRow key={one.name} title={one.title} kind={one.kind} enabled={one.enabled} onEnabled={(on) => made.setEnabled(one.name, on)} />
          ))}
        </div>
      )}
    </SvPanel>
  );
}
