// "Your checkout" as a real window draws it: no Sessions fixtures in the scenario, so the card runs over `WiredSessions` and the
// fake's own `window.armada`, with the Sessions list not yet read. Run and the files' changes must be on the card then too: they are
// Fleet's, answered before any Session is, and Copy and Run show without the pointer over the block.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { s257CheckoutHeldBack } from "./scenarios/checkout-held-back";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const SAFE = "880 commits behind main";

/** The walk's scenario as a real window has it: the same fleet, and no Sessions draft. */
const wired = { ...s257CheckoutHeldBack, draft: { ...s257CheckoutHeldBack.draft, sessions: undefined } };

async function opened() {
  const app = mount(wired);
  await onScreen();
  await page.getByRole("region", { name: "Merge queue" }).getByRole("button", { name: `Main is green, checkout ${SAFE}` }).click();
  return { app, section: page.getByRole("dialog", { name: `Main is green, checkout ${SAFE}` }).getByRole("region", { name: "Your checkout" }) };
}

const opacityOf = (button: Element) => getComputedStyle(button.closest(".armada-prose__run-acts")!).opacity;

test("a window with no Sessions read yet still has Run on the card's command, and Copy and Run show without hover", async () => {
  const { section } = await opened();
  const run = section.getByRole("button", { name: "Run" });
  await expect.element(run).toBeVisible();
  await expect.element(section.getByRole("button", { name: "Copy" })).toBeVisible();
  expect(opacityOf(run.element())).toBe("1");
  expect(opacityOf(section.getByRole("button", { name: "Copy" }).element())).toBe("1");
});

test("Run asks Fleet to catch the repository up by its Manifest alone, and sends no command", async () => {
  const { app, section } = await opened();
  const catchUp = vi.spyOn(app.api, "catchUpCheckout");
  await section.getByRole("button", { name: "Run" }).click();
  await expect.poll(() => section.getByRole("log", { name: "Output" }).element().textContent ?? "").not.toBe("");
  await expect.poll(() => catchUp.mock.calls.length).toBe(1);
  expect(catchUp.mock.calls[0]).toHaveLength(1);
  expect(typeof catchUp.mock.calls[0]?.[0]).toBe("string");
});

test("Pull request asks Fleet by the repository's Manifest alone, and waits for the hold to carry the pull request", async () => {
  const { app, section } = await opened();
  const send = vi.spyOn(app.api, "sendCheckoutAsPullRequest").mockResolvedValue({ ok: true, value: {} });
  const button = section.getByRole("button", { name: "Pull request" });
  await userEvent.click(button);
  await expect.poll(() => send.mock.calls.length).toBe(1);
  expect(send.mock.calls[0]).toHaveLength(1);
  expect(typeof send.mock.calls[0]?.[0]).toBe("string");
  // Fleet answered, but the card still draws the hold it was given: the button stays pending rather than offering a second press.
  await expect.element(button).toHaveAttribute("aria-busy", "true");
  await expect.element(button).toHaveAttribute("aria-disabled", "true");
});

test("a refused Pull request frees the button again", async () => {
  const { app, section } = await opened();
  const send = vi.spyOn(app.api, "sendCheckoutAsPullRequest").mockResolvedValue({
    ok: false,
    outcome: {
      ok: false,
      why: "refused",
      error: { code: "fleet.checkout_not_catchable", message: "an uncommitted file also changed on `main`: armada.yml", run_id: "r", fields: {}, chain: [] },
    },
  });
  const button = section.getByRole("button", { name: "Pull request" });
  await userEvent.click(button);
  await expect.poll(() => send.mock.calls.length).toBe(1);
  await expect.element(button).not.toHaveAttribute("aria-busy");
  await expect.element(button).toBeVisible();
});

test("a path in Uncommitted opens the change Fleet reads for it, and a second press folds it", async () => {
  const { app, section } = await opened();
  const read = vi.spyOn(app.api, "readCheckoutDiff").mockResolvedValue({
    ok: true,
    value: { diff: "diff --git a/armada.yml b/armada.yml\n--- a/armada.yml\n+++ b/armada.yml\n@@ -1,1 +1,2 @@\n x: 1\n+auto_merge: checks-pass\n" },
  });
  const path = section.getByRole("button", { name: "armada.yml" });
  await expect.element(path).toHaveAttribute("aria-expanded", "false");
  await userEvent.click(path);
  await expect.element(section.getByText("+auto_merge: checks-pass")).toBeVisible();
  await expect.element(path).toHaveAttribute("aria-expanded", "true");
  // The change sits inside the card and the card inside the window.
  const card = section.element().closest("[role=dialog]")!.getBoundingClientRect();
  expect(card.bottom).toBeLessThanOrEqual(window.innerHeight);
  expect(section.element().querySelector(".deck-checkout__diff")!.getBoundingClientRect().bottom).toBeLessThanOrEqual(card.bottom);
  await userEvent.click(path);
  await expect.element(section.getByText("+auto_merge: checks-pass")).not.toBeInTheDocument();
  expect(read.mock.calls.length).toBeLessThanOrEqual(1);
});

test("a change Fleet cut is drawn with one line after it saying how much", async () => {
  const { app, section } = await opened();
  vi.spyOn(app.api, "readCheckoutDiff").mockResolvedValue({
    ok: true,
    value: { diff: "diff --git a/armada.yml b/armada.yml\n--- a/armada.yml\n+++ b/armada.yml\n@@ -1,1 +1,3 @@\n x: 1\n+y: 2\n+z: 3\n", cut: true },
  });
  await userEvent.click(section.getByRole("button", { name: "armada.yml" }));
  await expect.element(section.getByRole("note")).toHaveTextContent("First 4 lines");
});
