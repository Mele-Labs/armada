// The Workflows page on a repository in two Workspaces, through `App`: the row picked in the list sets the level, so the repository's own copy edits that repository and Armada's edits the default; the warning
// where Platform and Mobile both change a rule; and the buttons that keep either one's rule as the repository's own. The reads and saves are `slices/lifecycles.ts`; the model is `lifecycle-editing`.

import { expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, toSystemPage, unmountAfterEach } from "./testing";
import { workspaceTabs } from "./scenarios/workspace-tabs";

unmountAfterEach();

const API = "01M1CNPKTV0018H2M1CXDNBK0C";
const ARMADA = () =>
  page.getByRole("group", {
    name: "Shipped with Armada",
    exact: true,
  });
const API_COPY = () =>
  page
    .getByRole("group", {
      name: "api repository",
      exact: true,
    })
    .getByRole("button", {
      name: "feature",
      exact: true,
    });
const LANES = () =>
  page.getByRole("region", {
    name: "Workflow",
    exact: true,
  });
const PANEL = page.getByRole("dialog");
const WARNING = () =>
  LANES().getByRole("img", {
    name: "Platform and Mobile differ",
    exact: true,
  });

/** The page on api, with api's own copy of feature picked in the list. */
async function onApi(): Promise<ReturnType<typeof mount>> {
  await page.viewport(2000, 1200);
  const app = mount(workspaceTabs);
  await onScreen();
  await toSystemPage("Workflows");
  await expect.element(LANES()).toBeVisible();
  await API_COPY().click();
  return app;
}

test("api's copy in the list says edits there change api only, and Armada's row says the default", async () => {
  const app = await onApi();
  const saved = vi.spyOn(app.api, "saveLifecyclePiece");
  await expect.element(page.getByText("Edits here change api only")).toBeVisible();
  expect(
    page
      .getByText("Edits here change every project", {
        exact: false,
      })
      .elements(),
  ).toHaveLength(0);
  // An edit made there is kept under api, not under a Workspace or the default.
  await LANES()
    .getByRole("button", {
      name: "Mark ready",
      exact: true,
    })
    .click();
  await PANEL.getByRole("button", {
    name: "Use Platform's",
    exact: true,
  }).click();
  await expect.poll(() => saved.mock.calls.length).toBe(1);
  expect(saved.mock.calls[0]![0]).toMatchObject({
    piece: {
      rule: {
        scope: {
          kind: "within",
          id: API,
        },
      },
    },
  });
  await ARMADA()
    .getByRole("button", {
      name: "feature",
      exact: true,
    })
    .click();
  await expect
    .element(
      page.getByText("Edits here change every project and repository that keeps the default"),
    )
    .toBeVisible();
  expect(page.getByText("Edits here change api only").elements()).toHaveLength(0);
});

test("api warns on Mark ready, where Platform and Mobile both change it, and says it runs the Default's", async () => {
  await onApi();
  await expect.element(WARNING()).toBeVisible();
  await LANES()
    .getByRole("button", {
      name: "Mark ready",
      exact: true,
    })
    .click();
  await expect
    .element(PANEL.getByText("Platform and Mobile both change Mark ready. api runs the Default's."))
    .toBeVisible();
  await expect
    .element(
      PANEL.getByRole("button", {
        name: "Use Platform's",
        exact: true,
      }),
    )
    .toBeVisible();
  await expect
    .element(
      PANEL.getByRole("button", {
        name: "Use Mobile's",
        exact: true,
      }),
    )
    .toBeVisible();
});

test("Use Mobile's keeps Mobile's rule as api's own, and the warning and the line are gone", async () => {
  const app = await onApi();
  const saved = vi.spyOn(app.api, "saveLifecyclePiece");
  await LANES()
    .getByRole("button", {
      name: "Mark ready",
      exact: true,
    })
    .click();
  await PANEL.getByRole("button", {
    name: "Use Mobile's",
    exact: true,
  }).click();
  await expect.poll(() => saved.mock.calls.length).toBe(1);
  expect(saved.mock.calls[0]![0]).toMatchObject({
    piece: {
      kind: "rule",
      rule: {
        id: "ready",
        autonomy: "acts",
        scope: {
          kind: "within",
          id: API,
        },
      },
    },
  });
  // The line is gone and api has a way back to the default's: the piece is its own now.
  await expect
    .element(
      PANEL.getByRole("button", {
        name: "Use the default",
        exact: true,
      }),
    )
    .toBeVisible();
  expect(
    PANEL.getByText("both change Mark ready", {
      exact: false,
    }).elements(),
  ).toHaveLength(0);
  expect(WARNING().elements()).toHaveLength(0);
  const read = await app.api.listLifecycleRules(API);
  const kept = read.ok
    ? read.value.rules
        .filter((one) => one.rule.id === "ready")
        .map((one) => [
          one.rule.scope,
          one.rule.autonomy,
        ])
    : [];
  expect(kept).toContainEqual([
    {
      kind: "within",
      id: API,
    },
    "acts",
  ]);
});

test("Use Platform's keeps Platform's rule as api's own", async () => {
  const app = await onApi();
  const saved = vi.spyOn(app.api, "saveLifecyclePiece");
  await LANES()
    .getByRole("button", {
      name: "Mark ready",
      exact: true,
    })
    .click();
  await PANEL.getByRole("button", {
    name: "Use Platform's",
    exact: true,
  }).click();
  await expect.poll(() => saved.mock.calls.length).toBe(1);
  expect(saved.mock.calls[0]![0]).toMatchObject({
    piece: {
      rule: {
        id: "ready",
        autonomy: "asks",
        scope: {
          kind: "within",
          id: API,
        },
      },
    },
  });
  expect(WARNING().elements()).toHaveLength(0);
});
