// The places bar and the title bar's menu: where Studios, the Logbook, Settings and Doctor are, and the keys that reach them. The
// owner, 15 Oct 2026: studios and the logbook (with the retro items on its Retros tab) in the main bar, Doctor in the menu at the top right.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const selected = () => document.querySelector('nav[aria-label="Places"] [role="tab"][aria-selected="true"]')?.textContent;

// The digits count the tabs the bar draws, in its order: ⌘1 the Cockpit, ⌘2 to ⌘4 the three after it.
test("The places bar reads Cockpit, Studios, Logbook, System, and ⌘2 to ⌘4 open the last three, ⌘1 the Cockpit", async () => {
  mount("cockpit-ship-flightdeck");
  await onScreen();

  const names = [...document.querySelectorAll('nav[aria-label="Places"] [role="tab"]')].map((one) => one.textContent);
  expect(names).toEqual(["Cockpit", "Studios", "Logbook", "System"]);
  const wanted = ["Studios", "Logbook", "System", "Cockpit", "System", "Cockpit"];
  for (const [at, name] of ["2", "3", "4", "1", "4", "1"].entries()) {
    await userEvent.keyboard(`{Meta>}${name}{/Meta}`);
    await expect.poll(selected, { message: `⌘${name}` }).toContain(wanted[at]!);
  }
});

test("Settings opens on ⌘, and ⌘3 opens the Logbook", async () => {
  mount("default");
  await onScreen();

  await userEvent.keyboard("{Meta>},{/Meta}");
  await expect.element(page.getByRole("navigation", { name: "System" }).getByRole("button", { name: /^Limits/ })).toHaveAttribute("aria-current", "page");
  await userEvent.keyboard("{Meta>}3{/Meta}");
  await expect.poll(selected).toContain("Logbook");
});

test("Doctor is a row of the title bar's menu and not an entry of the System sidebar", async () => {
  mount("default");
  await onScreen();

  await userEvent.click(page.getByRole("button", { name: "Menu" }));
  await userEvent.click(page.getByRole("dialog", { name: "Menu" }).getByRole("button", { name: "Doctor" }));
  await expect.element(page.getByRole("region", { name: "Doctor" })).toBeVisible();

  await userEvent.click(page.getByRole("navigation", { name: "Places" }).getByRole("tab", { name: "System" }));
  expect(page.getByRole("navigation", { name: "System" }).getByRole("button", { name: "Doctor" }).query()).toBeNull();
});

// The owner, 23 Oct 2026: the places and the search hang from the window's top edge as one notch, not a floating bar.
test("The places and the search sit on one notch that meets the window's top edge, square above and round below", async () => {
  mount("cockpit-ship-flightdeck");
  await onScreen();

  const notch = document.querySelector<HTMLElement>(".armada-title-bar__center")!;
  const style = getComputedStyle(notch);
  expect(notch.getBoundingClientRect().top).toBe(0);
  expect([style.borderTopLeftRadius, style.borderTopRightRadius]).toEqual(["0px", "0px"]);
  expect(parseFloat(style.borderBottomLeftRadius)).toBeGreaterThan(0);
  expect(parseFloat(style.borderBottomRightRadius)).toBeGreaterThan(0);
});

test("The tab strip and the search trigger draw no ground, edge or shadow of their own on the notch", async () => {
  mount("cockpit-ship-flightdeck");
  await onScreen();

  const drawn = [document.querySelector<HTMLElement>('nav[aria-label="Places"] .armada-tabs')!, document.querySelector<HTMLElement>(".armada-title-bar__search")!];
  for (const one of drawn) {
    const style = getComputedStyle(one);
    expect(style.backgroundColor, one.className).toBe("rgba(0, 0, 0, 0)");
    expect(style.borderTopColor, one.className).toBe("rgba(0, 0, 0, 0)");
    expect(style.boxShadow, one.className).toBe("none");
  }
});
