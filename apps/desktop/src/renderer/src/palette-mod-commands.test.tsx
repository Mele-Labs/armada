// The rows a Commands mod adds to the palette, as a person meets them: found by name or alias, asking for each blank in a step of
// its own that names the command, and finishing with the act's verb. The mod list and the `commands` Fleet checked are served by a
// fake window the way the real one serves them; the rows are not handed to the palette.

import { createRoot, type Root } from "react-dom/client";
import { useState } from "react";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { CommandsFile, ModChecked } from "@armada/protocol";

import "./styles/index.css";
import { NOTHING_YET, type BridgeState } from "../../shared/bridge";
import { PaletteMount, type PaletteMountProps } from "./PaletteMount";
import { PaletteFixturesFrom } from "./palette-fixtures";
import { forgetPaletteMemory, PALETTE_KEY, seedPaletteMemory } from "./palette-memory";
import { setKnownRows } from "./palette-known";
import { forgetPanel, hostsPanel, usePanel } from "./shared-panel/store";

const roots: { root: Root; host: HTMLElement }[] = [];

const FILE: CommandsFile = {
  command: [
    { name: "Security pass on a pull request", aliases: ["pentest"], hotkey: "⌥⌘S", prompt: "Review {pr} for {focus} problems", blanks: ["pr", "focus"] },
    { name: "Open pull request on the forge", url: "https://forge.example/pull/{pr}?q={focus}", blanks: ["pr", "focus"] },
    { name: "Test a pull request", shell: "pnpm test --pr {pr}", blanks: ["pr"] },
    { name: "Audit dependencies", shell: "pnpm audit" },
    { name: "Triage the board", chain: ["nav:reviews", "nav:projects"] },
    { name: "Triage with a gap", chain: ["nav:reviews", "cmd:team-commands:nowhere", "nav:projects"] },
  ],
};
const WENT_BAD: CommandsFile = { command: [{ name: "Stale pass", prompt: "Nope" }] };
const OTHER: CommandsFile = { command: [{ name: "Disabled pass", prompt: "Nope" }] };

const MODS = [
  { name: "team-commands", kind: "commands", enabled: true, valid: true },
  { name: "switched-off", kind: "commands", enabled: false, valid: true },
  { name: "broken", kind: "commands", enabled: true, valid: false },
  { name: "went-bad", kind: "commands", enabled: true, valid: true },
  { name: "a-theme", kind: "theme", enabled: true, valid: true },
].map((one) => ({ ...one, origin: "local", version: "1", changed_at: "2026-10-27T09:00:00Z" }));

const PULLS = [
  { repo: "armada", number: 1899, title: "Plan windows", url: "https://forge.example/armada/pull/1899" },
  { repo: "armada", number: 1901, title: "Palette acts", url: "https://forge.example/armada/pull/1901" },
];

function Panel() {
  const { open } = usePanel();
  return <output aria-label="Side panel">{open === undefined ? "" : JSON.stringify(open)}</output>;
}

function Host(props: PaletteMountProps) {
  const [open, setOpen] = useState(props.open);
  return (
    <PaletteFixturesFrom fixtures={{ pulls: PULLS, issues: [], commands: [] }}>
      <PaletteMount {...props} open={open} onOpen={() => setOpen(true)} onClose={() => setOpen(false)} />
      <Panel />
    </PaletteFixturesFrom>
  );
}

let release: (() => void) | undefined;
beforeEach(() => {
  release = hostsPanel();
});
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
  release?.();
  forgetPanel();
  setKnownRows([]);
  window.localStorage.removeItem(PALETTE_KEY);
  forgetPaletteMemory();
  Object.defineProperty(window, "armada", { value: undefined, configurable: true });
});

function mount(open = true, picked: string | null = null) {
  const state = { ...NOTHING_YET, mods: { mods: MODS } } as BridgeState;
  const validateMod = vi.fn(async (name: string): Promise<ModChecked | null> => {
    // A mod whose files stopped passing still has the commands it last read.
    if (name === "went-bad") return { name, valid: false, problems: [{ message: "gone wrong" }] as unknown as ModChecked["problems"], commands: WENT_BAD };
    return { name, valid: true, problems: [], commands: name === "team-commands" ? FILE : OTHER };
  });
  const openLink = vi.fn(async () => ({ ok: true }));
  Object.defineProperty(window, "armada", { value: { syncState: async () => ({ full: state, seq: 1 }), subscribeChanges: () => () => undefined, validateMod, openLink, getReviewLists: async () => ({ ok: false }) }, configurable: true });
  const onDispatch = vi.fn();
  const surface = vi.fn();
  const open_ = vi.fn((_address: string) => false);
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({ root, host });
  const props: PaletteMountProps = {
    open,
    onClose: () => {},
    shownStudio: null,
    surfaces: [
      { id: "reviews", label: "Reviews" },
      { id: "projects", label: "Projects" },
    ],
    checkoutRunSheet: NOTHING_YET.checkoutRunSheet,
    failing: null,
    live: true,
    repositories: [
      { root: "/Users/me/armada", label: "armada" },
      { root: "/Users/me/pocket", label: "Pocket" },
    ] as PaletteMountProps["repositories"],
    picked,
    acts: { surface } as unknown as PaletteMountProps["acts"],
    dispatch: { kind: "session", onDispatch },
    linkRoutes: { kindOf: () => null, open: open_ },
  };
  root.render(<Host {...props} />);
  return { validateMod, onDispatch, surface, openLink: open_, openedBrowser: openLink };
}

const dialog = () => page.getByRole("dialog", { name: "Command palette" });
const row = (name: string) => page.getByRole("option", { name: new RegExp(`^${name}`) });
const panel = () => page.getByLabelText("Side panel");

test("rows come only from Commands mods that are on and valid, and nothing else is asked of Fleet", async () => {
  const { validateMod } = mount();
  await userEvent.keyboard("pass");
  await expect.element(row("Security pass on a pull request")).toBeVisible();
  expect(page.getByRole("option", { name: /Disabled pass|Stale pass/ }).elements()).toHaveLength(0);
  // A mod that no longer passes adds nothing, and the three that are not both on and valid are never read.
  await vi.waitFor(() => expect(validateMod.mock.calls.map((call) => call[0]).sort()).toEqual(["team-commands", "went-bad"]));
});

test("a row is found by its alias, which is not drawn", async () => {
  mount();
  await userEvent.keyboard("pentest");
  await expect.element(row("Security pass on a pull request")).toBeVisible();
  await expect.element(row("Security pass on a pull request")).not.toHaveTextContent("pentest");
});

test("a row with blanks asks them in order, each step naming the command, with Next until the last and then the verb", async () => {
  const { onDispatch } = mount();
  await userEvent.keyboard("security");
  await userEvent.keyboard("{Enter}");
  // The command's name leads the field and the step's title is its placeholder, not a line over the list.
  await expect.element(page.getByRole("combobox", { name: "Pull request" })).toHaveAttribute("placeholder", "Pull request");
  await expect.element(dialog()).toHaveTextContent("Security pass on a pull request");
  await expect.element(dialog()).toHaveTextContent(/Next\s*↵/);
  await userEvent.click(page.getByRole("option", { name: /Palette acts/ }));
  await expect.element(page.getByRole("combobox", { name: "Focus" })).toHaveAttribute("placeholder", "Focus");
  await expect.element(dialog()).toHaveTextContent("Security pass on a pull request");
  await expect.element(dialog()).toHaveTextContent(/Start a Job\s*↵/);
  await userEvent.click(page.getByRole("combobox", { name: "Focus" }));
  await userEvent.keyboard("auth");
  await userEvent.keyboard("{Enter}");
  await vi.waitFor(() => expect(onDispatch).toHaveBeenCalledWith("Review #1901 for auth problems", "session"));
});

test("an address fills with the number and the word encoded, and finishes with Open", async () => {
  const { openLink } = mount();
  await userEvent.keyboard("open pull request on");
  await userEvent.click(row("Open pull request on the forge"));
  await userEvent.click(page.getByRole("option", { name: /Plan windows/ }));
  await expect.element(dialog()).toHaveTextContent(/Open\s*↵/);
  await userEvent.click(page.getByRole("combobox", { name: "Focus" }));
  await userEvent.keyboard("a b&c");
  await userEvent.keyboard("{Enter}");
  await vi.waitFor(() => expect(openLink).toHaveBeenCalledWith("https://forge.example/pull/1899?q=a%20b%26c"));
});

test("a shell line takes a pull request as its bare number, asks for a repository where none is picked, and runs in the side panel", async () => {
  mount();
  await userEvent.keyboard("test a pull");
  await userEvent.click(row("Test a pull request"));
  await userEvent.click(page.getByRole("option", { name: /Plan windows/ }));
  await expect.element(page.getByRole("combobox", { name: "Repository" })).toBeVisible();
  await expect.element(dialog()).toHaveTextContent(/Run\s*↵/);
  await userEvent.click(page.getByRole("option", { name: /Pocket/ }));
  await expect.element(panel()).toHaveTextContent('"kind":"run"');
  await expect.element(panel()).toHaveTextContent('"command":"pnpm test --pr 1899"');
  await expect.element(panel()).toHaveTextContent('"dir":"/Users/me/pocket"');
  await expect.element(panel()).toHaveTextContent('"repository":"Pocket"');
});

test("a shell line with no blank runs in the repository picked, without a step", async () => {
  mount(true, "/Users/me/armada");
  await userEvent.keyboard("audit dep");
  await userEvent.click(row("Audit dependencies"));
  await expect.element(panel()).toHaveTextContent('"command":"pnpm audit"');
  await expect.element(panel()).toHaveTextContent('"dir":"/Users/me/armada"');
});

test("a chain runs the rows it names in order", async () => {
  const { surface } = mount();
  await userEvent.keyboard("triage the board");
  await userEvent.click(row("Triage the board"));
  await vi.waitFor(() => expect(surface.mock.calls.map((call) => call[0])).toEqual(["reviews", "projects"]));
});

test("a chain naming a row that is not there stops, and says which", async () => {
  const { surface } = mount();
  const told: string[] = [];
  const hear = (event: Event) => told.push((event as CustomEvent<{ sentence: string }>).detail.sentence);
  window.addEventListener("armada:tell", hear);
  await userEvent.keyboard("triage with a gap");
  await userEvent.click(row("Triage with a gap"));
  await vi.waitFor(() => expect(told).toEqual(["Triage with a gap stopped: no row cmd:team-commands:nowhere"]));
  window.removeEventListener("armada:tell", hear);
  expect(surface.mock.calls.map((call) => call[0])).toEqual(["reviews"]);
});

test("a key the file names runs a row that has blanks by opening the palette on its first step", async () => {
  mount(false);
  // The rows load behind the window; the key is allowed once they have.
  await vi.waitFor(async () => {
    await userEvent.keyboard("{Meta>}{Alt>}s{/Alt}{/Meta}");
    await expect.element(page.getByRole("combobox", { name: "Pull request" })).toBeVisible({ timeout: 300 });
  });
  await expect.element(dialog()).toHaveTextContent("Security pass on a pull request");
  await expect.element(dialog()).toHaveTextContent(/Next\s*↵/);
});

test("a key he recorded for another row wins over the file's key, which then runs nothing", async () => {
  seedPaletteMemory({ hotkeys: { "cmd:team-commands:audit-dependencies": "⌥⌘S" } });
  mount(false, "/Users/me/armada");
  await vi.waitFor(async () => {
    await userEvent.keyboard("{Meta>}{Alt>}s{/Alt}{/Meta}");
    await expect.element(panel()).toHaveTextContent('"command":"pnpm audit"', { timeout: 300 });
  });
  expect(page.getByRole("dialog", { name: "Command palette" }).elements()).toHaveLength(0);
});
