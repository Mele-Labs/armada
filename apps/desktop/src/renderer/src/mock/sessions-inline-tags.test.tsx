// A tag is written into the message at the caret, through `App` with real keys: the chip stands in
// the line, the row of what waits stays empty, Backspace takes the chip whole, and what is sent
// carries `@title` in the text and the tag beside it. The sent message draws the same chip.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { RepositorySummary } from "@armada/protocol";
import { job, repository } from "@armada/screens/src/fixtures/build/base";

import { onBoard } from "./scenario";
import { FakeSessionsFleet, hosted } from "./sessions-fleet";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const message = () => page.getByRole("textbox", { name: "Message" });

test("Sessions inline tags: the chip stands in the line, Backspace takes it whole, and a send carries it", async () => {
  const fleet = new FakeSessionsFleet([hosted("01SESSIONAAAAAAAAAAAAAAAAA", { title: "First one" })]);
  const stopped = job("escalated", { id: "01JOBSTOPPED", handle: "52-the-retry-loop", title: "The retry loop", branch: "fix/retry-loop", owner_manifest_id: "armada" });
  mount(fleet.scenario(onBoard([stopped], { repositories: [ARMADA], picked: ARMADA.root })));
  await onScreen();
  await userEvent.click(page.getByRole("button", { name: "Sessions", exact: true }));
  await userEvent.click(page.getByRole("button", { name: "First one" }));

  await userEvent.click(message());
  await userEvent.keyboard("What stopped @retry");
  await userEvent.click(page.getByRole("option", { name: /The retry loop/ }));
  await userEvent.keyboard("again");
  const chip = message().getByText("The retry loop");
  await expect.element(chip).toBeVisible();
  await expect.element(page.getByRole("group", { name: "Attached" }).getByText("The retry loop")).not.toBeInTheDocument();

  // Back over "again" and the space, then one Backspace more: the chip goes in one press.
  await userEvent.keyboard("{Backspace}".repeat("again".length + 1));
  await expect.element(chip).toBeVisible();
  await userEvent.keyboard("{Backspace}");
  await expect.element(message().getByText("The retry loop")).not.toBeInTheDocument();

  await userEvent.keyboard("@retry");
  await userEvent.click(page.getByRole("option", { name: /The retry loop/ }));
  await userEvent.keyboard("again{Enter}");
  await expect.poll(() => fleet.calls.sent.length).toBe(1);
  expect(fleet.calls.sent[0]?.text).toBe("What stopped @The retry loop again");
  expect(fleet.calls.sent[0]?.mentions).toMatchObject([{ kind: "job", id: "01JOBSTOPPED", title: "The retry loop" }]);
  await expect.element(page.getByRole("region", { name: "Thread" }).getByText("The retry loop")).toBeVisible();
});
