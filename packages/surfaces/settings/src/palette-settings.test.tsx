// Choosing "Fleet limits" from the command palette goes to the Settings screen.
//
// **This is the wiring the palette's Settings section exists to prove.** The
// primitive draws the row and `@armada/shell` turns choosing it into a
// `PaletteChoice`; neither package can mount `BridgeSettings`, since a screen
// sits above both on the layer this package's own comment names. So the
// claim — a chosen row actually navigates, not only that something fired —
// is provable only here, where `Palette` and the screen can be mounted side
// by side.

import { afterEach, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import { expect } from "vitest";
import { useState } from "react";

import { Palette } from "@armada/shell";
import type { Outcome } from "@armada/protocol";

import { BridgeSettings } from "./BridgeSettings";
import { MOCK_SETTINGS } from "./fake-settings";
import { openSettingsAt } from "./sections";
import { mount, unmount } from "@armada/screens/src/mounted";

afterEach(unmount);

/** A stand-in for the app: the palette and the rail's own toggle, wired the way `App.tsx` wires them. */
function Host() {
  const [showing, setShowing] = useState(false);
  return (
    <>
      <Palette
        open
        onClose={() => {}}
        context="board"
        on={null}
        surfaces={[]}
        jobs={[]}
        settings={[{ id: "fleet_settings", label: "Fleet limits" }]}
        onChoose={(choice) => {
          if (choice.of === "setting") openSettingsAt(choice.id, () => setShowing(true));
        }}
        onConfirmAct={() => {}}
      />
      {showing ? (
        <BridgeSettings
          settings={MOCK_SETTINGS}
          live
          onSaveSettings={(): Promise<Outcome> => Promise.resolve({ ok: true })}
          onOpenSettingsFile={() => Promise.resolve({ ok: true })}
        />
      ) : null}
    </>
  );
}

test("choosing Fleet limits from the palette shows the Settings screen", async () => {
  mount(<Host />);

  await expect.element(page.getByRole("heading", { name: "Limits" })).not.toBeInTheDocument();

  await userEvent.click(page.getByRole("option", { name: "Fleet limits" }));

  // The row names a section, and its group is what opens.
  await expect.element(page.getByRole("heading", { name: "Limits" })).toBeVisible();
  await expect.element(page.getByRole("tab", { name: "Fleet" })).toHaveAttribute("aria-selected", "true");
});
