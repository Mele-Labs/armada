// What the palette is handed, and what it hands back — #1528. Lifted out of
// `App.tsx` whole, beside `palette.ts`, which already holds what the palette
// can reach and what is dormant where.
//
// **The acts travel as one object** because `carryOut` already takes them as
// one. Ten props would be ten chances for a caller to wire one to the wrong
// thing; the object is the shape that function already agrees on.

import { Puzzle } from "lucide-react";
import { Palette, type PaletteRepository, type PaletteSurface } from "@armada/shell";
import type { PaletteDispatch } from "@armada/components";
import type { BridgeState } from "../../shared/bridge";
import { checkoutRunnablesOf } from "@armada/manifest";
import { SETTINGS_PALETTE } from "@armada/settings";
import { studioName } from "@armada/screens";
import { askStudioAdd, askStudioRun, useStudioAddOff, useStudioRunOff } from "@armada/studios";
import { askToDebugBridge } from "./debug-bridge";
import { askToOpenSession, openSessionIn } from "./open-session";
import { openInPanel, openRequestsInPanel } from "./shared-panel/store";
import { useSessions } from "./sessions-draft";
import { askToCreateMod } from "./create-mod";
import { kindOfLink } from "./link-routes";
import { absentIn, carryOut, CREATE_MOD, dormantIn, FLEET_DOWN } from "./palette";

export type PaletteMountProps = {
  open: boolean;
  onClose: () => void;
  /** The Studio open on its whiteboard, or nothing — by its name alone, which
      is all a title needs and all this should be able to read. */
  shownStudio: { name?: string } | null;
  surfaces: readonly PaletteSurface[];
  checkoutRunSheet: BridgeState["checkoutRunSheet"];
  failing: unknown | null;
  /** Whether Fleet is connected. */
  live: boolean;
  /** The repositories the title row's picker lists, and the one picked: `null` is All. */
  repositories: readonly PaletteRepository[];
  picked: string | null;
  /**
   * Everything a choice can do, in `carryOut`'s own shape — but the Studio's
   * Run and Add, which this reaches itself: the rail takes the ask directly, so
   * `App` holds nothing for them.
   */
  acts: Omit<Parameters<typeof carryOut>[1], "studioRun" | "studioAdd" | "createMod" | "debugBridge" | "openSession" | "requests">;
  /** The one bar's dispatch row. `one-bar.tsx`. */
  dispatch: PaletteDispatch;
  /** Opens the page a typed link names, and answers whether it did. `link-routes.ts`. */
  openLink: (typed: string) => boolean;
};

/**
 * **The one surface present whatever else is**, which is why `App.tsx` mounts
 * it as a sibling of the shell rather than inside a screen.
 */
export function PaletteMount({
  open,
  onClose,
  shownStudio,
  surfaces,
  checkoutRunSheet,
  failing,
  live,
  repositories,
  picked,
  acts,
  dispatch,
  openLink,
}: PaletteMountProps) {
  const studioRun = useStudioRunOff();
  const studioAdd = useStudioAddOff();
  const sessions = useSessions();
  return (
    <Palette
      open={open}
      onClose={onClose}
      dispatch={dispatch}
      // A pasted pull request opens in Reviews and an issue in the side panel.
      link={{ kindOf: kindOfLink, onOpen: (typed) => openLink(typed) }}
      // A pasted path opens in the panel, in the Session's checkout where it is relative: any file the person can read.
      path={{
        onOpen: async (typed) => {
          const item = await window.armada.openPanelPath(typed, openSessionIn(document));
          if (item !== null) openInPanel(item);
          return item !== null;
        },
      }}
      // Two places: a Studio open on its whiteboard is not the Cockpit, and the acts
      // scoped to it act on the board rather than on anything focused — #1364.
      context={shownStudio === null ? "board" : "studio"}
      // The block is titled with what its rows act on, which on a Studio is the
      // Studio: the acts scoped there put a node on the board.
      on={shownStudio === null ? null : `Studio — ${studioName(shownStudio)}`}
      surfaces={surfaces}
      // **No filter rows.** They were the Job Board's state tabs, and that
      // surface is gone, so there is nothing here for a row to set.
      filters={[]}
      // A row about the window as a whole: it starts a Mod Job through Fleet, so it waits on the connection.
      board={[{ id: CREATE_MOD, label: "Create mod", icon: Puzzle, ...(live ? {} : { dormant: FLEET_DOWN }) }]}
      // One row per Check and Command, off the same reading the Manifest
      // surface draws from — Journey 9's own table. Empty until that read has
      // answered, which is what `App.tsx`'s effect holds open.
      runnables={checkoutRunnablesOf(checkoutRunSheet)}
      repositories={repositories}
      picked={picked}
      // Only the live Sessions: the palette jumps to work in hand.
      // A Session is found by any word of its title or by its id, whole or the `s-` address.
      sessions={sessions.filter((one) => one.dead === undefined).map((one) => ({ id: one.id, label: one.title ?? one.address ?? one.id, aliases: [one.id, ...(one.address === undefined ? [] : [one.address])] }))}
      // One row per category of Settings, each opening Settings at it — Fleet
      // settings first. None carries a value: each opens a screen rather than
      // stating a field.
      settings={SETTINGS_PALETTE}
      dormant={dormantIn({ failing: failing !== null, live, debuggable: window.armadaDev !== undefined, studioRun, studioAdd })}
      absent={absentIn({ failing: failing !== null })}
      onChoose={(choice: Parameters<typeof carryOut>[0]) =>
        carryOut(choice, { ...acts, studioRun: askStudioRun, studioAdd: askStudioAdd, createMod: askToCreateMod, debugBridge: askToDebugBridge, openSession: askToOpenSession, requests: () => void openRequestsInPanel(openSessionIn(document)) })
      }
    />
  );
}

