// A read-in that comes back with nothing to place says so in a toast, through `App`. The owner,
// 2 Oct 2026: "there should be a toast notification or something saying nothing was found."

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { NOTHING_FOUND } from "../nothing-found";
import { mountApp, type Mounted } from "./mount";
import { NOTHING_STUDIO_NAME, readingNothing } from "./studio-read-nothing";

let mounted: { app: Mounted; host: HTMLElement } | null = null;

afterEach(() => {
  mounted?.app.unmount();
  mounted?.host.remove();
  mounted = null;
});

const said = () => page.getByRole("status").filter({ hasText: NOTHING_FOUND });

test("a read-in that comes back with nothing says so in a toast, once its scout has answered", async () => {
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

  await expect.element(page.getByRole("group", { name: /^Finding: Read in/ })).toBeVisible();
  await expect.element(said()).toBeVisible();
  const studio = fleet.studios()[0]!;
  expect(studio.nodes.map((node) => node.kind)).toEqual(["link", "zone", "finding"]);
});
