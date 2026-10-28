// A sketch put to the person, answered on its card in a Session's thread, on a real Fleet's path through `App`: Approve
// and Send back with a note each go to `POST /sessions/sketch/answer` as an `AnswerSketch`, carrying the drawing the ledger
// held when the card was drawn.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "./scenario";
import { arrow, box, scene } from "./scenes";
import { FakeSessionsFleet, held, hosted } from "./sessions-fleet";
import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };
const ID = "01SKETCHAAAAAAAAAAAAAAAAAA";
const TITLE = "Pause on the row";
const DRAWING = scene([box("row", 0, 0, "Job row"), box("fleet", 320, 0, "Fleet")], [arrow("row", "fleet", "pause")]);

async function open() {
  const fleet = new FakeSessionsFleet(
    [hosted(ID, { title: "Pause a Job from its row", attachments: [held("artifact", `sketch:${TITLE}`, { form: "sketch", title: TITLE, scene: JSON.stringify(DRAWING), asked: "true", drawn: "drawn-1" })] })],
    { [ID]: [{ kind: "tool", id: "t-1", at: "2026-10-07T13:49:00.000Z", text: `show_sketch ${JSON.stringify({ title: TITLE, scene: DRAWING })}` }] },
  );
  const app = mount(fleet.scenario(onBoard({ repositories: [ARMADA], picked: ARMADA.root })));
  const answer = vi.spyOn(app.api, "answerSketch").mockResolvedValue({ ok: true, value: hosted(ID) });
  await onScreen();
  await toSession("Pause a Job from its row");
  return answer;
}

const card = () => page.getByRole("article", { name: `Sketch: ${TITLE}` });

test("Approve on a sketch's card answers it approved, with the drawing it was shown", async () => {
  const answer = await open();
  await card().getByRole("button", { name: "Approve" }).click();
  await expect.poll(() => answer.mock.calls.length).toBe(1);
  expect(answer).toHaveBeenLastCalledWith({ session_id: ID, sketch: `sketch:${TITLE}`, drawn: "drawn-1", how: "approved" });
});

test("Send back on a sketch's card sends the note it was typed with, as words", async () => {
  const answer = await open();
  await card().getByRole("button", { name: "Send back" }).click();
  await userEvent.fill(card().getByRole("textbox", { name: "Note" }), "  Show the held turn too.  ");
  await card().getByRole("button", { name: "Send back" }).click();
  await expect.poll(() => answer.mock.calls.length).toBe(1);
  expect(answer).toHaveBeenLastCalledWith({ session_id: ID, sketch: `sketch:${TITLE}`, drawn: "drawn-1", how: "sent_back", words: "Show the held turn too." });
});
