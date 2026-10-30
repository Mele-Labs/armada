// The scenario `document-asks-again`: revision 2 of the video script, kept after he approved revision 1, carries no answer, so its card
// asks again in the thread.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { documentAsksAgain } from "./scenarios/document-asks-again";
import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

test("revision 2 asks again on its card", async () => {
  mount(documentAsksAgain);
  await onScreen();
  await toSession("Demo marketing video");
  const card = page.getByRole("article", { name: "Document: Video script, Revision 2" });
  await expect.element(card.getByRole("button", { name: "Approve" })).toBeVisible();
  await expect.element(card.getByRole("button", { name: "Send back" })).toBeVisible();
});
