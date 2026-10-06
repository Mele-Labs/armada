// A read-in that comes back with nothing to place lands one Note off its Finding saying so, through
// `App`. The owner, 2 Oct 2026: "What about a note that extends from finding that just "Nothing was
// found that could be pulled into the studio"".

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mountApp, type Mounted, readingNothing } from "@armada/desktop/mock";
import { NOTHING_FOUND, NOTHING_STUDIO_NAME } from "@armada/studios/fake";

let mounted: { app: Mounted; host: HTMLElement } | null = null;

afterEach(() => {
  mounted?.app.unmount();
  mounted?.host.remove();
  mounted = null;
});

test("a read-in that comes back with nothing lands one Note off its Finding saying so", async () => {
  const fleet = readingNothing();
  const host = document.createElement("div");
  host.id = "root";
  document.body.append(host);
  mounted = { app: mountApp(fleet.scenario, host), host };
  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await page.getByRole("cell", { name: NOTHING_STUDIO_NAME, exact: true }).click();
  await page.getByRole("button", { name: "Continue" }).click();

  const link = page.getByRole("group", { name: /^Link: / });
  await expect.element(link).toBeVisible();
  link.element().focus({ focusVisible: true } as FocusOptions);
  await userEvent.keyboard("{Enter}");
  await page.getByRole("group", { name: "What is picked" }).getByRole("button", { name: "Read in", exact: true }).click();
  await page.getByRole("dialog").getByRole("button", { name: "Read in", exact: true }).click();

  await expect.element(page.getByRole("group", { name: `Note: ${NOTHING_FOUND}` })).toBeVisible();
  const studio = fleet.studios()[0]!;
  const [zone, finding, note] = studio.nodes.slice(1);
  expect(studio.nodes.map((node) => node.kind)).toEqual(["link", "zone", "finding", "note"]);
  expect(note!.within).toBe(zone!.id);
  expect(studio.edges.some((edge) => edge.from === finding!.id && edge.to === note!.id && edge.kind === "produced")).toBe(true);
});
