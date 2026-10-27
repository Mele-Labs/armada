// The command palette, wired: what goes in it, in what order, and what a
// chosen row means.
//
// **A shell holds a screen; it is not one** — and the palette is the same
// whatever surface is open, which is exactly what makes it a layer. It also
// spans both screens: the acts on the board and the acts on one job read whole
// are different sets under one artifact, so a palette owned by either screen
// would be a second one the moment the other needed it.
//
// # The contents, in the order the contract fixes
//
// Acts on the current context, navigation, sessions, settings. The context block
// is **titled with what it acts on**, so a palette that will act on something
// says which something before the first row is read.
//
// # Everything here is the registry's, and nothing is decided here
//
// The verb, the glyph, the binding and whether an act is destructive all come
// from `actions.ts`, which transcribes `crates/core-model/domain/actions.toml`.
// This file decides which of them a context offers — and even that is the
// registry's `scope` column read through `actsIn`. What it adds is one thing
// the registry cannot know: whether *this app, right now* can reach the act.
//
// **`dormant` is that answer and the caller supplies it.** A row nothing
// answers is drawn dimmed with the reason beside its binding rather than left
// out, because the palette is the discovery surface: a binding a person cannot
// find is a binding they never learn, and one they press to no effect is worse
// than a row that says why.
//
// # ⌘K is bound once, here
//
// `useCommandPalette` owns the binding. It is a modified key, so it does not
// go through the contextual tier's suppression — the whole point of the
// modifier is that it works while a text field has focus, which is when a
// person most wants out.

import { useEffect, useState } from "react";
import { Ban, Check, CircleDot, Copy, ExternalLink, FileTerminal, GitFork, GitMerge, GitPullRequest, Globe, ListTodo, MessageSquarePlus, Pause, Play, Plus, Route, Send } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { actsIn, ALIASES, globalActs, CommandPalette, isPressed, keyFor } from "@armada/components";
import type { Action, ActionContext, PaletteAction, PaletteDispatch, PaletteLink, PalettePath, PaletteEntry, PaletteMeta, PaletteSection, PaletteStep } from "@armada/components";

/**
 * A Session as a search result: found by any word of its title, or by its id. **A Job says where it is and how it
 * stands**: its repository, its state as a registry glyph with the tooltip that names it, and the pull request it holds.
 */
export type PaletteSession = {
  id: string;
  label: string;
  aliases: readonly string[];
  repo?: string;
  state?: { icon: LucideIcon; said: string; pulsing?: true };
  paused?: boolean;
  pull?: number;
};

/** An open pull request, an issue: found by its number or any word of its title. */
export type PaletteTicket = { number: number; title: string; repo: string };

/** One of his tasks, with the project it sits in. */
export type PaletteTask = { id: string; title: string; project: string; repo: string };

/**
 * A row a Commands mod brings: opening a URL, running a shell line, a chain of palette steps, or a prompt that
 * waits on a blank. **A `prompt` with a blank asks for it in a second step** before it dispatches.
 */
export type PaletteCommand = { id: string; label: string; kind: "url" | "shell" | "chain" | "prompt"; value?: string; alias?: string; hotkey?: string };

/** What the page showing offers: its title heads the block, and its rows are the acts of that page. */
export type PalettePage = { title: string; rows: readonly (PaletteBoardRow & { shortcut?: string })[] };

/** What the empty query shows. A setting. */
export type PaletteOpens = "pinned" | "page" | "everything";

export const PALETTE_OPENS: readonly { id: PaletteOpens; label: string }[] = [
  { id: "pinned", label: "Pinned and recent" },
  { id: "page", label: "This page first" },
  { id: "everything", label: "Everything" },
];

/**
 * A destination in Bridge, with the digit that reaches it where it has one.
 *
 * **The digit is optional and that is a gap, named rather than filled.** The
 * contract binds `⌘1–⌘9` to *Bridge surfaces in rail order*, and Helm is on
 * `⌘J`, which toggles the dock rather than taking a digit. So a digit is a
 * position in the rail and nothing else. A destination that is not in the rail
 * has no position, and inventing one for it would take a digit the rail owes
 * to a surface named in `docs/concepts/bridge.md` — which changes what a
 * published binding says, and is not a wiring decision.
 *
 * So a destination without a rail row is reached by name in here and by no
 * digit. That is a smaller thing than it looks: the palette is the discovery
 * surface, and the rule it exists to keep is that no action lives outside it.
 */
export type PaletteSurface = {
  id: string;
  label: string;
  shortcut?: string;
  /**
   * Words that find it, never drawn. Only for a place a person already knows
   * by another word — the control they reached it by before it was in here.
   */
  aliases?: readonly string[];
  icon: LucideIcon;
};

/**
 * One thing the Manifest surface can run — a Check, a Command, or a server.
 *
 * **A row per entry, not one row spelling "run something".** Journey 9's own
 * table says the palette carries one entry per Check and Command, for the
 * reason the Board's filters are five rows rather than the registry's single
 * `state_filter`: the set is what a person is choosing from, and a row that
 * named none of them would leave every Check reachable only by finding the
 * surface first.
 *
 * **Its `run` line is the value**, drawn in mono to the right the way a
 * setting's current value is — so a palette that will run `pnpm -C
 * apps/desktop typecheck` says so before it is chosen.
 */
export type PaletteRunnable = {
  /** The entry id the Manifest surface uses — `check:format`, `command:fmt`. */
  id: string;
  /** The Manifest's own name for it. */
  label: string;
  /** The `run` or `serve` line, exactly as declared. */
  value: string;
};

/**
 * A setting, which states its current value in mono right of the label where
 * it has one.
 *
 * **`value` is optional, and a row without one is not a display.** Fleet
 * settings is the first row this section carries: choosing it opens the
 * sheet rather than stating a field, because the section is where the app's
 * standing configuration lives, whether that configuration is read here or
 * changed behind its own screen.
 */
export type PaletteSetting = { id: string; label: string; value?: string; aliases?: readonly string[] };

/**
 * One of the Board's state filters, with the digit that sets it.
 *
 * **They appear as rows and the registry's single `state_filter` does not.**
 * That row spells its binding `1–5` over five tabs, for the reason
 * `deck_tabs` spells `⌘1–⌘7` over its destinations — the rule is tab
 * order rather than the digits. A palette drawing that one row would name no
 * filter and set none, which would leave `1`–`5` the one part of the app
 * learnable only by finding the tabs.
 */
export type PaletteFilter = { id: string; label: string; shortcut: string };

/**
 * A row about the window as a whole, with no registry row and no rail row: Reported. Drawn under
 * Navigation beside the global acts. The palette is the only place Reported is drawn — the owner's note of 1 Oct 2026.
 */
export type PaletteBoardRow = { id: string; label: string; icon?: LucideIcon; destructive?: boolean; dormant?: string };

/** What a chosen row was. The section it came from decides what happens. */
export type PaletteChoice =
  | { of: "act"; id: string }
  | { of: "session"; id: string }
  | { of: "surface"; id: string }
  | { of: "run"; id: string }
  | { of: "setting"; id: string }
  | { of: "filter"; id: string }
  | { of: "board"; id: string }
  | { of: "page"; id: string }
  | { of: "pull"; id: string }
  | { of: "issue"; id: string }
  | { of: "task"; id: string }
  | { of: "command"; id: string }
  /** An action on a row: the row's id as the palette drew it, and the action's. */
  | { of: "on"; target: string; action: string }
  /** `root` is `null` for All repositories. */
  | { of: "repository"; root: string | null };

/** A repository the palette can pick, as the title row's picker names it. */
export type PaletteRepository = { root: string; label: string };

export type PaletteProps = {
  open: boolean;
  onClose: () => void;
  /** Where the person is standing: the board, or a Studio. */
  context: ActionContext;
  /**
   * What the acts act on, as a person reads it. `null` where nothing is
   * focused — and then the block says so rather than being titled with
   * nothing, because a heading that names no object over rows that need one is
   * the ambiguity the title exists to remove.
   */
  on: string | null;
  surfaces: readonly PaletteSurface[];
  /** The Board's state filters, where the surface has them. */
  filters?: readonly PaletteFilter[];
  /**
   * What the Manifest surface can run, where it is the surface showing.
   *
   * **Empty everywhere else, and that is the whole of the scoping.** Choosing
   * one runs it in the checkout, which is an act on a surface rather than on a
   * Session — so a row offered from the Board would be a run started from a
   * surface that cannot show its output.
   */
  runnables?: readonly PaletteRunnable[];
  /** Rows about the window as a whole. Absent draws none. */
  board?: readonly PaletteBoardRow[];
  /** Sessions to jump to. Absent draws none. */
  sessions?: readonly PaletteSession[];
  settings: readonly PaletteSetting[];
  /**
   * The repositories to pick between, and which one is picked (`null` is All). Absent or empty draws
   * no block: with nothing to choose between, a row would only be a no-op.
   */
  repositories?: readonly PaletteRepository[];
  picked?: string | null;
  /** Why an act cannot be chosen here, by action id. Absent means it can. */
  dormant?: Readonly<Record<string, string | undefined>>;
  /**
   * Acts left out here rather than drawn dimmed, by action id. A binding the
   * registry marks `unbuilt` is drawn whatever this says, since its reason
   * holds everywhere and not just here.
   */
  absent?: readonly string[];
  /**
   * A row was chosen. **Destructive acts do not come through here** — they go
   * to `onConfirmAct`, because every destructive action confirms even from the
   * keyboard, and the palette stays open behind the confirmation. Absent where
   * no act that confirms is offered.
   */
  onChoose: (choice: PaletteChoice) => void;
  onConfirmAct?: (actionId: string) => void;
  /** The one bar's dispatch row: what was typed, sent as a Session or an Autopilot. */
  dispatch?: PaletteDispatch;
  /** A typed path, offered as a file to open. */
  path?: PalettePath;
  /** A typed link to a pull request or an issue, offered as the page to open. */
  link?: PaletteLink;
  /** The title bar's field, which the palette opens over. */
  anchor?: string;
  /** What the page showing offers, first in the results. Absent draws no block. */
  page?: PalettePage;
  pulls?: readonly PaletteTicket[];
  issues?: readonly PaletteTicket[];
  tasks?: readonly PaletteTask[];
  /** The rows a Commands mod brings. */
  commands?: readonly PaletteCommand[];
  /** The second step a command with a blank opens, or none. */
  commandStep?: (commandId: string) => PaletteStep | undefined;
  /** What was pinned and what was opened last, as the palette drew them: `session:…`, `pull:…`. */
  pinned?: readonly string[];
  recent?: readonly string[];
  opens?: PaletteOpens;
  /** A destructive action on a row was chosen: it confirms over the palette before it runs. */
  onConfirmOn?: (target: string, action: string) => void;
};

/** The section ids. Their order is the contract's contents order. */
const CONTEXT = "context";
const NAVIGATION = "navigation";
const RUN = "run";
const SESSIONS = "sessions";
const SETTINGS = "settings";
const PAGE = "page";
const PINNED = "pinned";
const RECENT = "recent";
const COMMANDS = "commands";
const PULLS = "pulls";
const ISSUES = "issues";
const TASKS = "tasks";
const REPOSITORIES = "repositories";

/** The row for All repositories. A root is an absolute path, so this id cannot be one. */
const ALL_REPOSITORIES = "repo-all";

/** The registry row the Board's filters stand in for. */
const STATE_FILTER = "state_filter";

/** What the block is titled where nothing is focused. */
const NOTHING_FOCUSED = "Nothing focused";

/**
 * Whether ⌘K is down, and the state it opens.
 *
 * **Bound on `window` and never suppressed by a focused field.** The
 * contextual tier is suppressed while a text input holds focus so that typing
 * "axe" cannot approve, kill and open something; a modified key is a different
 * tier and that is the whole separation. `Esc` closes it, and the palette
 * itself owns that key while it is up.
 */
export function useCommandPalette(): {
  open: boolean;
  onOpen: () => void;
  onClose: () => void;
} {
  const [open, setOpen] = useState(false);

  useEffect(() => {
    function pressed(event: KeyboardEvent): void {
      // The palette marks a ⌘K it used for a row's actions, and that press is not a toggle.
      if (event.defaultPrevented || !isPressed("command_palette", event)) return;
      event.preventDefault();
      // Toggled, not opened. ⌘K twice is how a person closes something they
      // opened by reflex, and `Esc` is not what they reach for while their
      // hand is still on the modifier.
      setOpen((was) => !was);
    }
    window.addEventListener("keydown", pressed);
    return () => window.removeEventListener("keydown", pressed);
  }, []);

  return { open, onOpen: () => setOpen(true), onClose: () => setOpen(false) };
}

export function Palette({
  open,
  onClose,
  context,
  on,
  surfaces,
  filters = [],
  runnables = [],
  board = [],
  sessions = [],
  settings,
  repositories = [],
  picked = null,
  dormant = {},
  absent = [],
  onChoose,
  onConfirmAct,
  dispatch,
  path,
  link,
  anchor,
  page,
  pulls = [],
  issues = [],
  tasks = [],
  commands = [],
  commandStep,
  pinned = [],
  recent = [],
  opens = "pinned",
  onConfirmOn,
}: PaletteProps) {
  const sections: PaletteSection[] = [
    // The page showing heads the results, titled with it.
    ...(page === undefined || page.rows.length === 0 ? [] : [{ id: PAGE, title: page.title }]),
    { id: CONTEXT, title: on ?? NOTHING_FOCUSED },
    { id: NAVIGATION, title: "Navigation" },
    ...(commands.length === 0 ? [] : [{ id: COMMANDS, title: "Commands" }]),
    // **Below Navigation and above Sessions, and it is a fifth block.** The
    // contract's contents order names four, and none of them is this: a Check
    // is not an act on what is focused, not a place, not a Session and
    // not a setting. It sits where it does because it is the one block scoped
    // to the surface showing — nearer the acts than the index of everything.
    ...(runnables.length === 0 ? [] : [{ id: RUN, title: "Run in the checkout" }]),
    // Where the title row's picker is, from the keyboard: the same pick, over the same list.
    ...(repositories.length === 0 ? [] : [{ id: REPOSITORIES, title: "Repositories" }]),
    ...(sessions.length === 0 ? [] : [{ id: SESSIONS, title: "Jobs" }]),
    ...(pulls.length === 0 ? [] : [{ id: PULLS, title: "Pull requests" }]),
    ...(issues.length === 0 ? [] : [{ id: ISSUES, title: "Issues" }]),
    ...(tasks.length === 0 ? [] : [{ id: TASKS, title: "Tasks" }]),
    { id: SETTINGS, title: "Settings" },
  ];

  const entries: PaletteEntry[] = [
    ...(page?.rows ?? []).map((row) => ({
      id: `page:${row.id}`,
      section: PAGE,
      label: row.label,
      ...(row.icon === undefined ? {} : { icon: row.icon }),
      ...(row.shortcut === undefined ? {} : { shortcut: row.shortcut }),
      ...(row.destructive === true ? { destructive: true } : {}),
      ...(row.dormant === undefined ? {} : { dormant: row.dormant }),
    })),
    ...actsIn(context).flatMap((action) =>
      action.unbuilt === null && absent.includes(action.id)
        ? []
        : action.id === STATE_FILTER && filters.length > 0
        ? // Expanded into its five, for the reason the roster expands
          // `deck_tabs` into its places. The one registry row spells a
          // range over a set whose order is the rule; only the Board knows
          // what `3` currently narrows to.
          filters.map((filter) => ({
            id: `tab:${filter.id}`,
            section: CONTEXT,
            label: filter.label,
            shortcut: filter.shortcut,
            ...(action.icon === null ? {} : { icon: action.icon }),
          }))
        : [entryOf(action, CONTEXT, dormant)],
    ),
    // The roster as it actually is, one row per destination. The registry keeps
    // `deck_tabs` as a single row on purpose — its rule is the tab's place in
    // the bar rather than the digits — so the roster is what expands it, and a
    // palette built from that row alone would name no destination at all.
    //
    // **Places with no tab come through here too**, which is what
    // the optional digit above is for. They are destinations by every other
    // measure — a title, a glyph in the registry's Navigation group, a screen
    // that replaces the panel — and leaving them out is the exact failure this
    // section is here to prevent.
    ...surfaces.map((surface) => ({
      id: `nav:${surface.id}`,
      section: NAVIGATION,
      label: surface.label,
      icon: surface.icon,
      ...(surface.shortcut === undefined ? {} : { shortcut: surface.shortcut }),
      ...(surface.aliases === undefined ? {} : { aliases: surface.aliases }),
    })),
    ...globalActs()
      .filter((action) => action.unbuilt !== null || !absent.includes(action.id))
      .map((action) => entryOf(action, NAVIGATION, dormant)),
    ...board.map((row) => ({
      id: `board:${row.id}`,
      section: NAVIGATION,
      label: row.label,
      ...(row.icon === undefined ? {} : { icon: row.icon }),
      ...(row.destructive === true ? { destructive: true } : {}),
      ...(row.dormant === undefined ? {} : { dormant: row.dormant }),
    })),
    ...commands.map((command) => ({
      id: `cmd:${command.id}`,
      section: COMMANDS,
      label: command.label,
      icon: COMMAND_GLYPH[command.kind],
      ...(command.hotkey === undefined ? {} : { shortcut: command.hotkey }),
      ...(command.value === undefined ? {} : { value: command.value }),
      ...(command.alias === undefined ? {} : { meta: { alias: command.alias } }),
    })),
    ...runnables.map((runnable) => ({
      id: `run:${runnable.id}`,
      section: RUN,
      label: runnable.label,
      // The `run` line, mono and to the right — a setting's own treatment,
      // because it answers the same question: what this row currently is.
      value: runnable.value,
    })),
    ...(repositories.length === 0
      ? []
      : [
          { id: ALL_REPOSITORIES, section: REPOSITORIES, label: "All repositories", aliases: ["all"], ...(picked === null ? { value: "picked" } : {}) },
          ...repositories.map((one) => ({
            id: `repo:${one.root}`,
            section: REPOSITORIES,
            label: one.label,
            ...(picked === one.root ? { value: "picked" } : {}),
          })),
        ]),
    ...sessions.map((session) => ({ id: `session:${session.id}`, section: SESSIONS, label: session.label, aliases: session.aliases, meta: metaOfJob(session) })),
    ...pulls.map((one) => ({ id: `pull:${one.repo}:${one.number}`, section: PULLS, label: one.title, icon: GitPullRequest, meta: { repo: one.repo, number: `#${one.number}` } })),
    ...issues.map((one) => ({ id: `issue:${one.repo}:${one.number}`, section: ISSUES, label: one.title, icon: CircleDot, meta: { repo: one.repo, number: `#${one.number}` } })),
    ...tasks.map((one) => ({ id: `task:${one.id}`, section: TASKS, label: one.title, icon: ListTodo, meta: { repo: one.repo }, aliases: [one.project] })),
    ...settings.map((setting) => ({
      id: `set:${setting.id}`,
      section: SETTINGS,
      label: setting.label,
      ...(setting.value === undefined ? {} : { value: setting.value }),
      ...(setting.aliases === undefined ? {} : { aliases: setting.aliases }),
    })),
  ];

  // With nothing typed: Pinned, then Recent; or this page first; or everything, as the setting says.
  const byId = new Map(entries.map((entry) => [entry.id, entry]));
  const kept = pinned.flatMap((id) => byId.get(id) ?? []);
  const lately = recent.filter((id) => !pinned.includes(id)).flatMap((id) => byId.get(id) ?? []);
  const home =
    opens === "pinned" && kept.length + lately.length > 0
      ? {
          sections: [...(kept.length === 0 ? [] : [{ id: PINNED, title: "Pinned" }]), ...(lately.length === 0 ? [] : [{ id: RECENT, title: "Recent" }])],
          entries: [...kept.map((entry) => ({ ...entry, section: PINNED })), ...lately.map((entry) => ({ ...entry, section: RECENT }))],
        }
      : opens === "everything"
      ? { sections: sections.filter((one) => one.id !== PAGE), entries: entries.filter((entry) => entry.section !== PAGE) }
      : undefined;

  return (
    <CommandPalette
      open={open}
      sections={sections}
      entries={entries}
      {...(home === undefined ? {} : { home })}
      actionsOf={(entry) => actionsOf(entry, { sessions, pinned })}
      stepOf={(entry) => (entry.id.startsWith("cmd:") ? commandStep?.(bare(entry.id)) : undefined)}
      onAct={(entry, action) => onChoose({ of: "on", target: entry.id, action: action.id })}
      searched={searchedSentence({ sessions: sessions.length, settings: settings.length, runnables: runnables.length, repositories: repositories.length, tickets: pulls.length + issues.length + tasks.length, commands: commands.length })}
      onClose={onClose}
      {...(dispatch === undefined ? {} : { dispatch })}
      {...(path === undefined ? {} : { path })}
      {...(link === undefined ? {} : { link })}
      {...(anchor === undefined ? {} : { anchor })}
      onSelect={(entry) => onChoose(choiceOf(entry.id))}
      // A destructive board row opens its own confirmation, so it is a choice.
      onConfirm={(entry) => {
        // An action on a row comes as the row's id, a bar and the action's.
        const [target, action] = entry.id.split("|");
        if (action !== undefined && target !== undefined) onConfirmOn?.(target, action);
        else if (entry.id.startsWith("board:")) onChoose(choiceOf(entry.id));
        else onConfirmAct?.(bare(entry.id));
      }}
    />
  );
}

/** One registry row as a palette row. Nothing is re-spelled on the way. */
function entryOf(
  action: Action,
  section: string,
  dormant: Readonly<Record<string, string | undefined>>,
): PaletteEntry {
  // Two kinds of dormancy and one rendering: a binding nothing answers
  // anywhere, which the registry names with an issue, and an act this app
  // cannot reach at this moment, which only the app knows.
  const why =
    action.unbuilt === null ? dormant[action.id] : `not built · ${action.unbuilt}`;
  return {
    id: `act:${action.id}`,
    section,
    label: action.verb,
    shortcut: keyFor(action.id),
    ...(action.icon === null ? {} : { icon: action.icon }),
    ...(ALIASES[action.id] === undefined ? {} : { aliases: ALIASES[action.id] }),
    ...(action.destructive ? { destructive: true } : {}),
    ...(why === undefined ? {} : { dormant: why }),
  };
}

/** The id without its section prefix. */
function bare(id: string): string {
  return id.slice(id.indexOf(":") + 1);
}

/** What a chosen row was, read off the prefix its section gave it. */
function choiceOf(id: string): PaletteChoice {
  const rest = bare(id);
  if (id === ALL_REPOSITORIES) return { of: "repository", root: null };
  if (id.startsWith("repo:")) return { of: "repository", root: rest };
  if (id.startsWith("act:")) return { of: "act", id: rest };
  if (id.startsWith("nav:")) return { of: "surface", id: rest };
  if (id.startsWith("session:")) return { of: "session", id: rest };
  if (id.startsWith("tab:")) return { of: "filter", id: rest };
  if (id.startsWith("run:")) return { of: "run", id: rest };
  if (id.startsWith("board:")) return { of: "board", id: rest };
  if (id.startsWith("page:")) return { of: "page", id: rest };
  if (id.startsWith("pull:")) return { of: "pull", id: rest };
  if (id.startsWith("issue:")) return { of: "issue", id: rest };
  if (id.startsWith("task:")) return { of: "task", id: rest };
  if (id.startsWith("cmd:")) return { of: "command", id: rest };
  return { of: "setting", id: rest };
}

/**
 * What was searched, for the state where nothing matched.
 *
 * **It names the extent of the index rather than apologising**, and there is
 * no did-you-mean: the palette is the discovery surface, so the useful answer
 * to a miss is what was looked through. The counts are the real ones — a
 * sentence claiming settings on a build that indexes none would be the labelled
 * blank this app refuses everywhere else.
 */
function searchedSentence(held: { sessions: number; settings: number; runnables: number; repositories: number; tickets: number; commands: number }): string {
  const found = [
    "every act available here",
    "every place in Bridge",
    held.commands === 0 ? null : "every command",
    held.runnables === 0 ? null : "every Check and Command this Manifest declares",
    held.repositories === 0 ? null : "every repository",
    held.sessions === 0 ? null : "every live job",
    held.tickets === 0 ? null : "every open pull request, issue and task",
    held.settings === 0 ? null : "every setting",
  ].filter((part): part is string => part !== null);
  return `${found.slice(0, -1).join(", ")} and ${found[found.length - 1]}`;
}

/** A Job's row says where it is, how it stands and the pull request it holds. */
function metaOfJob(session: PaletteSession): PaletteMeta {
  return {
    ...(session.repo === undefined ? {} : { repo: session.repo }),
    ...(session.pull === undefined ? {} : { number: `#${session.pull}` }),
    ...(session.state === undefined ? {} : { state: session.state }),
  };
}

const COMMAND_GLYPH: Record<PaletteCommand["kind"], LucideIcon> = { url: Globe, shell: FileTerminal, chain: Route, prompt: MessageSquarePlus };

/**
 * What ⌘K opens on a row. **Pin is on every one**, since a pin is kept by the host and a row can be pinned from
 * whichever list it was found in. Stop confirms: it is the one act here that ends work.
 */
function actionsOf(entry: PaletteEntry, held: { sessions: readonly PaletteSession[]; pinned: readonly string[] }): readonly PaletteAction[] {
  const pin: PaletteAction = held.pinned.includes(entry.id) ? { id: "unpin", label: "Unpin", stays: true } : { id: "pin", label: "Pin", stays: true };
  if (entry.id.startsWith("session:")) {
    const job = held.sessions.find((one) => `session:${one.id}` === entry.id);
    return [
      { id: "open", label: "Open" },
      job?.paused === true ? { id: "resume", label: "Resume", icon: Play } : { id: "pause", label: "Pause", icon: Pause },
      { id: "stop", label: "Stop", icon: Ban, destructive: true },
      { id: "fork", label: "Fork", icon: GitFork },
      ...(job?.pull === undefined ? [] : [{ id: "open-pull", label: "Open pull request", icon: GitPullRequest }, { id: "merge-pull", label: "Merge pull request", icon: GitMerge }]),
      { id: "copy-id", label: "Copy Job id", icon: Copy },
      pin,
    ];
  }
  if (entry.id.startsWith("pull:")) {
    return [{ id: "open", label: "Open in Reviews", icon: GitPullRequest }, { id: "github", label: "Open on GitHub", icon: ExternalLink }, { id: "start-job", label: "Start a Job on it", icon: Plus }, pin];
  }
  if (entry.id.startsWith("issue:")) return [{ id: "open", label: "Open" }, { id: "start-job", label: "Start a Job on it", icon: Plus }, pin];
  if (entry.id.startsWith("task:")) return [{ id: "open", label: "Open" }, { id: "dispatch", label: "Dispatch", icon: Send }, { id: "tick", label: "Tick", icon: Check }, pin];
  if (entry.id.startsWith("cmd:")) return [{ id: "run", label: "Run" }, pin];
  return [];
}
