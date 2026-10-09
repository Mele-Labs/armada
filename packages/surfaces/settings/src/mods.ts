// Mock-only: the mods on this machine and the theme in force. Held in this module until Fleet serves
// either; a theme overrides custom properties on the document root and nothing else. A mod lives at
// `~/Library/Application Support/Armada/mods/<name>/` as `mod.toml` and `theme.css`.

import { useSyncExternalStore } from "react";

export type ModKind = "theme";
export type Mod = { name: string; title: string; kind: ModKind; enabled: boolean; promoted: boolean; vars: Record<string, string> };

/** Dark is the shipped token set; Light is the baseline drawn under `[data-theme="light"]`. */
const LIGHT: Record<string, string> = {
  "--bg-base": "#F6F8FA", "--bg-sunken": "#EDF0F3", "--bg-raised": "#FFFFFF", "--bg-overlay": "#FFFFFF", "--bg-hover": "#E8ECF0",
  "--border-subtle": "#E3E8ED", "--border-default": "#D0D7DE", "--border-strong": "#AEB8C3",
  "--fg-default": "#18202A", "--fg-muted": "#4D5B6B", "--fg-subtle": "#5F6D7D", "--fg-inverse": "#FFFFFF",
  "--accent": "#1F6FB2", "--accent-hover": "#185C95", "--accent-muted": "#D6E7F5",
  "--bg-glass": "rgb(255 255 255 / 0.9)", "--bg-glass-end": "rgb(246 248 250 / 0.9)",
};

const NORD: Record<string, string> = {
  "--bg-base": "#2E3440", "--bg-sunken": "#272C36", "--bg-raised": "#3B4252", "--bg-overlay": "#434C5E", "--bg-hover": "#4C566A",
  "--border-subtle": "#3F4759", "--border-default": "#4C566A", "--border-strong": "#616E88",
  "--fg-default": "#ECEFF4", "--fg-muted": "#C2CBDA", "--fg-subtle": "#A9B4C8", "--fg-inverse": "#2E3440",
  "--accent": "#88C0D0", "--accent-hover": "#8FBCBB", "--accent-muted": "#3E5663",
  "--bg-glass": "rgb(59 66 82 / 0.9)", "--bg-glass-end": "rgb(46 52 64 / 0.9)",
};

const SOLARIZED: Record<string, string> = {
  "--bg-base": "#FDF6E3", "--bg-sunken": "#EEE8D5", "--bg-raised": "#FFFBEF", "--bg-overlay": "#FFFBEF", "--bg-hover": "#EEE8D5",
  "--border-subtle": "#EAE3CC", "--border-default": "#D8D0B8", "--border-strong": "#93A1A1",
  "--fg-default": "#073642", "--fg-muted": "#44636C", "--fg-subtle": "#586E75", "--fg-inverse": "#FDF6E3",
  "--accent": "#268BD2", "--accent-hover": "#1D70AB", "--accent-muted": "#DCE9EC",
  "--bg-glass": "rgb(255 251 239 / 0.9)", "--bg-glass-end": "rgb(238 232 213 / 0.9)",
};

const DUSK: Record<string, string> = {
  "--bg-base": "#1A1822", "--bg-sunken": "#13111A", "--bg-raised": "#231F2E", "--bg-overlay": "#2B2638", "--bg-hover": "#332D42",
  "--border-subtle": "#2A2536", "--border-default": "#383149", "--border-strong": "#4D4463",
  "--fg-default": "#ECE8F3", "--fg-muted": "#A9A1BA", "--fg-subtle": "#9189A4", "--fg-inverse": "#1A1822",
  "--accent": "#E8A25C", "--accent-hover": "#F0B274", "--accent-muted": "#473422",
  "--bg-glass": "rgb(35 31 46 / 0.9)", "--bg-glass-end": "rgb(26 24 34 / 0.9)",
};

export const DUSK_MOD: Mod = { name: "dusk", title: "Dusk", kind: "theme", enabled: true, promoted: false, vars: DUSK };

const initial = (): { mods: Mod[]; theme: string } => ({
  mods: [
    { name: "nord", title: "Nord", kind: "theme", enabled: true, promoted: false, vars: NORD },
    { name: "solarized", title: "Solarized", kind: "theme", enabled: true, promoted: false, vars: SOLARIZED },
  ],
  theme: "dark",
});

let held = initial();
const listeners = new Set<() => void>();
let applied: string[] = [];

/** Overrides on the document root. A name outside the built-ins and the enabled mods falls back to Dark. */
function apply(): void {
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  applied.forEach((key) => root.style.removeProperty(key));
  const vars = held.theme === "light" ? LIGHT : (held.mods.find((one) => one.name === held.theme && one.enabled)?.vars ?? {});
  applied = Object.keys(vars);
  Object.entries(vars).forEach(([key, value]) => root.style.setProperty(key, value));
  root.dataset["theme"] = held.theme;
}

function set(next: typeof held): void {
  if (next.theme !== "dark" && next.theme !== "light" && !next.mods.some((one) => one.name === next.theme && one.enabled)) next = { ...next, theme: "dark" };
  held = next;
  apply();
  listeners.forEach((on) => on());
}

const subscribe = (on: () => void) => {
  listeners.add(on);
  return () => void listeners.delete(on);
};

export const useMods = () => useSyncExternalStore(subscribe, () => held);
export const chooseTheme = (theme: string) => set({ ...held, theme });
export const enableMod = (name: string, enabled: boolean) => set({ ...held, mods: held.mods.map((one) => (one.name === name ? { ...one, enabled } : one)) });
export const promoteMod = (name: string) => set({ ...held, mods: held.mods.map((one) => (one.name === name ? { ...one, promoted: true } : one)) });
export const installMod = (mod: Mod) => held.mods.some((one) => one.name === mod.name) || set({ ...held, mods: [...held.mods, mod] });
/** Back to the two mods a machine starts the mock with, Dark in force. */
export function resetMods(): void {
  held = initial();
  apply();
  listeners.forEach((on) => on());
}
