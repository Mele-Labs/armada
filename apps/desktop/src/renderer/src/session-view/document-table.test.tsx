// A table in a Job's document reads row by row: the header is shaded, each row has a rule under it, alternate rows are faintly shaded, and a
// thin rule runs between columns. Read as a person sees it, from computed styles on the page the mock Job opens, through `App`.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "../mock/testing";
import { walkThrough } from "../mock/walk";
import { WALKS } from "../mock/walks";
import { LEASE_DOC_TITLE } from "../mock/document-pages";

unmountAfterEach();

async function opened(): Promise<HTMLElement> {
  const script = WALKS.get("job-document")!;
  mount(script.scenario);
  await onScreen();
  await walkThrough(script.steps.slice(0, script.steps.findIndex((one) => one.say.startsWith("A press opens the page")) + 1));
  const article = page.getByRole("article", { name: LEASE_DOC_TITLE, exact: true });
  await expect.element(article).toBeVisible();
  return article.element() as HTMLElement;
}

// A token as the colour the browser resolves it to, so it compares with what a computed style returns.
function colourOf(token: string): string {
  const probe = document.body.appendChild(document.createElement("span"));
  probe.style.color = `var(${token})`;
  const resolved = getComputedStyle(probe).color;
  probe.remove();
  return resolved;
}

// The colour behind an element: its own background, or the nearest ancestor's that paints one.
function groundOf(el: Element | null): string {
  for (let at = el; at; at = at.parentElement) {
    const bg = getComputedStyle(at).backgroundColor;
    if (bg !== "rgba(0, 0, 0, 0)" && bg !== "transparent") return bg;
  }
  return "rgba(0, 0, 0, 0)";
}

test("a table's header is shaded, each row is ruled, alternate rows are shaded and the columns are ruled apart", async () => {
  const article = await opened();
  const table = article.querySelector("table")!;
  const head = table.querySelector("thead th")!;
  const rows = [...table.querySelectorAll("tbody tr")];
  expect(rows.length, "the mock page's table has rows enough to show zebra").toBeGreaterThanOrEqual(4);

  const [first, second, third] = rows.map((row) => getComputedStyle(row).backgroundColor);
  const shade = getComputedStyle(head).backgroundColor;
  expect.soft(shade, "the header is shaded").not.toBe("rgba(0, 0, 0, 0)");
  expect.soft(shade, "the header's shade differs from the page's ground").not.toBe(groundOf(table.parentElement));
  expect.soft(second, "the second body row is shaded against the first").not.toBe(first);
  expect.soft(third, "the third body row is plain like the first").toBe(first);

  const cells = rows[0]!.querySelectorAll("td");
  expect.soft(getComputedStyle(cells[0]!).borderBottomColor, "a body row has a visible rule under it").toBe(colourOf("--border-default"));
  expect.soft(getComputedStyle(cells[1]!).borderInlineStartColor, "a thin rule runs between columns").toBe(colourOf("--border-subtle"));
  expect.soft(getComputedStyle(cells[1]!).borderInlineStartWidth, "the rule between columns is drawn").not.toBe("0px");
  expect.soft(getComputedStyle(cells[0]!).borderInlineStartWidth, "the first column has no rule at its edge").toBe("0px");
}, 60_000);

test("the row under the pointer is tinted, and a noted row stays stronger than it", async () => {
  const article = await opened();
  const rows = [...article.querySelectorAll("tbody tr")];
  const plain = getComputedStyle(rows[2]!).backgroundColor;
  await page.elementLocator(rows[2]!).hover();
  const hovered = getComputedStyle(rows[2]!).backgroundColor;
  expect.soft(hovered, "the row under the pointer is tinted").not.toBe(plain);
  expect.soft(hovered, "the tint is the row hover surface").toBe(colourOf("--surface-row-hover"));
  rows[2]!.setAttribute("data-noted", "");
  expect.soft(getComputedStyle(rows[2]!).backgroundColor, "a noted row wins over the hover under it").toBe(colourOf("--accent-muted"));
}, 60_000);
