// A Board row draws the bell for `JobSummary.alert`: one mark, naming the Trigger, and none where there is no alert.

import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";

import { running } from "./fixtures/build/running";
import { mount, unmount } from "./mounted";
import { Row } from "./Row";

afterEach(() => unmount());

const props = (job: ReturnType<typeof running>["job"]) => ({
  job,
  headline: job.title,
  stale: false,
  now: Date.now(),
  workflows: [],
  selected: false,
  focused: false,
  onOpen: () => undefined,
  onKill: () => undefined,
  onRedispatch: () => undefined,
  onClear: () => undefined,
  onPausing: () => undefined,
  onCopied: () => undefined,
});

test("a row carries the bell for a Job a Trigger holds, with the Trigger in its tooltip", async () => {
  const { job } = running();
  mount(<Row {...props({ ...job, alert: { kind: "held", trigger: "deploy_qa", when: "pr_opened", step: "handoff" } })} />);
  const bell = page.getByRole("img", { name: "Held, deploy_qa, PR opened, handoff" });
  await expect.element(bell).toBeVisible();
  await bell.hover();
  expect(document.querySelector(".armada-tooltip__label")?.textContent).toContain("deploy_qa");
});

test("a row with no alert draws no bell", async () => {
  const { job } = running();
  mount(<Row {...props(job)} />);
  await expect.element(page.getByText(job.title).first()).toBeVisible();
  expect(page.getByRole("img", { name: /deploy_qa/ }).elements()).toHaveLength(0);
});
