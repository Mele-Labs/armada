// The palette picks a repository as the title row's picker does: a block of the repositories and an
// All row, the picked one marked, and a choice hands its root (or `null` for All) to `pickRepository`.

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

const REPOSITORIES = [
  { root: "/r/armada", label: "armada" },
  { root: "/r/ynap", label: "ynap" },
];

function palette(picked: string | null, pickRepository: PaletteHands["pickRepository"], repositories = REPOSITORIES) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({ root, host });
  const hands = { pickRepository } as PaletteHands;
  root.render(
    <Palette
      open
      onClose={() => {}}
      context="board"
      on={null}
      surfaces={[]}
      jobs={[]}
      settings={[]}
      repositories={repositories}
      picked={picked}
      onChoose={(choice) => carryOut(choice, null, hands)}
      onConfirmAct={() => {}}
    />,
  );
}

test("choosing a repository in the palette picks it", async () => {
  const pick = vi.fn();
  palette(null, pick);
  await userEvent.keyboard("ynap");
  await userEvent.click(page.getByRole("option", { name: /ynap/ }));
  expect(pick).toHaveBeenCalledWith("/r/ynap");
});

test("All repositories clears the pick, and the picked repository is marked", async () => {
  const pick = vi.fn();
  palette("/r/armada", pick);
  await expect.element(page.getByRole("option", { name: /armada.*picked/ })).toBeVisible();
  await userEvent.keyboard("all repos");
  await userEvent.click(page.getByRole("option", { name: /All repositories/ }));
  expect(pick).toHaveBeenCalledWith(null);
});

test("with no repository to pick between, the palette draws no Repositories block", async () => {
  palette(null, vi.fn(), []);
  await userEvent.keyboard("all repos");
  await expect.element(page.getByRole("option", { name: /All repositories/ })).not.toBeInTheDocument();
});
