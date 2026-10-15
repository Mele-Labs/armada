// The places bar and the title bar's menu: where Studios, Retros, Settings and Doctor are, and the keys that reach them. The
// owner, 15 Oct 2026: studios and retros in the main bar, Doctor in the menu at the top right.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const selected = () => document.querySelector('nav[aria-label="Places"] [role="tab"][aria-selected="true"]')?.textContent;

// A scenario whose Fleet serves Sessions: the bar leaves a tab off where nothing serves it.
test("The places bar reads Cockpit, Jobs, Sessions, Studios, Retros, System and ⌘1 to ⌘6 open them in that order", async () => {
  mount("cockpit-ship-flightdeck");
  await onScreen();

  const names = [...document.querySelectorAll('nav[aria-label="Places"] [role="tab"]')].map((one) => one.textContent);
  expect(names).toEqual(["Cockpit", "Jobs", "Sessions", "Studios", "Retros", "System"]);
  const wanted = ["Jobs", "Sessions", "Studios", "Retros", "System", "Cockpit"];
  for (const [at, name] of ["2", "3", "4", "5", "6", "1"].entries()) {
    await userEvent.keyboard(`{Meta>}${name}{/Meta}`);
    await expect.poll(selected, { message: `⌘${name}` }).toContain(wanted[at]!);
  }
});

test("Settings opens on ⌘, and ⌘5 opens Retros", async () => {
  mount("every-state");
  await onScreen();

  await userEvent.keyboard("{Meta>},{/Meta}");
  await expect.element(page.getByRole("navigation", { name: "System" }).getByRole("tab", { name: "Settings" })).toHaveAttribute("aria-selected", "true");
  await userEvent.keyboard("{Meta>}5{/Meta}");
  await expect.poll(selected).toContain("Retros");
});

test("Doctor is a row of the title bar's menu and not a page of the System strip", async () => {
  mount("every-state");
  await onScreen();

  await userEvent.click(page.getByRole("button", { name: "Menu" }));
  await userEvent.click(page.getByRole("dialog", { name: "Menu" }).getByRole("button", { name: "Doctor" }));
  await expect.element(page.getByRole("region", { name: "Doctor" })).toBeVisible();

  await userEvent.click(page.getByRole("navigation", { name: "Places" }).getByRole("tab", { name: "System" }));
  expect(page.getByRole("navigation", { name: "System" }).getByRole("tab", { name: "Doctor" }).query()).toBeNull();
});
