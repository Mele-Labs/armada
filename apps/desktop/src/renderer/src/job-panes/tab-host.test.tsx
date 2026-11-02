// A built tab mod's body as Fleet's word on the mod moves: switched off it stops drawing and says so, and refused, failing to load or throwing it says why and offers Fix,
// which starts a Mod Job on the tab that the tab then follows. The module loader is stood in for by name, since a browser test has no mod folder to read.

import { act, createElement, useSyncExternalStore } from "react";
import { createRoot, type Root } from "react-dom/client";
import type { ModSummary } from "@armada/protocol";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import { peekCustom } from "./custom";
import { TabMod } from "./tab-host";

const MOD = "review-queue";
const TAB = "tab-1";

// Fleet's mod list, which a test moves as Fleet's rescan would.
let mods: ModSummary[] = [];
const listeners = new Set<() => void>();
const publish = (next: ModSummary[]) => {
  mods = next;
  listeners.forEach((on) => on());
};
vi.mock("../mod-list", () => ({
  useModList: () =>
    useSyncExternalStore(
      (on) => (listeners.add(on), () => void listeners.delete(on)),
      () => mods,
    ),
}));

// Each module the tests load, by the code `validate_mod` hands back.
vi.mock("./tab-module", () => ({
  loadTabModule: async (code: string) => {
    if (code === "good") return () => createElement("p", null, "Pull request #812");
    if (code === "throws")
      return () => {
        throw new Error("rows is not defined");
      };
    throw new Error("tab.js did not load: Unexpected token '}'");
  },
}));

let code = "good";
const startModSession = vi.fn(async () => ({
  ok: true,
  value: {
    session_id: "fixer",
    title: "Mod Fix Review queue",
  },
}));
const row = (more: Partial<ModSummary> = {}): ModSummary =>
  ({
    name: MOD,
    kind: "tab",
    enabled: true,
    valid: true,
    made_by: "builder",
    changed_at: "2026-11-01T12:00:00Z",
    ...more,
  }) as ModSummary;
let mounted:
  | {
      root: Root;
      host: HTMLElement;
    }
  | undefined;

beforeEach(() => {
  code = "good";
  startModSession.mockClear();
  (
    window as unknown as {
      armada: unknown;
    }
  ).armada = {
    validateMod: async (name: string) => ({
      name,
      valid: true,
      problems: [],
      tab: {
        code,
      },
    }),
    startModSession,
  };
  mods = [
    row(),
  ];
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(<TabMod mod={MOD} jobId="job" tabId={TAB} />));
  mounted = {
    root,
    host,
  };
});
afterEach(() => {
  mounted?.root.unmount();
  mounted?.host.remove();
  mounted = undefined;
});

const drawn = () => page.getByText("Pull request #812");
const fix = () =>
  page.getByRole("button", {
    name: "Fix",
    exact: true,
  });

test("a tab whose mod is switched off stops drawing it, says so, and offers no Fix", async () => {
  await expect.element(drawn()).toBeVisible();
  act(() =>
    publish([
      row({
        enabled: false,
      }),
    ]),
  );
  await expect.element(page.getByRole("alert")).toHaveTextContent("Switched off");
  await expect.element(drawn()).not.toBeInTheDocument();
  await expect.element(fix()).not.toBeInTheDocument();
});

test("a tab Fleet refuses shows the reason and Fix, which starts a Mod Job on the tab that the tab then follows", async () => {
  await expect.element(drawn()).toBeVisible();
  const reason = "tab.js line 1: an `import` cannot be resolved, the tab is handed `host` instead";
  act(() =>
    publish([
      row({
        valid: false,
        reason,
        changed_at: "2026-11-01T12:01:00Z",
      }),
    ]),
  );
  await expect.element(page.getByRole("alert")).toHaveTextContent(reason);
  await expect.element(drawn()).not.toBeInTheDocument();
  await fix().click();
  await vi.waitFor(() => expect(startModSession).toHaveBeenCalledOnce());
  const [start] = startModSession.mock.calls[0] as unknown as [
    {
      aim: string;
      said: string;
      from_job: string;
      from_tab: string;
    },
  ];
  expect(start.aim).toBe("tab");
  // The Job it builds for is the one whose panes hold the tab, so it gets no row of its own.
  expect([
    start.from_job,
    start.from_tab,
  ]).toEqual([
    "job",
    TAB,
  ]);
  expect(start.said).toContain(`\`${MOD}\``);
  expect(start.said).toContain(reason);
  await vi.waitFor(() => expect(peekCustom(TAB).job).toBe("fixer"));
});

test("a rewrite that does not load is told above the tab still drawn, with Fix", async () => {
  await expect.element(drawn()).toBeVisible();
  code = "broken";
  act(() =>
    publish([
      row({
        changed_at: "2026-11-01T12:02:00Z",
      }),
    ]),
  );
  await expect.element(page.getByRole("alert")).toHaveTextContent("tab.js did not load");
  await expect.element(drawn()).toBeVisible();
  await expect.element(fix()).toBeVisible();
});

test("a tab that throws as it draws says what it threw and offers Fix", async () => {
  await expect.element(drawn()).toBeVisible();
  code = "throws";
  act(() =>
    publish([
      row({
        changed_at: "2026-11-01T12:03:00Z",
      }),
    ]),
  );
  await expect.element(page.getByRole("alert")).toHaveTextContent("rows is not defined");
  await expect.element(fix()).toBeVisible();
});
