// The application menu keeps what Electron's default gives and adds Revert after About, on while a build before is kept.

import { describe, expect, it, vi } from "vitest";

import type { FleetBuildReport } from "@armada/protocol";
import { appMenuTemplate, REVERT_ITEM } from "./app-menu";
import { canRevert } from "./revert-asked";

const PREVIOUS: FleetBuildReport["previous"] = {
  built_at: "2026-11-05T22:00:00Z",
  commit: "b".repeat(40),
};
const REPORT: FleetBuildReport = {
  on: "main",
  previous: PREVIOUS,
};

describe("the template", () => {
  it("keeps the file, edit, view and window roles and the app menu's own items", () => {
    const menu = appMenuTemplate("Armada", {
      enabled: true,
      click: () => {},
    });
    expect(menu.slice(1).map((one) => one.role)).toEqual([
      "fileMenu",
      "editMenu",
      "viewMenu",
      "windowMenu",
    ]);
    expect(menu[0]!.label).toBe("Armada");
    const app = menu[0]!.submenu as {
      role?: string;
      label?: string;
    }[];
    expect(app.map((one) => one.role ?? one.label).filter((one) => one !== undefined)).toEqual([
      "about",
      REVERT_ITEM,
      "services",
      "hide",
      "hideOthers",
      "unhide",
      "quit",
    ]);
  });

  it("puts the revert item right after About, enabled as told, and clicking runs the flow", () => {
    const click = vi.fn();
    for (const enabled of [
      true,
      false,
    ]) {
      const app = appMenuTemplate("Armada", {
        enabled,
        click,
      })[0]!.submenu as {
        label?: string;
        enabled?: boolean;
        click?: () => void;
      }[];
      expect(app[1]).toMatchObject({
        label: "Revert to the build before…",
        enabled,
      });
    }
    (
      appMenuTemplate("Armada", {
        enabled: true,
        click,
      })[0]!.submenu as {
        click?: () => void;
      }[]
    )[1]!.click!();
    expect(click).toHaveBeenCalledTimes(1);
  });
});

describe("when the item is on", () => {
  it("needs a previous build in the report", () => {
    expect(canRevert(REPORT)).toBe(true);
    expect(canRevert(null)).toBe(false);
    expect(
      canRevert({
        on: "main",
      }),
    ).toBe(false);
  });

  it("is off while a revert, a restart or a swap is under way", () => {
    expect(
      canRevert({
        ...REPORT,
        reverting: true,
      }),
    ).toBe(false);
    expect(
      canRevert({
        ...REPORT,
        restarting: "main",
      }),
    ).toBe(false);
    expect(
      canRevert({
        ...REPORT,
        swapping: true,
      }),
    ).toBe(false);
  });
});
