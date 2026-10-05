// The Overview's fallback cards, mounted — the ones a Job draws while its
// canvas has nothing to draw (a read that failed or has not answered).
//
// **A block comment written as JSX text is drawn.** One sat unbraced inside the
// fragment and showed as prose above the Brief on the owner's own screen.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { OverviewBoard } from "./OverviewBoard";
import { mount } from "./mounted";

test("the fallback cards draw no raw comment text", async () => {
  mount(
    <OverviewBoard
      lead={{ said: "Waiting for your approval", because: "" }}
      waiting={undefined}
      briefAbsent="Fleet did not answer"
      workflowAbsent="Fleet did not answer"
      planAbsent="Fleet did not answer"
      pulse={[]}
      settings=""
      onOpenTab={() => undefined}
    />
  );
  await expect.element(page.getByText("Fleet did not answer").first()).toBeVisible();
  const text = document.querySelector(".armada-overview-board")?.textContent ?? "";
  expect(text).not.toContain("/*");
  expect(text).not.toContain("*/");
});
