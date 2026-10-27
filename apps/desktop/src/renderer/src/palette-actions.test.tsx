// The palette's second panel, pins and the empty state, as a person drives them from the keyboard: ⌘K on a row lists what
// can be done to it, a chord runs an act from the results or the panel, Stop asks first, Pin keeps the palette up, and
// Esc goes back with the words kept.

import { Palette, type PaletteChoice, type PaletteProps } from "@armada/shell";
import { createRoot, type Root } from "react-dom/client";
import { Check } from "lucide-react";
import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import "./styles/index.css";

const roots: { root: Root; host: HTMLElement }[] = [];

afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
});

const JOBS = [
  { id: "q1", label: "Cache key for the plan windows", aliases: ["q1"], repo: "armada", state: { icon: Check, said: "Working" }, pull: 1899 },
  { id: "pk1", label: "Migration notes for Pocket", aliases: ["pk1"], repo: "Pocket", state: { icon: Check, said: "Paused" }, paused: true },
  { id: "w1", label: "Warm the install icons", aliases: ["w1"], repo: "Pocket", pull: 212, without: ["merge-pull", "fork"] },
];

function palette(over: Partial<PaletteProps> = {}) {
  const chosen: PaletteChoice[] = [];
  const confirmed = vi.fn();
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({ root, host });
  root.render(
    <Palette
      open
      onClose={() => {}}
      context="board"
      on={null}
      surfaces={[]}
      sessions={JOBS}
      tasks={[{ id: "t1", label: "Tick a task from the palette", detail: "Palette · armada" }]}
      settings={[]}
      onChoose={(choice) => chosen.push(choice)}
      onConfirmOn={confirmed}
      {...over}
    />,
  );
  return { chosen, confirmed };
}

// The search takes focus once the palette has drawn: a key pressed before then lands nowhere, as the first
// letter of "cache key" did on a loaded runner.
const typeIn = async (words: string) => {
  await expect.element(page.getByRole("combobox")).toHaveFocus();
  await userEvent.keyboard(words);
};

const actsOfRow = async (typed: string) => {
  await typeIn(typed);
  await userEvent.keyboard("{Meta>}k{/Meta}");
};

test("⌘K on a Job lists what can be done to it, and a pull request adds its two acts", async () => {
  palette();
  await actsOfRow("cache key");
  for (const name of ["Open", "Pause", "Fork", "Open pull request", "Merge pull request", "Copy Job id", "Pin", "Stop"]) await expect.element(page.getByRole("option", { name, exact: true })).toBeVisible();
});

test("a Job with no pull request has none of the two, and a paused one offers Resume and not Pause", async () => {
  palette();
  await actsOfRow("migration notes");
  await expect.element(page.getByRole("option", { name: "Resume", exact: true })).toBeVisible();
  expect(page.getByRole("option", { name: "Pause", exact: true }).elements()).toHaveLength(0);
  expect(page.getByRole("option", { name: "Open pull request", exact: true }).elements()).toHaveLength(0);
});

test("an act the Job's draft has nothing behind is left out rather than drawn dead", async () => {
  palette();
  await actsOfRow("install icons");
  await expect.element(page.getByRole("option", { name: "Open pull request", exact: true })).toBeVisible();
  expect(page.getByRole("option", { name: "Merge pull request", exact: true }).elements()).toHaveLength(0);
  expect(page.getByRole("option", { name: "Fork", exact: true }).elements()).toHaveLength(0);
});

test("pressing an act in the panel hands it on with the row it was for", async () => {
  const { chosen } = palette();
  await actsOfRow("cache key");
  await userEvent.click(page.getByRole("option", { name: "Pause", exact: true }));
  expect(chosen).toContainEqual({ of: "on", target: "session:q1", action: "pause" });
});

test("an act's own chord runs it from the results, with the panel never opened", async () => {
  const { chosen } = palette();
  await typeIn("cache key");
  await userEvent.keyboard("{Meta>}{Shift>}p{/Shift}{/Meta}");
  expect(chosen).toContainEqual({ of: "on", target: "session:q1", action: "open-pull" });
});

test("Stop asks first: its chord goes to the confirmation and never to the choice", async () => {
  const { chosen, confirmed } = palette();
  await typeIn("cache key");
  await userEvent.keyboard("{Meta>}{Backspace}{/Meta}");
  await expect.poll(() => confirmed.mock.calls.length).toBe(1);
  expect(confirmed).toHaveBeenCalledWith("session:q1", "stop");
  expect(chosen.some((one) => one.of === "on" && one.action === "stop")).toBe(false);
});

test("Pin is chosen and the palette stays up, and a pinned row offers Unpin", async () => {
  const { chosen } = palette({ pinned: ["session:q1"] });
  await actsOfRow("cache key");
  await expect.element(page.getByRole("option", { name: "Unpin", exact: true })).toBeVisible();
  await userEvent.click(page.getByRole("option", { name: "Unpin", exact: true }));
  expect(chosen).toContainEqual({ of: "on", target: "session:q1", action: "unpin" });
  await expect.element(page.getByRole("combobox")).toBeVisible();
});

test("Esc goes back to the results with the words kept", async () => {
  palette();
  await actsOfRow("cache key");
  await expect.element(page.getByRole("option", { name: "Fork", exact: true })).toBeVisible();
  await userEvent.keyboard("{Escape}");
  expect(page.getByRole("option", { name: "Fork", exact: true }).elements()).toHaveLength(0);
  await expect.element(page.getByRole("combobox")).toHaveValue("cache key");
});

test("a task has Tick among its acts", async () => {
  const { chosen } = palette();
  await actsOfRow("tick a task");
  await userEvent.click(page.getByRole("option", { name: "Tick", exact: true }));
  expect(chosen).toContainEqual({ of: "on", target: "task:t1", action: "tick" });
});

test("with nothing typed, pinned come first and then recent, and a task is shown at rest only when pinned", async () => {
  palette({ pinned: ["task:t1", "session:q1"], recent: ["session:pk1", "session:q1"] });
  const rows = page.getByRole("option");
  await expect.element(rows.first()).toHaveTextContent("Tick a task from the palette");
  const names = rows.elements().map((one) => one.textContent ?? "");
  expect(names).toHaveLength(3);
  expect(names[1]).toContain("Cache key");
  expect(names[2]).toContain("Migration notes");
});

test("with nothing pinned or recent, a task is not listed at rest and is found by typing", async () => {
  palette({ opens: "everything" });
  expect(page.getByRole("option", { name: /Tick a task/ }).elements()).toHaveLength(0);
  await typeIn("tick a task");
  await expect.element(page.getByRole("option", { name: /Tick a task/ })).toBeVisible();
});

test("an alias the person gave a row finds it", async () => {
  palette({ aliases: { "session:q1": "ckey" } });
  await typeIn("ckey");
  await expect.element(page.getByRole("option", { name: /Cache key/ })).toBeVisible();
  expect(page.getByRole("option", { name: /Warm the install/ }).elements()).toHaveLength(0);
});
