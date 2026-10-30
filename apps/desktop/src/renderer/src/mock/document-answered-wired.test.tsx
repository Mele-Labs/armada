// A Job's document put to the person, answered on its card in the thread, on a real Fleet's path through `App`: the card is read off
// the page's artifact, with no write in the thread, and Approve and Send back with a note each go to `POST /sessions/document/answer`
// as an `AnswerDocument`, naming the revision the ledger held when the card was drawn.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "./scenario";
import { FakeSessionsFleet, held, hosted } from "./sessions-fleet";
import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };
const ID = "01DOCUMENTAAAAAAAAAAAAAAAA";
const PATH = "/work/.armada/documents/video-script.html";
const TITLE = "Video script";

async function open() {
  const fleet = new FakeSessionsFleet(
    [hosted(ID, { title: "Demo marketing video", attachments: [held("artifact", PATH, { form: "document", title: TITLE, revisions: JSON.stringify(["/s/r1.html", "/s/r2.html"]), asked: "true", revision: "2" })] })],
    { [ID]: [{ kind: "tool", id: "t-1", at: "2026-10-07T13:40:00.000Z", text: "Bash node scripts/render-script.js" }] },
  );
  const app = mount(fleet.scenario(onBoard({ repositories: [ARMADA], picked: ARMADA.root })));
  const answer = vi.spyOn(app.api, "answerDocument").mockResolvedValue({ ok: true, value: hosted(ID) });
  await onScreen();
  await toSession("Demo marketing video");
  return answer;
}

const card = () => page.getByRole("article", { name: `Document: ${TITLE}, Revision 2` });

test("Approve on a document's card answers its newest revision approved", async () => {
  const answer = await open();
  await card().getByRole("button", { name: "Approve" }).click();
  await expect.poll(() => answer.mock.calls.length).toBe(1);
  expect(answer).toHaveBeenLastCalledWith({ session_id: ID, document: `document:${PATH}`, revision: "2", how: "approved" });
});

test("Send back on a document's card sends the note it was typed with, as words", async () => {
  const answer = await open();
  await card().getByRole("button", { name: "Send back" }).click();
  await userEvent.fill(card().getByRole("textbox", { name: "Note" }), "  Make the close shorter.  ");
  await card().getByRole("button", { name: "Send back" }).click();
  await expect.poll(() => answer.mock.calls.length).toBe(1);
  expect(answer).toHaveBeenLastCalledWith({ session_id: ID, document: `document:${PATH}`, revision: "2", how: "sent_back", words: "Make the close shorter." });
});
