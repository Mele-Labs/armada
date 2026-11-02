// A Session row's context menu gets Mod and Report from the app (`PointActs`), so the row can be reported like anything else, and carries Name it for me, and holds Close with the Job's acts above the View group.

import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import "../../styles/index.css";
import type { AnnotationsDevApi } from "../../../../shared/annotations";
import { PointActsProvided } from "../../point-acts";
import { SessionsFrom } from "../../sessions-draft";
import { RowMenu } from "./RowMenu";

let mounted: {
  root: Root;
  host: HTMLElement;
}[] = [];

afterEach(() => {
  for (const one of mounted) {
    one.root.unmount();
    one.host.remove();
  }
  mounted = [];
  window.armadaDev = undefined;
  document.querySelectorAll("[data-noted]").forEach((one) => one.remove());
});

/** The row menu opened by a right press on `row`, inside the app's own acts, as `App` mounts it. */
function pressedOn(row: HTMLElement): void {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted.push({
    root,
    host,
  });
  root.render(
    <PointActsProvided>
      <RowMenu
        at={{
          id: "s-1",
          root: "/repo",
          title: "Job",
          x: 20,
          y: 20,
          row,
        }}
        onClose={() => undefined}
        onAutopilot={() => undefined}
      />
    </PointActsProvided>,
  );
  // Noted in the capture phase, ahead of the menu, as the right press that opens it is.
  row.dispatchEvent(
    new MouseEvent("contextmenu", {
      bubbles: true,
      cancelable: true,
      clientX: 20,
      clientY: 20,
    }),
  );
}

test("the menu lists Report, and choosing it asks to report the row", async () => {
  window.armadaDev = {} as AnnotationsDevApi;
  const row = document.createElement("button");
  row.setAttribute("data-noted", "");
  document.body.append(row);
  await new Promise((done) => setTimeout(done, 0));
  pressedOn(row);

  const asked = vi.fn();
  window.addEventListener("armada:report", asked);
  await page
    .getByRole("menuitem", {
      name: "Report",
    })
    .click();
  window.removeEventListener("armada:report", asked);

  expect(asked).toHaveBeenCalledTimes(1);
  expect((asked.mock.calls[0]?.[0] as CustomEvent<Element>).detail).toBe(row);
});

test("Mod and Report come last, after the Job's acts and the View group, and the row menu has none of its own", async () => {
  window.armadaDev = {} as AnnotationsDevApi;
  const row = document.createElement("button");
  row.setAttribute("data-noted", "");
  document.body.append(row);
  await new Promise((done) => setTimeout(done, 0));
  pressedOn(row);

  await expect
    .element(
      page.getByRole("menuitem", {
        name: "Report",
      }),
    )
    .toBeVisible();
  const shown = labels();
  expect(shown.slice(-2)).toEqual([
    "Mod",
    "Report",
  ]);
  expect(shown.filter((one) => one === "Mod" || one === "Report")).toHaveLength(2);
  expect(shown.indexOf("Auto Pilot")).toBeLessThan(shown.indexOf("Mod"));
  expect(shown.indexOf("Active PRs")).toBeLessThan(shown.indexOf("Mod"));
});

test("without the app's acts around it the row menu has no Mod or Report", async () => {
  window.armadaDev = {} as AnnotationsDevApi;
  openOn(serving({}));
  await expect
    .element(
      page.getByRole("menuitem", {
        name: "Auto Pilot",
      }),
    )
    .toBeVisible();
  expect(labels()).not.toContain("Mod");
  expect(labels()).not.toContain("Report");
});

/** A draft that serves only what the menu reads: the list (one stable array, as a store reads), and the acts a test hands it. */
const NONE: never[] = [];
const serving = (acts: Partial<SessionsDraft>) =>
  ({
    get: () => NONE,
    subscribe: () => () => undefined,
    ...acts,
  }) as unknown as SessionsDraft;

function openOn(draft: SessionsDraft | undefined): void {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted.push({
    root,
    host,
  });
  root.render(
    <SessionsFrom held={draft}>
      <RowMenu
        at={{
          id: "s-1",
          root: "/repo",
          title: "Job",
          x: 20,
          y: 20,
          row: null,
        }}
        onClose={() => undefined}
        onAutopilot={() => undefined}
      />
    </SessionsFrom>,
  );
}

const labels = (): string[] =>
  [
    ...document.querySelectorAll('[role^="menuitem"]'),
  ].map((one) => one.textContent?.trim() ?? "");

test("Name it for me is in the menu where the draft can name a Job, and choosing it names the row's Job", async () => {
  const nameForMe = vi.fn(async (_id: string) => undefined);
  openOn(
    serving({
      nameForMe,
    }),
  );

  await page
    .getByRole("menuitem", {
      name: "Name it for me",
    })
    .click();

  expect(nameForMe).toHaveBeenCalledTimes(1);
  expect(nameForMe).toHaveBeenCalledWith("s-1");
});

test("Name it for me is left out where the draft cannot name a Job", async () => {
  openOn(serving({}));

  await expect
    .element(
      page.getByRole("menuitem", {
        name: "Auto Pilot",
      }),
    )
    .toBeVisible();
  expect(labels()).not.toContain("Name it for me");
});

test("Close sits with the Job's own acts, before the View group's first entry", async () => {
  openOn(
    serving({
      close: vi.fn(),
      nameForMe: async () => undefined,
    }),
  );

  await expect
    .element(
      page.getByRole("menuitem", {
        name: "Close",
      }),
    )
    .toBeVisible();
  const shown = labels();
  expect(shown.indexOf("Close")).toBeGreaterThan(shown.indexOf("Name it for me"));
  // The View group opens on its first field (`VIEW_FIELDS`).
  expect(shown.indexOf("Close")).toBeLessThan(shown.indexOf("Active PRs"));
  expect(shown.indexOf("Close")).toBeGreaterThanOrEqual(0);
});

/** A Job holding a deploy that is rolling out, as the lanes carry it. */
const ROLLING = [
  {
    id: "s-1",
    lanes: {
      job: {
        copy: {
          lifecycles: [
            {
              id: "deploy",
              states: [
                {
                  id: "rolling_out",
                  label: "Rolling out",
                },
              ],
            },
          ],
        },
        instances: [
          {
            id: "deploy:Deploy",
            lifecycle: "deploy",
            label: "Deploy",
            state: "rolling_out",
          },
        ],
      },
    },
  },
];

test("Close on a Job that holds something in flight lists it first, and ends the Job only on End job", async () => {
  const close = vi.fn();
  openOn(
    serving({
      close,
      get: () => ROLLING as unknown as readonly Session[],
    }),
  );

  await page
    .getByRole("menuitem", {
      name: "Close",
    })
    .click();
  await expect.element(page.getByText("Deploy: Rolling out")).toBeVisible();
  expect(close).not.toHaveBeenCalled();

  await page
    .getByRole("button", {
      name: "End job",
    })
    .click();
  expect(close).toHaveBeenCalledTimes(1);
  expect(close).toHaveBeenCalledWith("s-1");
});

test("Close on a Job that holds nothing in flight ends it at once", async () => {
  const close = vi.fn();
  openOn(
    serving({
      close,
    }),
  );

  await page
    .getByRole("menuitem", {
      name: "Close",
    })
    .click();

  expect(close).toHaveBeenCalledTimes(1);
  expect(close).toHaveBeenCalledWith("s-1");
});
