// An Epic's wave read off the wire alone, through `App` (#1692, 23.14): the
// strip names each pass with its plan's line, and the graph draws the order
// each Job waits in from its own Board row — no draft, no `get_job` per child.

import { expect, test, describe } from "vitest";
import { page } from "vitest/browser";

import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

describe("a wave off the wire", () => {
  const wave = () => page.getByRole("region", { name: "The wave" }).first();
  const waveCard = (title: string) => wave().getByRole("button", { name: new RegExp(`^${title}, `) });

  test("the strip names each pass with the first sentence of its plan's approach", async () => {
    mount("epic/wave-off-the-wire");
    await expect.element(wave().getByRole("tab", { name: "Wave 1 · The seam as one Job" })).toBeVisible();
    await expect
      .element(wave().getByRole("tab", { name: "Wave 2 · The seam first, then every surface that reads it" }))
      .toHaveAttribute("aria-selected", "true");
  });

  test("the graph draws each Job behind what its row says it waits on", async () => {
    mount("epic/wave-off-the-wire");
    await expect.element(waveCard("Drop the second error shape")).toBeVisible();
    await expect
      .element(page.getByLabelText("Drop the second error shape waits on Say which half refused"))
      .toBeInTheDocument();
    await expect
      .element(page.getByLabelText("Name the fault in the toast waits on Refuse an unknown code at the seam"))
      .toBeInTheDocument();
  });

  test("pressing the first pass draws its Jobs, with their own edge", async () => {
    mount("epic/wave-off-the-wire");
    await wave().getByRole("tab", { name: "Wave 1 · The seam as one Job" }).click();
    await expect.element(waveCard("Handle every refusal at the seam")).toBeVisible();
    await expect
      .element(page.getByLabelText("Handle every refusal at the seam waits on List every code Fleet refuses with"))
      .toBeInTheDocument();
    expect(waveCard("Say which half refused").query()).toBeNull();
  });
});
