// A draft card in a Session's thread, acted on from words selected in it: a right press on the words offers Edit and Add note.
// Add note opens the note box with the words quoted; Edit opens the draft's box with the words selected, or plain when the
// words are not in the source. The walk `draft-card` shows it.

import { expect, test } from "vitest";
import { page } from "vitest/browser";
import type { RepositorySummary, SessionRow } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "./scenario";
import { FakeSessionsFleet, hosted } from "./sessions-fleet";
import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };
const AT = "2026-10-07T13:48:02.000Z";
const LIVE = "01LIVEAAAAAAAAAAAAAAAAAAAA";

const TEXT = ["## Why", "", "A photo upload that lost its connection halfway failed for good.", "", "- A retry resumes from the last part the server confirmed", "- Tom &amp; Jerry keep their place"].join("\n");
const WORDS = "resumes from the last part the server confirmed";

const thread = (): Record<string, SessionRow[]> => ({ [LIVE]: [{ kind: "draft", id: "d-1", at: AT, title: "Description for #412", text: TEXT }] });

async function open() {
  const fleet = new FakeSessionsFleet([hosted(LIVE, { title: "A live hosted one" })], thread());
  mount(fleet.scenario(onBoard({ repositories: [ARMADA], picked: ARMADA.root })));
  await onScreen();
  await toSession("A live hosted one");
}

const card = () => page.getByRole("article", { name: "Draft: Description for #412" });

/** Select `words` inside the draft's text and right press on them, as a pointer does. */
function pressOn(words: string) {
  const block = card().element().querySelector(".armada-session-draft__text")!;
  const all = document.createTreeWalker(block, NodeFilter.SHOW_TEXT);
  for (let node = all.nextNode(); node !== null; node = all.nextNode()) {
    const at = node.textContent?.indexOf(words) ?? -1;
    if (at < 0) continue;
    window.getSelection()!.setBaseAndExtent(node, at, node, at + words.length);
    const box = window.getSelection()!.getRangeAt(0).getBoundingClientRect();
    node.parentElement!.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, composed: true, view: window, clientX: box.x + 4, clientY: box.y + box.height / 2, button: 2 }));
    return;
  }
  throw new Error(`the draft does not hold "${words}"`);
}

test("A right press on words selected in a draft offers Edit and Add note", async () => {
  await open();
  pressOn(WORDS);
  await expect.element(page.getByRole("menuitem", { name: "Edit" })).toBeVisible();
  await expect.element(page.getByRole("menuitem", { name: "Add note" })).toBeVisible();
  await expect.element(page.getByRole("menuitem")).toHaveLength(2);
});

test("Add note opens the note box with the selected words quoted", async () => {
  await open();
  pressOn(WORDS);
  await page.getByRole("menuitem", { name: "Add note" }).click();
  const note = page.getByRole("textbox", { name: "Note" });
  await expect.element(note).toHaveValue(`> ${WORDS}\n\n`);
});

test("Edit opens the draft's box with the selected words selected", async () => {
  await open();
  pressOn(WORDS);
  await page.getByRole("menuitem", { name: "Edit" }).click();
  const box = page.getByRole("textbox", { name: "Draft text" }).element() as HTMLTextAreaElement;
  await expect.poll(() => [box.selectionStart, box.selectionEnd]).toEqual([TEXT.indexOf(WORDS), TEXT.indexOf(WORDS) + WORDS.length]);
});

test("Edit opens the box plain when the selected words are not in the source", async () => {
  await open();
  pressOn("Tom & Jerry");
  await page.getByRole("menuitem", { name: "Edit" }).click();
  const box = page.getByRole("textbox", { name: "Draft text" }).element() as HTMLTextAreaElement;
  expect(box.value).toBe(TEXT);
  expect(box.selectionStart).toBe(box.selectionEnd);
});
