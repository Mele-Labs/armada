// The token sets the Mods mock re-skins with. Dark is the shipped set in `packages/tokens/src/colors.css`
// (so it overrides nothing); Light is a new baseline drawn here only; Nord and Solarized stand in for
// `theme.css` files a mod ships. A theme overrides custom properties and nothing else.

export type ThemeId = "dark" | "light" | "nord" | "solarized" | "dusk";
export type Vars = Record<string, string>;

export const THEMES: Record<ThemeId, { name: string; vars: Vars }> = {
  dark: { name: "Dark", vars: {} },
  light: {
    name: "Light",
    vars: {
      "--bg-base": "#F6F8FA", "--bg-sunken": "#EDF0F3", "--bg-raised": "#FFFFFF", "--bg-overlay": "#FFFFFF", "--bg-hover": "#E8ECF0",
      "--border-subtle": "#E3E8ED", "--border-default": "#D0D7DE", "--border-strong": "#AEB8C3",
      "--fg-default": "#18202A", "--fg-muted": "#4D5B6B", "--fg-subtle": "#5F6D7D", "--fg-inverse": "#FFFFFF",
      "--accent": "#1F6FB2", "--accent-hover": "#185C95", "--accent-muted": "#D6E7F5",
    },
  },
  nord: {
    name: "Nord",
    vars: {
      "--bg-base": "#2E3440", "--bg-sunken": "#272C36", "--bg-raised": "#3B4252", "--bg-overlay": "#434C5E", "--bg-hover": "#4C566A",
      "--border-subtle": "#3F4759", "--border-default": "#4C566A", "--border-strong": "#616E88",
      "--fg-default": "#ECEFF4", "--fg-muted": "#C2CBDA", "--fg-subtle": "#A9B4C8", "--fg-inverse": "#2E3440",
      "--accent": "#88C0D0", "--accent-hover": "#8FBCBB", "--accent-muted": "#3E5663",
    },
  },
  solarized: {
    name: "Solarized",
    vars: {
      "--bg-base": "#FDF6E3", "--bg-sunken": "#EEE8D5", "--bg-raised": "#FFFBEF", "--bg-overlay": "#FFFBEF", "--bg-hover": "#EEE8D5",
      "--border-subtle": "#EAE3CC", "--border-default": "#D8D0B8", "--border-strong": "#93A1A1",
      "--fg-default": "#073642", "--fg-muted": "#44636C", "--fg-subtle": "#586E75", "--fg-inverse": "#FDF6E3",
      "--accent": "#268BD2", "--accent-hover": "#1D70AB", "--accent-muted": "#DCE9EC",
    },
  },
  dusk: {
    name: "Dusk",
    vars: {
      "--bg-base": "#1A1822", "--bg-sunken": "#13111A", "--bg-raised": "#231F2E", "--bg-overlay": "#2B2638", "--bg-hover": "#332D42",
      "--border-subtle": "#2A2536", "--border-default": "#383149", "--border-strong": "#4D4463",
      "--fg-default": "#ECE8F3", "--fg-muted": "#A9A1BA", "--fg-subtle": "#9189A4", "--fg-inverse": "#1A1822",
      "--accent": "#E8A25C", "--accent-hover": "#F0B274", "--accent-muted": "#473422",
    },
  },
};

/** Where a mod lives on disk, and what it holds. */
export const MODS_DIR = "~/Library/Application Support/Armada/mods";
