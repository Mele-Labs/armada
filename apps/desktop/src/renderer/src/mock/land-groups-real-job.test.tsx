// The finished-Job board's groups, on the owner's Job 2 as `GET /jobs/2`
// served it.
//
// **The rows overlapped there and on no hand-made mock** (owner, 1 Oct 2026):
// the commit is forty characters on the wire, its track was sized to it with
// nothing allowed to wrap, so the row ran past its card and the Checks were
// squeezed to nothing and drew over the commit. The mock's commits were seven.
//
// **A geometry test, so it lives here and not in a story**: what overlaps is
// decided by the width the window gives the card.

import { afterEach, describe, expect, test } from "vitest";
import { page } from "vitest/browser";

import { job2Landed } from "./job-2-landed";
import { onJob } from "./scenario";
import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

afterEach(async () => {
  await page.viewport(1440, 900);
});

const COMMIT = "daae5427cfd2c705d4cbcf719121e6b5e1b285d7";

type Box = { text: string; left: number; right: number; top: number; bottom: number };

/**
 * Where each run of words in a row is drawn — **the words, not their cell**: a
 * cell squeezed to nothing has a box of nothing, and its words still draw
 * past it. A cell that clips its overflow is where its words stop.
 */
function wordsOf(row: Element): Box[] {
  const walker = document.createTreeWalker(row, NodeFilter.SHOW_TEXT);
  const boxes: Box[] = [];
  for (let node = walker.nextNode(); node !== null; node = walker.nextNode()) {
    const text = node.textContent?.trim() ?? "";
    const cell = node.parentElement;
    // A tooltip's bubble is in the document while closed, and hidden.
    if (text === "" || cell === null || cell.closest("[hidden]") !== null) continue;
    const range = document.createRange();
    range.selectNodeContents(node);
    const drawn = range.getBoundingClientRect();
    const clip = getComputedStyle(cell).overflowX === "hidden" ? cell.getBoundingClientRect() : drawn;
    boxes.push({
      text,
      left: Math.max(drawn.left, clip.left),
      right: Math.min(drawn.right, clip.right),
      top: drawn.top,
      bottom: drawn.bottom,
    });
  }
  return boxes;
}

const overlaps = (a: Box, b: Box) =>
  a.left < b.right - 0.5 && b.left < a.right - 0.5 && a.top < b.bottom - 0.5 && b.top < a.bottom - 0.5;

async function groups() {
  await page.viewport(1280, 800);
  mount(onJob(job2Landed()));
  // By what it holds rather than by its name, so this finds the list as it was
  // before the fix too, and the run against the fix reverted fails on the geometry.
  const list = page.getByRole("list").filter({ hasText: "Group one" }).last();
  await expect.element(list).toBeVisible();
  return list;
}

describe("Job 2's groups, merged, as Fleet served them", () => {
  test("no words in one row draw over another's, and every row stays inside its card at 1280", async () => {
    const list = await groups();
    const card = list.element().closest(".armada-chapter")!.getBoundingClientRect();
    const rows = list.getByRole("listitem").elements();
    expect(rows).toHaveLength(4);
    const every = rows.flatMap((row) => wordsOf(row));
    const collided = every.flatMap((one, at) =>
      every.slice(at + 1).filter((other) => overlaps(one, other)).map((other) => `${one.text} / ${other.text}`),
    );
    expect(collided).toEqual([]);
    const outside = every.filter((one) => one.left < card.left || one.right > card.right).map((one) => one.text);
    expect(outside).toEqual([]);
  });

  test("the commit is short, with the whole of it in its tooltip", async () => {
    const list = await groups();
    const first = list.getByRole("listitem").first();
    const short = first.getByText(COMMIT.slice(0, 7), { exact: true });
    await expect.element(short).toBeVisible();
    // What is drawn, and not the closed bubble, which holds the whole of it.
    expect(wordsOf(first.element()).filter((one) => one.text.includes(COMMIT))).toEqual([]);
    const described = short.element().closest("[aria-describedby]");
    expect(described).not.toBeNull();
    await expect.element(described as HTMLElement).toHaveAccessibleDescription(`Commit ${COMMIT}`);
  });

  test("no task count, no word for a missing time, and no line about what nothing timed", async () => {
    const list = await groups();
    const said = list.element().closest(".armada-chapter")!.textContent ?? "";
    expect(said).not.toMatch(/\d+ of \d+ done/);
    expect(said).not.toContain("not timed");
    expect(said).not.toContain("Nothing times a group");
    // Open, the head counts nothing: the groups it would count are under it.
    expect(said).not.toContain("4 groups · 4 tasks");
  });
});
