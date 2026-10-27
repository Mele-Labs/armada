// Settings → Command palette: what the empty search shows, then the rows that hold an alias, a hotkey or a pin, each
// editable where it stands. The rows are the palette's, named by the app; with none, the table is left out.

import { PaletteRows, Select, type PaletteRowsProps } from "@armada/components";
import { PALETTE_OPENS, type PaletteOpens } from "@armada/shell";

export type PaletteSettingsProps = PaletteRowsProps & {
  opens: PaletteOpens;
  onOpens: (opens: PaletteOpens) => void;
};

export function PaletteSettings({ opens, onOpens, rows, onAlias, onHotkey, onPin }: PaletteSettingsProps) {
  return (
    <div className="armada-palette-settings">
      <Select label="Palette opens with" value={opens} onChange={(event) => onOpens(event.target.value as PaletteOpens)}>
        {PALETTE_OPENS.map((one) => (
          <option key={one.id} value={one.id}>
            {one.label}
          </option>
        ))}
      </Select>
      {rows.length === 0 ? null : <PaletteRows rows={rows} onAlias={onAlias} onHotkey={onHotkey} onPin={onPin} />}
    </div>
  );
}
