// A Session's message box keeps what was started in it, through `App`: leave for another surface or
// another Session and come back, and the text, the attached file and the tag are still there. A send
// clears them.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { RepositorySummary } from "@armada/protocol";
import { job, repository } from "@armada/screens/src/fixtures/build/base";

import { onBoard } from "./scenario";
import { FakeSessionsFleet, hosted } from "./sessions-fleet";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const open = async (title: string) => {
  await userEvent.click(page.getByRole("button", { name: "Sessions", exact: true }));
  await userEvent.click(page.getByRole("region", { name: "Sessions" }).getByRole("button", { name: title }));
};

const message = () => page.getByRole("textbox", { name: "Message" });

test("Sessions keep: text, a file and an inline tag survive another surface and another Session, and a send clears them", async () => {
  const fleet = new FakeSessionsFleet([hosted("01SESSIONAAAAAAAAAAAAAAAAA", { title: "First one" }), hosted("01SESSIONBBBBBBBBBBBBBBBBB", { title: "Second one" })]);
  const stopped = job("escalated", { id: "01JOBSTOPPED", handle: "52-the-retry-loop", title: "The retry loop", branch: "fix/retry-loop", owner_manifest_id: "armada" });
  mount(fleet.scenario(onBoard([stopped], { repositories: [ARMADA], picked: ARMADA.root })));
  await onScreen();
  await open("First one");

  await userEvent.fill(message(), "Look at @retry");
  await userEvent.click(page.getByRole("option", { name: /The retry loop/ }));
  await userEvent.upload(page.getByLabelText("Files to attach"), new File(["hello"], "notes.txt", { type: "text/plain" }));
  await userEvent.type(message(), " and then");

  await userEvent.click(page.getByRole("button", { name: "Worktree Slots", exact: true }));
  await open("Second one");
  await expect.element(message()).toHaveTextContent("");
  await userEvent.fill(message(), "Only for the second");
  await open("First one");

  await expect.element(message()).toHaveTextContent("and then");
  await expect.element(message().getByText("The retry loop")).toBeVisible();
  await expect.element(page.getByRole("group", { name: "Attached" }).getByText("notes.txt")).toBeVisible();
  await open("Second one");
  await expect.element(message()).toHaveTextContent("Only for the second");

  await open("First one");
  await userEvent.click(page.getByRole("button", { name: "Send" }));
  await expect.poll(() => fleet.calls.sent.length).toBe(1);
  await open("Second one");
  await open("First one");
  await expect.element(message()).toHaveTextContent("");
  await expect.element(message().getByText("The retry loop")).not.toBeInTheDocument();
  await expect.element(page.getByLabelText("Attached").getByText("notes.txt")).not.toBeInTheDocument();
});
