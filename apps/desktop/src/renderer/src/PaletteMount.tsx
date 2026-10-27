// What the palette is handed, and what it hands back — #1528. Lifted out of
// `App.tsx` whole, beside `palette.ts`, which already holds what the palette
// can reach and what is dormant where.
//
// **The acts travel as one object** because `carryOut` already takes them as
// one. Ten props would be ten chances for a caller to wire one to the wrong
// thing; the object is the shape that function already agrees on.

import { useEffect, useRef, useState } from "react";
import { GitPullRequest, Puzzle } from "lucide-react";
import { choiceOf, Palette, SURFACE, type PaletteChoice, type PaletteRepository, type PaletteSurface, type PaletteTask } from "@armada/shell";
import { Dialog, parseChord, pressesChord, type PaletteDispatch, type PaletteStep } from "@armada/components";
import { useProjectsStore } from "./projects-tab/store";
import { useSessionsDraft, useSessionsEverywhere } from "./sessions-draft";
import { useDeck } from "./cockpit/deck/store";
import { jobRow } from "./palette-job-row";
import { usePaletteRows } from "./palette-fixtures";
import { knownRows, setKnownRows } from "./palette-known";
import { nextOpens, opensLabel, remember, rowOf, usePaletteMemory } from "./palette-memory";
import { pageOf } from "./palette-page";
import { carryOutRow } from "./palette-rows";
import type { BridgeState } from "../../shared/bridge";
import { checkoutRunnablesOf } from "@armada/manifest";
import { SETTINGS_PALETTE } from "@armada/settings";
import { studioName } from "@armada/screens";
import { askStudioAdd, askStudioRun, useStudioAddOff, useStudioRunOff } from "@armada/studios";
import { askToDebugBridge } from "./debug-bridge";
import { askToOpenSession, openSessionIn } from "./open-session";
import { openInPanel, openRequestsInPanel } from "./shared-panel/store";
import { askToCreateMod } from "./create-mod";
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
  /** The page showing, by its surface id: the palette heads its results with that page's own acts. */
  showing?: string;
  /** The name a Job's repository goes by in the title row's picker, from the Manifest id the Job carries. */
  repoOf?: (manifestId: string) => string | undefined;
  /** The Projects tab's tasks and subtasks, found by title. `projects-tab/palette.ts`. */
  tasks?: readonly PaletteTask[];
  /** The one bar's dispatch row. `one-bar.tsx`. */
  dispatch: PaletteDispatch;
  /** What a typed link names, and opening it. `link-routes.ts`. */
  linkRoutes: { kindOf: (typed: string) => "pull" | "issue" | null; open: (typed: string) => boolean };
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
  showing,
  repoOf,
  acts,
  tasks,
  dispatch,
  linkRoutes,
}: PaletteMountProps) {
  const studioRun = useStudioRunOff();
  const studioAdd = useStudioAddOff();
  // Every repository's Jobs, whatever the title row has picked: the palette jumps to work anywhere.
  const sessions = useSessionsEverywhere();
  const draft = useSessionsDraft();
  const fixtures = usePaletteRows(open);
  const memory = usePaletteMemory();
  const projects = useProjectsStore();
  const onDeck = useDeck((deck) => (deck.sel?.kind === "session" ? deck.sel.key : undefined));
  const job = showing === SURFACE.overview && onDeck !== undefined ? sessions.find((one) => one.id === onDeck) : undefined;
  // A Job that asks to be stopped is confirmed over the palette, which stays up behind it.
  const [stopping, setStopping] = useState<string | null>(null);
  const stopped = stopping === null ? undefined : sessions.find((one) => one.id === stopping);

  const world = {
    fixtures, jobs: sessions, draft, job,
    projects, ...(acts.task === undefined ? {} : { task: acts.task }),
    taskTitle: (rowId: string) => tasks?.find((one) => one.id === rowId)?.label,
    openSession: askToOpenSession, compose: acts.compose, surface: acts.surface,
    openLink: (address: string) => linkRoutes.open(address),
    startJob: (words: string) => dispatch.onDispatch(words, "session"),
  };
  // A command with a blank asks for it in the palette's second step, listing what can fill it.
  const commandStep = (id: string): PaletteStep | undefined =>
    id !== SECURITY_REVIEW
      ? undefined
      : {
          title: "Pull request to review",
          sections: [{ id: "pulls", title: "Open pull requests" }],
          entries: fixtures.pulls.map((one) => ({ id: `pull:${one.repo}:${one.number}`, section: "pulls", label: one.title, icon: GitPullRequest, meta: { repo: one.repo, number: `#${one.number}` } })),
          searched: "every open pull request",
          onChoose: (entry) => {
            const one = fixtures.pulls.find((held) => `pull:${held.repo}:${held.number}` === entry.id);
            if (one !== undefined) dispatch.onDispatch(`Review #${one.number} ${one.title} for security`, "session");
          },
        };
  const choose = (choice: PaletteChoice): void => {
    if (choice.of === "setting" && choice.id === PALETTE_OPENS_ROW) return nextOpens();
    const row = rowOf(choice);
    if (row !== undefined) remember(row);
    if (choice.of === "pull" || choice.of === "issue" || choice.of === "task" || choice.of === "command" || choice.of === "page" || choice.of === "on") return carryOutRow(choice, world);
    carryOut(choice, { ...acts, studioRun: askStudioRun, studioAdd: askStudioAdd, createMod: askToCreateMod, debugBridge: askToDebugBridge, openSession: askToOpenSession, requests: () => void openRequestsInPanel(openSessionIn(document)) });
  };
  // **A key he gave a row fires anywhere in Bridge**, with the palette down, and runs the row as ↵ would. It is read in the
  // capture phase so a terminal or a field does not keep it, and a key being recorded in Settings is left to the recorder.
  const choosing = useRef(choose);
  choosing.current = choose;
  const hotkeys = memory.hotkeys;
  useEffect(() => {
    if (open) return;
    function fire(event: KeyboardEvent): void {
      if (event.defaultPrevented || event.repeat || (event.target as Element | null)?.closest?.("[data-recording]") != null) return;
      for (const [id, spelled] of Object.entries(hotkeys)) {
        const chord = parseChord(spelled);
        if (chord === null || !pressesChord(chord, event)) continue;
        if (knownRows().find((one) => one.id === id)?.dormant === true) return;
        event.preventDefault();
        event.stopPropagation();
        choosing.current(choiceOf(id));
        return;
      }
    }
    window.addEventListener("keydown", fire, true);
    return () => window.removeEventListener("keydown", fire, true);
  }, [open, hotkeys]);
  return (
    <>
    <Palette
      open={open}
      onClose={onClose}
      dispatch={dispatch}
      // A pasted pull request opens in Reviews and an issue in the side panel.
      link={{ kindOf: linkRoutes.kindOf, onOpen: (typed) => linkRoutes.open(typed) }}
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
      sessions={sessions.filter((one) => one.dead === undefined).map((one) => jobRow(one, repoOf, draft))}
      // Tasks and subtasks by title, listed only while something is typed.
      {...(tasks === undefined ? {} : { tasks })}
      // One row per category of Settings, each opening Settings at it — Fleet
      // settings first. None carries a value: each opens a screen rather than
      // stating a field.
      settings={[...SETTINGS_PALETTE, { id: PALETTE_OPENS_ROW, label: "Palette opens with", value: opensLabel(memory.opens) }]}
      page={pageOf(showing ?? "", job)}
      pulls={fixtures.pulls}
      issues={fixtures.issues}
      commands={fixtures.commands}
      commandStep={commandStep}
      pinned={memory.pinned}
      recent={memory.recent}
      opens={memory.opens}
      aliases={memory.aliases}
      hotkeys={memory.hotkeys}
      onRows={setKnownRows}
      onConfirmAct={(id) => id === "job-stop" && job !== undefined && setStopping(job.id)}
      onConfirmOn={(target, action) => action === "stop" && setStopping(target.slice("session:".length))}
      dormant={dormantIn({ failing: failing !== null, live, debuggable: window.armadaDev !== undefined, studioRun, studioAdd })}
      absent={absentIn({ failing: failing !== null })}
      onChoose={choose}
    />
    <Dialog
      open={stopped !== undefined}
      title={`Stop ${stopped?.title ?? "this Job"}?`}
      confirmLabel="Stop"
      onConfirm={() => {
        if (stopping !== null) draft?.stop?.(stopping);
        setStopping(null);
        onClose();
      }}
      onCancel={() => setStopping(null)}
    >
      The running turn ends. The Job rests and takes the next message.
    </Dialog>
    </>
  );
}

/** The row `Settings` carries for what the empty query shows, which cycles on a press until the setting has a page. */
const PALETTE_OPENS_ROW = "palette-opens";

/** The command whose blank is a pull request. */
const SECURITY_REVIEW = "security-review";
