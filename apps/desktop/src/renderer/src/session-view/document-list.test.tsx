// A list in a Job's document shows its markers: discs for a bulleted list, circles when one is nested, numbers for a numbered list, each
// indented under the text. Read as a person sees it, from computed styles on the page the mock Job opens, through `App`, whose reset
// strips every list's markers and indent.

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

test("a bulleted list shows discs, a nested one circles, a numbered list numbers, and each is indented", async () => {
  const article = await opened();
  const bulleted = article.querySelector(":scope ul:not(li ul)")!;
  const nested = bulleted.querySelector("li ul")!;
  const numbered = article.querySelector("ol")!;
  expect.soft(getComputedStyle(bulleted).listStyleType, "a bulleted list shows discs").toBe("disc");
  expect.soft(getComputedStyle(nested).listStyleType, "a nested bulleted list shows circles").toBe("circle");
  expect.soft(getComputedStyle(numbered).listStyleType, "a numbered list shows numbers").toBe("decimal");
  for (const [said, list] of [["bulleted", bulleted], ["numbered", numbered]] as const) {
    expect.soft(Number.parseFloat(getComputedStyle(list).paddingInlineStart), `the ${said} list is indented`).toBeGreaterThan(0);
  }
}, 60_000);
