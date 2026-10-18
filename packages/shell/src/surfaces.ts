// Where a person can go in Bridge, and the digit that reaches it.
//
// **One roster, read by everything that navigates.** The title bar's tabs and System strip draw from it and the palette lists
// it. A second copy is how a place ends up in one and not the other, or answers a key the other spells differently.
//
// **A digit is a place in the Cockpit window's tab bar**: `⌘1` to `⌘6` are Cockpit, Jobs, Sessions, Studios, Retros and System,
// and Settings is `⌘,`. Every other destination is reached from the System strip, the title bar's menu, a link on the deck or
// the palette by name, and takes no key.

import { useEffect, useRef } from "react";
import {
  BookOpen,
  ChartNoAxesColumn,
  Box,
  Briefcase,
  FileCog,
  Eye,
  Rewind,
  HardDrive,
  LayoutDashboard,
  Merge,
  Puzzle,
  Presentation,
  Settings as SettingsIcon,
  ShieldCheck,
  Stethoscope,
  SquareTerminal,
  UserCheck,
  Workflow,
} from "lucide-react";
import { digitKeyFor, isPressed, keyFor, pressedDigit } from "@armada/components";

import type { PaletteSurface } from "./Palette";

/** The ids Bridge routes on. Here, so a typo cannot be a dead row. */
export const SURFACE = {
  overview: "overview",
  manifest: "manifest",
  worktrees: "worktrees",
  kit: "kit",
  settings: "settings",
  studios: "studios",
  doctor: "doctor",
  guides: "guides",
  mergeLine: "merge-line",
  lessons: "lessons",
  workflows: "workflows",
  checks: "checks",
  sessions: "sessions",
  mods: "mods",
  jobs: "jobs",
  account: "account",
  reviews: "reviews",
  usage: "usage",
} as const;

type SurfaceId = (typeof SURFACE)[keyof typeof SURFACE];

/** The digit each surface answers to: the tab it is. A place with none is reached by name, and Settings has `⌘,`. */
const DIGIT: Readonly<Partial<Record<SurfaceId, number>>> = {
  [SURFACE.overview]: 1,
  [SURFACE.jobs]: 2,
  [SURFACE.sessions]: 3,
  [SURFACE.studios]: 4,
  [SURFACE.lessons]: 5,
};

/** What reaches a surface, spelled as this person has `deck_tabs`: `⌘3` unless Settings → Keyboard moved the row. */
function digitOf(id: SurfaceId): string | undefined {
  const at = DIGIT[id];
  return at === undefined ? undefined : digitKeyFor("deck_tabs", at);
}

/**
 * The places that exist, with what finds each one.
 *
 * **What is not built is not in here.** A row a person presses and gets nothing
 * from is worse than one that is absent, which is the contract's own rule about
 * a registered binding nothing answers. So the digits skip: `⌘3` is owed to
 * Alerts and reaches nothing today.
 *
 * `held disk` is an alias because that was the word on the Overview menu's
 * item for this screen until the menu went, and a person who learned it should
 * not have to learn a second.
 */
export const SURFACES: readonly PaletteSurface[] = [
  {
    id: SURFACE.overview,
    label: "Cockpit",
    get shortcut() {
      return digitOf(SURFACE.overview);
    },
    // No alias, for the same reason Manifest carries none: this is the first
    // surface built at this name, so there is no earlier word to keep.
    icon: LayoutDashboard,
  },
  {
    id: SURFACE.jobs,
    label: "Jobs",
    // No key; reached from the System strip, a link on the deck and by name.
    get shortcut() {
      return digitOf(SURFACE.jobs);
    },
    aliases: ["job"],
    // `box` is the registry's Job, at 12px on a chip; the tab bar draws it at 16px. For the owner to rule on.
    icon: Box,
  },
  {
    id: SURFACE.studios,
    label: "Studios",
    get shortcut() {
      return digitOf(SURFACE.studios);
    },
    // No alias, Manifest's reason: the first surface built at this name.
    icon: Presentation,
  },
  {
    id: SURFACE.doctor,
    label: "Doctor",
    shortcut: digitOf(SURFACE.doctor),
    // What a person looking for it will type. "health" and "status" are the
    // words for the question it answers; `armada doctor` is the verb (#99).
    aliases: ["health", "status", "system health"],
    icon: Stethoscope,
  },
  {
    id: SURFACE.manifest,
    label: "Manifest",
    get shortcut() {
      return digitOf(SURFACE.manifest);
    },
    // No alias. The rule above is that one is for a place a person already
    // knows by another word, and nothing in Bridge has ever reached this
    // surface — there is no earlier word for it to keep.
    icon: FileCog,
  },
  {
    id: SURFACE.worktrees,
    label: "Worktree Slots",
    get shortcut() {
      return digitOf(SURFACE.worktrees);
    },
    aliases: ["cleanup", "held worktrees", "disk", "held disk"],
    icon: HardDrive,
  },
  {
    id: SURFACE.kit,
    label: "Kit",
    get shortcut() {
      return digitOf(SURFACE.kit);
    },
    // "MCP" and "servers" are what a person looking for this will type: Kit
    // is Armada's word for the set, and the thing they came to connect has
    // its own name in every other tool they use.
    aliases: ["mcp", "servers", "mcp servers"],
    icon: Briefcase,
  },
  {
    id: SURFACE.settings,
    label: "Settings",
    get shortcut() {
      return keyFor("open_settings");
    },
    // No alias: `fleet_settings` is the palette's own row, in its own
    // section, and it names an id rather than a word somebody already knows.
    icon: SettingsIcon,
  },
  {
    id: SURFACE.mods,
    label: "Mods",
    get shortcut() {
      return digitOf(SURFACE.mods);
    },
    // No alias: the first surface built at this name.
    icon: Puzzle,
  },
  {
    id: SURFACE.guides,
    label: "Guides",
    get shortcut() {
      return digitOf(SURFACE.guides);
    },
    // What a person looking for this will type. "Help" is the word every other
    // application uses for the place explanations live, and "guide" singular is
    // what somebody types who met one card and wants the rest.
    aliases: ["help", "guide", "explain"],
    icon: BookOpen,
  },
  {
    id: SURFACE.mergeLine,
    label: "Merge line",
    // No key; reached from the System strip, a link on the deck and by name.
    get shortcut() {
      return digitOf(SURFACE.mergeLine);
    },
    // `armada land` is the word a person already runs to read it.
    aliases: ["land"],
    icon: Merge,
  },
  {
    id: SURFACE.lessons,
    label: "Retros",
    // No key; reached from the System strip, a link on the deck and by name.
    get shortcut() {
      return digitOf(SURFACE.lessons);
    },
    // What each item is, in `docs/concepts/retro.md`'s word.
    aliases: ["retro"],
    icon: Rewind,
  },
  {
    id: SURFACE.reviews,
    label: "Reviews",
    // No key: the registry's `deck_tabs` spells six digits, and a seventh is the owner's to bind.
    aliases: ["review", "pull requests", "pr review"],
    // `eye` is Review's.
    icon: Eye,
  },
  {
    id: SURFACE.workflows,
    label: "Workflows",
    // No key; reached from the System strip, a link on the deck and by name.
    get shortcut() {
      return digitOf(SURFACE.workflows);
    },
    aliases: ["workflow", "create workflow"],
    icon: Workflow,
  },
  {
    id: SURFACE.checks,
    label: "Checks",
    // No key; reached from the System strip, a link on the deck and by name.
    get shortcut() {
      return digitOf(SURFACE.checks);
    },
    aliases: ["check", "checks run"],
    // A sketch: `shield-check` is the registry's Check family. A Navigation glyph is the owner's to mint.
    icon: ShieldCheck,
  },
  {
    id: SURFACE.sessions,
    label: "Jobs",
    // No key; reached from the System strip, a link on the deck and by name.
    get shortcut() {
      return digitOf(SURFACE.sessions);
    },
    aliases: ["job", "new job", "session", "sessions", "new session"],
    icon: SquareTerminal,
  },
  {
    id: SURFACE.account,
    label: "Account",
    // Past the ninth, so no key; reached from the title row's menu, from Settings, and by name.
    get shortcut() {
      return digitOf(SURFACE.account);
    },
    aliases: ["sign in", "sign out", "profile", "profiles", "login"],
    icon: UserCheck,
  },
  {
    id: SURFACE.usage,
    label: "Usage",
    // No key and not on the strip yet: reached by name.
    aliases: ["tokens", "burn", "spend", "plan usage"],
    // A sketch: `chart-no-axes-column` is not in the registry. A Navigation glyph is the owner's to mint.
    icon: ChartNoAxesColumn,
  },
];

/**
 * The window's six tabs, and the surface each opens. These tabs are how a person moves about, and `⌘1` to `⌘6` reach them.
 *
 * System has no surface of its own: it opens the first of the strip's pages, Kit, and the strip under the
 * title row lists them all.
 */
export const DECK_TABS = [
  { id: "cockpit", label: "Cockpit", surface: SURFACE.overview },
  { id: "jobs", label: "Jobs", surface: SURFACE.jobs },
  { id: "sessions", label: "Jobs", surface: SURFACE.sessions },
  { id: "studios", label: "Studios", surface: SURFACE.studios },
  { id: "retros", label: "Retros", surface: SURFACE.lessons },
  { id: "reviews", label: "Reviews", surface: SURFACE.reviews },
  { id: "system", label: "System", surface: SURFACE.kit },
] as const;

/** The tabs a digit reaches. Reviews takes none: `deck_tabs` binds `⌘1` to `⌘6`, so the digits skip it and System keeps `⌘6`. */
const DIGIT_TABS = DECK_TABS.filter((tab) => tab.id !== "reviews");

export type DeckTabId = (typeof DECK_TABS)[number]["id"];

/**
 * The pages under System, in the order the strip draws them. **The names are the roster's**, read from
 * `SURFACES`, so a page renamed there is renamed here.
 */
export const SYSTEM_PAGES: readonly { surface: string; label: string }[] = [
  SURFACE.kit,
  SURFACE.mods,
  SURFACE.settings,
  SURFACE.guides,
  SURFACE.workflows,
].map((surface) => ({ surface, label: SURFACES.find((one) => one.id === surface)?.label ?? surface }));

/** The places a layout may name: the tabs that are a surface, then the System strip's pages the layout reaches. */
export const PLACES: readonly string[] = [
  SURFACE.overview,
  SURFACE.jobs,
  SURFACE.sessions,
  SURFACE.kit,
  SURFACE.mods,
  SURFACE.settings,
  SURFACE.guides,
  SURFACE.workflows,
  SURFACE.studios,
  SURFACE.lessons,
  SURFACE.reviews,
];

/** The tab a surface is under, or `undefined` where the Cockpit window places none (Manifest, the merge line). */
export function deckTabOf(showing: string): DeckTabId | undefined {
  const own = DECK_TABS.find((tab) => tab.id !== "system" && tab.surface === showing);
  if (own !== undefined) return own.id;
  return SYSTEM_PAGES.some((page) => page.surface === showing) ? "system" : undefined;
}

/** What `⌘<digit>` reaches: the tab's surface for `⌘1` to `⌘6`. */
export function surfaceOfDigit(digit: number): string | undefined {
  return DIGIT_TABS[digit - 1]?.surface;
}

/**
 * `⌘1` to `⌘6`, bound as `deck_tabs` has them, and Settings on `open_settings`.
 *
 * **A modified key, so a focused field does not suppress it.** `⌘K` is bound the same way and for the same reason: the
 * contextual tier is suppressed while a text input holds focus precisely so that typing cannot act, and the whole point of the
 * modifier is that it still works from inside one.
 */
export function useSurfaceKeys(onSurface: (surfaceId: string) => void): void {
  // **Bound once, and the handler is read through a ref.** The app rebuilds its `goTo` every render, and a listener
  // re-registered on every render is one that has to be right about removal every time.
  const latest = useRef(onSurface);
  latest.current = onSurface;

  useEffect(() => {
    function pressed(event: KeyboardEvent): void {
      if (isPressed("open_settings", event)) {
        event.preventDefault();
        latest.current(SURFACE.settings);
        return;
      }
      const digit = pressedDigit("deck_tabs", event);
      if (digit === null) return;
      const surface = surfaceOfDigit(digit);
      if (surface === undefined) return;
      event.preventDefault();
      latest.current(surface);
    }
    window.addEventListener("keydown", pressed);
    return () => window.removeEventListener("keydown", pressed);
  }, []);
}
