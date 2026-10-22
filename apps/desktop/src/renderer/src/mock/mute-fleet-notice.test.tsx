// Fleet's behind-base notice, muted from a right press on it. Each choice sends one `set_notice_mute` body.
// A message from another Job has no such menu. The walk `mute-fleet-notice` shows how it reads.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "./scenario";
import { FakeSessionsFleet } from "./sessions-fleet";
import { mount, onScreen, toSession, unmountAfterEach } from "./testing";
import { NOTICE_JOB, noticeSessions, noticeThreads } from "./sessions/mute-notice";

unmountAfterEach();

async function open() {
  const fleet = new FakeSessionsFleet(noticeSessions(), noticeThreads());
  const armada = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };
  mount(fleet.scenario(onBoard({ repositories: [armada], picked: armada.root })));
  await onScreen();
  await toSession("Fix the flaky store test");
  return fleet;
}

const rightPress = (region: string, words: RegExp): void => {
  const target = page.getByRole("region", { name: region }).getByRole("button", { name: words }).element();
  const box = target.getBoundingClientRect();
  target.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, composed: true, view: window, clientX: box.x + 4, clientY: box.y + 4, button: 2 }));
};

const choices = [
  ["For job", { kind: "job", session_id: NOTICE_JOB }],
  ["For repo", { kind: "repo", manifest_id: "armada" }],
  ["For 1 hour", { kind: "hour", session_id: NOTICE_JOB }],
] as const;

for (const [item, scope] of choices) {
  test(`${item} sends one mute for the notice`, async () => {
    const fleet = await open();
    rightPress("Fleet turn", /23 commits behind master/);
    await expect.element(page.getByRole("menuitem", { name: item })).toBeVisible();
    await userEvent.click(page.getByRole("menuitem", { name: item }));
    expect(fleet.calls.muted).toEqual([{ notice: "behind_base", scope, muted: true }]);
  });
}

test("the menu holds exactly the three choices", async () => {
  await open();
  rightPress("Fleet turn", /23 commits behind master/);
  await expect.element(page.getByRole("menuitem")).toHaveLength(3);
});

test("a message from another Job has no Disable menu", async () => {
  await open();
  const other = page.getByRole("region", { name: "Message from Round the totals" }).getByText(/totals file/).element();
  other.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, composed: true, view: window, button: 2 }));
  await expect.element(page.getByRole("menuitem", { name: /^Disable/ })).not.toBeInTheDocument();
});
