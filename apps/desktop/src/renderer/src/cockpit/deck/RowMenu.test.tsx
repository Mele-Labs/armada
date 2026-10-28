// A Session row's context menu carries Report, so the row can be reported like anything else, and Name it for me, and holds Close with the Job's acts above the View group.

import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";
import type { SessionsDraft } from "@armada/screens/src/draft/sessions";

import "../../styles/index.css";
import { SessionsFrom } from "../../sessions-draft";
import { RowMenu } from "./RowMenu";

let mounted: { root: Root; host: HTMLElement }[] = [];

afterEach(() => {
  for (const one of mounted) {
    one.root.unmount();
    one.host.remove();
  }
  mounted = [];
  document.querySelectorAll("[data-noted]").forEach((one) => one.remove());
});

test("the menu lists Report, and choosing it asks to report the row", async () => {
  const row = document.createElement("button");
  row.setAttribute("data-noted", "");
  document.body.append(row);
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted.push({ root, host });
  root.render(<RowMenu at={{ id: "s-1", root: "/repo", title: "Job", x: 20, y: 20, row }} onClose={() => undefined} onAutopilot={() => undefined} />);

  const asked = vi.fn();
  window.addEventListener("armada:report", asked);
  await page.getByRole("menuitem", { name: "Report" }).click();
  window.removeEventListener("armada:report", asked);

  expect(asked).toHaveBeenCalledTimes(1);
  expect((asked.mock.calls[0]?.[0] as CustomEvent<Element>).detail).toBe(row);
});

/** A draft that serves only what the menu reads: the list (one stable array, as a store reads), and the acts a test hands it. */
const NONE: never[] = [];
const serving = (acts: Partial<SessionsDraft>) => ({ get: () => NONE, subscribe: () => () => undefined, ...acts }) as unknown as SessionsDraft;

function openOn(draft: SessionsDraft | undefined): void {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted.push({ root, host });
  root.render(
    <SessionsFrom held={draft}>
      <RowMenu at={{ id: "s-1", root: "/repo", title: "Job", x: 20, y: 20, row: null }} onClose={() => undefined} onAutopilot={() => undefined} />
    </SessionsFrom>,
  );
}

const labels = (): string[] => [...document.querySelectorAll('[role^="menuitem"]')].map((one) => one.textContent?.trim() ?? "");

test("Name it for me is in the menu where the draft can name a Job, and choosing it names the row's Job", async () => {
  const nameForMe = vi.fn(async (_id: string) => undefined);
  openOn(serving({ nameForMe }));

  await page.getByRole("menuitem", { name: "Name it for me" }).click();

  expect(nameForMe).toHaveBeenCalledTimes(1);
  expect(nameForMe).toHaveBeenCalledWith("s-1");
});

test("Name it for me is left out where the draft cannot name a Job", async () => {
  openOn(serving({}));

  await expect.element(page.getByRole("menuitem", { name: "Auto Pilot" })).toBeVisible();
  expect(labels()).not.toContain("Name it for me");
});

test("Close sits with the Job's own acts, before the View group's first entry", async () => {
  openOn(serving({ close: vi.fn(), nameForMe: async () => undefined }));

  await expect.element(page.getByRole("menuitem", { name: "Close" })).toBeVisible();
  const shown = labels();
  expect(shown.indexOf("Close")).toBeGreaterThan(shown.indexOf("Name it for me"));
  // The View group opens on its first field (`VIEW_FIELDS`).
  expect(shown.indexOf("Close")).toBeLessThan(shown.indexOf("Active PRs"));
  expect(shown.indexOf("Close")).toBeGreaterThanOrEqual(0);
});
