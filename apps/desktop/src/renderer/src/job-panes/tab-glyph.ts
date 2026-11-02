// The glyph a tab mod wears: its `glyph` by name in the icon registry, among the names Bridge draws. A name it does not draw wears the Custom glyph.

import { Gauge, PanelTop, type LucideIcon } from "lucide-react";

import { GLYPHS } from "../cockpit/deck/flight-glyphs";

/** The names a tab's code and `mod.toml` may use. */
export const TAB_GLYPHS: Readonly<Record<string, LucideIcon>> = {
  ...GLYPHS,
  gauge: Gauge,
  "panel-top": PanelTop,
};

export const tabGlyph = (name: string | undefined): LucideIcon =>
  (name === undefined ? undefined : TAB_GLYPHS[name]) ?? PanelTop;

/** The same for an instrument mod, whose starter names `gauge`: a name it does not draw wears that. */
export const instrumentGlyph = (name: string | undefined): LucideIcon =>
  (name === undefined ? undefined : TAB_GLYPHS[name]) ?? Gauge;
