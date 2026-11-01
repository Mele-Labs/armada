// A layout fix, read off the real layout in the browser: the Jobs list's filter chip lies clear of the first row. Over the `job-browser` scenario.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, toPlace, unmountAfterEach } from "./testing";

unmountAfterEach();

const box = (element: Element) => element.getBoundingClientRect();

test("the Jobs list's filter chip does not lie over the first row", async () => {
  await page.viewport(1440, 900);
  mount("job-browser");
  await onScreen();
  await toPlace("Cockpit");
  const list = page.getByRole("region", { name: "Jobs", exact: true });
  await list.hover();
  const chip = list.element().querySelector(".deck-band")!;
  const row = list.element().querySelector(".deck-row")!;
  await expect.poll(() => getComputedStyle(chip).opacity).toBe("1");
  expect(box(chip).bottom).toBeLessThanOrEqual(box(row).top);
});
