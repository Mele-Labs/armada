// Mock-only: the mods on this machine, held in memory until Fleet serves the mod folder. Each
// is a `theme.css` as a mod would ship it: one `[data-theme="<name>"]` block over the tokens.

import { createThemeSource } from "@armada/settings";
import type { ThemeMod } from "@armada/settings";

const block = (name: string, vars: Record<string, string>) =>
  `[data-theme="${name}"] {\n${Object.entries(vars).map(([key, value]) => `  ${key}: ${value};`).join("\n")}\n}\n`;

const mod = (name: string, title: string, vars: Record<string, string>, enabled = true): ThemeMod => ({ name, title, css: block(name, vars), enabled, promoted: false });

const NORD = mod("nord", "Nord", {
  "--bg-base": "#2E3440", "--bg-sunken": "#272C36", "--bg-raised": "#3B4252", "--bg-overlay": "#434C5E", "--bg-hover": "#4C566A",
  "--border-subtle": "#3F4759", "--border-default": "#4C566A", "--border-strong": "#616E88",
  "--fg-default": "#ECEFF4", "--fg-muted": "#C2CBDA", "--fg-subtle": "#A9B4C8", "--fg-inverse": "#2E3440",
  "--accent": "#88C0D0", "--accent-hover": "#8FBCBB", "--accent-muted": "#3E5663",
  "--bg-glass": "rgb(59 66 82 / 0.9)", "--bg-glass-end": "rgb(46 52 64 / 0.9)",
});

const SOLARIZED = mod("solarized", "Solarized", {
  "--bg-base": "#FDF6E3", "--bg-sunken": "#EEE8D5", "--bg-raised": "#FFFBEF", "--bg-overlay": "#FFFBEF", "--bg-hover": "#EEE8D5",
  "--border-subtle": "#EAE3CC", "--border-default": "#D8D0B8", "--border-strong": "#93A1A1",
  "--fg-default": "#073642", "--fg-muted": "#44636C", "--fg-subtle": "#586E75", "--fg-inverse": "#FDF6E3",
  "--accent": "#268BD2", "--accent-hover": "#1D70AB", "--accent-muted": "#DCE9EC",
  "--bg-glass": "rgb(255 251 239 / 0.9)", "--bg-glass-end": "rgb(238 232 213 / 0.9)",
});

/** The theme a Session writes into the mod folder in the `mods-themes` scenario. */
export const DUSK_MOD = mod("dusk", "Dusk", {
  "--bg-base": "#1A1822", "--bg-sunken": "#13111A", "--bg-raised": "#231F2E", "--bg-overlay": "#2B2638", "--bg-hover": "#332D42",
  "--border-subtle": "#2A2536", "--border-default": "#383149", "--border-strong": "#4D4463",
  "--fg-default": "#ECE8F3", "--fg-muted": "#A9A1BA", "--fg-subtle": "#9189A4", "--fg-inverse": "#1A1822",
  "--accent": "#E8A25C", "--accent-hover": "#F0B274", "--accent-muted": "#473422",
  "--bg-glass": "rgb(35 31 46 / 0.9)", "--bg-glass-end": "rgb(26 24 34 / 0.9)",
});

/** Nord and Solarized installed, Dark in force: where every mock window starts. */
export const mockThemes = createThemeSource(() => ({ mods: [NORD, SOLARIZED], active: "dark" }));
