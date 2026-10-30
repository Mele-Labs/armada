// The Fleet section of the Teach panel draws a rule as Fleet read it: the notice, the pattern it matches as code, what it does with each
// condition after it, where, and one line for what Fleet cannot test. `TeachPanel` in `@armada/components`, in a browser.

import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";

import { TeachPanel } from "@armada/components";
import type { TeachReading } from "@armada/components";
import { mount, unmount } from "@armada/screens/src/mounted";

afterEach(unmount);

const shown = (fleet: NonNullable<TeachReading["fleet"]>) =>
  mount(
    <TeachPanel
      source={{ kind: "notice", text: "3 commits behind main" }}
      say="Batch these, and tell me only when it blocks a merge"
      onSay={() => undefined}
      keep="teach-panel-fleet"
      repository="armada"
      reach="repository"
      onReach={() => undefined}
      stage="shown"
      reading={{ fleet }}
      onTeach={() => undefined}
      onRule={() => undefined}
      onChange={() => undefined}
      onSave={() => undefined}
    />,
  );

test("a rule draws the notice, what it matches as code, what it does with its condition, and where", async () => {
  shown({ notice: "Slot behind notice", matches: "\\d+ commits behind main", does: "Batch, and tell only when it blocks a merge", scope: "armada" });
  const section = page.getByRole("region", { name: "Fleet" });

  await expect.element(section.getByText("Slot behind notice")).toBeVisible();
  await expect.element(section.getByText("Matches", { exact: true })).toBeVisible();
  await expect.element(section.getByText("\\d+ commits behind main")).toHaveClass("mono");
  await expect.element(section.getByText("Batch, and tell only when it blocks a merge")).toBeVisible();
  await expect.element(section.getByText("armada", { exact: true })).toBeVisible();
  await expect.element(section.getByText(/Fleet cannot test/)).not.toBeInTheDocument();
});

test("a rule with no pattern draws no Matches row", async () => {
  shown({ notice: "Slot behind notice", does: "Batch", scope: "armada" });

  await expect.element(page.getByRole("region", { name: "Fleet" }).getByText("Does", { exact: true })).toBeVisible();
  await expect.element(page.getByText("Matches", { exact: true })).not.toBeInTheDocument();
});

test("what Fleet cannot test is one line under the rule", async () => {
  shown({ notice: "Slot behind notice", does: "Batch", scope: "armada", untestable: "Fleet cannot test: the build is green, it is Friday" });

  await expect.element(page.getByText("Fleet cannot test: the build is green, it is Friday")).toBeVisible();
});
