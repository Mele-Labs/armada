// The palette jumps to a Session: found by any word of its title, in any order, or by its id.

import { Palette } from "@armada/shell";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import "./styles/index.css";
import { carryOut, type PaletteHands } from "./palette";

const roots: { root: Root; host: HTMLElement }[] = [];

afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
});

const SESSIONS = [
  { id: "01b2b572-aaaa", label: "Palette search for sessions", aliases: ["01b2b572-aaaa", "s-01b2b572"] },
  { id: "9ca3b0d8-bbbb", label: "Fix the merge line", aliases: ["9ca3b0d8-bbbb", "s-9ca3b0d8"], repo: "Pocket", pull: 1899 },
];

function palette(openSession: PaletteHands["openSession"]) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({ root, host });
  const hands = { openSession } as PaletteHands;
  root.render(
    <Palette
      open
      onClose={() => {}}
      context="board"
      on={null}
      surfaces={[]}
      sessions={SESSIONS}
      settings={[]}
      onChoose={(choice) => carryOut(choice, hands)}
      onConfirmAct={() => {}}
    />,
  );
}

test("a word of the title finds the Session, wherever it sits", async () => {
  const open = vi.fn();
  palette(open);
  await userEvent.keyboard("sessions palette");
  await userEvent.click(page.getByRole("option", { name: /Palette search for sessions/ }));
  expect(open).toHaveBeenCalledWith("01b2b572-aaaa");
});

test("the id finds it, whole or as the address", async () => {
  const open = vi.fn();
  palette(open);
  await userEvent.keyboard("9ca3");
  await userEvent.click(page.getByRole("option", { name: /Fix the merge line/ }));
  expect(open).toHaveBeenCalledWith("9ca3b0d8-bbbb");
});

test("a Job is found by its pull request's number, which the row does not draw", async () => {
  const open = vi.fn();
  palette(open);
  await userEvent.keyboard("#1899");
  const row = page.getByRole("option", { name: /Fix the merge line/ });
  await expect.element(row).toBeVisible();
  expect(row.element().textContent).not.toContain("1899");
  expect(row.element().textContent).toContain("Pocket");
  await userEvent.click(row);
  expect(open).toHaveBeenCalledWith("9ca3b0d8-bbbb");
});
