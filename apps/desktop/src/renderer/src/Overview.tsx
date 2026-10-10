// The Dashboard, as the window mounts it, out of `App.tsx`, which is at the length the gate refuses.
// Three tabs over the panels: Command Central (what needs the owner), Running, and Done. Work is
// dispatched from the title bar's one bar (`one-bar.tsx`). Each reads Jobs, Sessions and the merge line alike. #921 mounted
// the tile band here first; Overview 27 (#1091) gave every panel a fold, and the tabs replaced the strip.
//
// `.armada-screen__overview` is the one child `.armada-screen__mounted` gets — the surface's own
// padding and gap, so neither the strip nor a panel sits against the window's edge. `Boundary`
// renders its children straight through when nothing has thrown.

import { useEffect, useState } from "react";
import type { FixMain, RepositorySummary } from "@armada/protocol";
import type { CallView } from "@armada/jobs/draft/calls";
import type { NowView } from "@armada/jobs/draft/now";
import type { BoardSection, PauseAct } from "@armada/screens";
import { OverviewLists, dashboardTabOf, overviewPanelId, type DashboardTab } from "@armada/overview";
import { Boundary, useLayout } from "@armada/shell";

import type { BridgeState } from "../../shared/bridge";
import { Nows } from "./Dashboard";
import { CommandCentral } from "./CommandCentral";
import { KeySheet, useDashboardKeys } from "./cockpit/keys";
import { proposeRequest } from "./dispatch";

import { usePanelOpen } from "./panel-open";
import { useDashboardTab } from "./remembered-views";

type StripSection = "needs-you" | "running" | "queued" | "recently-ended";

export function Overview({
  state,
  now,
  live,
  repositories,
  disconnected,
  selected,
  onOpen,
  onKill,
  onRedispatch,
  onClear,
  onPausing,
  onCompose,
  onCopied,
  onCursor,
  land,
  onLanded,
  onOpenLink,
  onOpenSession,
  onFix,
  nowViews,
  nows,
  onTell,
  bring,
  onBrought,
}: {
  /** A call an alert asked for, handed to the cockpit once it is mounted. */
  bring?: { key: string; at: number } | null;
  onBrought?: () => void;
  state: BridgeState;
  now: number;
  live: boolean;
  /** Every repository Fleet serves. */
  repositories: readonly RepositorySummary[];
  /** The connection's own statement, where Fleet cannot be reached. */
  disconnected: string | null;
  /** The Job whose detail is open, where one is. */
  selected: string | null;
  onOpen: (jobId: string) => void;
  onKill: (jobId: string) => void;
  /** Ask to redispatch — Recently ended's own control, Overview 28 (#1092). Asks; never redispatches. */
  onRedispatch: (jobId: string) => void;
  /** Ask to clear — Recently ended's caret, beside Redispatch. Asks; never clears. */
  onClear: (jobId: string) => void;
  onPausing?: (act: PauseAct, jobId: string) => void;
  /** Open the composer — `n`, `OverviewLists`' own prop, passed straight through. */
  onCompose: () => void;
  onCopied: (value: string) => void;
  /** Where the cursor is, reported up — `OverviewLists`' own state, mirrored. #1075. */
  onCursor?: (jobId: string | null) => void;
  /**
   * A section a pressed notification asked for — `App`'s own token. **Opened
   * and scrolled to once this is mounted**, which the press could not do: it
   * may have arrived over the composer or over a Job. `at` is what makes a
   * second press of the same section land.
   */
  land?: { section: StripSection; at: number } | null;
  /** Taken, so the token is not acted on twice. */
  onLanded?: () => void;
  /** Hands an address to whatever opens addresses on this machine: the merge line's pull requests. */
  onOpenLink: (address: string) => void;
  /** A Session the list opened. Where no Sessions are served the list draws nothing. */
  onOpenSession: (sessionId: string) => void;
  /** Hands main's red to a Job, from the merge line's band. */
  onFix?: (fix: FixMain) => void;
  /** What each Job asks and has gone wrong in, by Job id. Mock only: Fleet serves none, and then only Jobs flagged as asking are listed. */
  nowViews?: Readonly<Record<string, CallView>>;
  /** What each Job is doing now, by Job id: the Running tab's pane and its tiles' icons. Mock only. */
  nows?: Readonly<Record<string, NowView>>;
  /** Says a sentence as a toast: a refusal the panel has to name. */
  onTell?: (sentence: string) => void;
}) {
  const guarded = { bridge: state.bridge, onCopied };

  // Overview 24's mechanism (#1088), widened past the left column to Overview's own five panels —
  // a fold survives a restart because it is a layout choice, not a fact Fleet holds.
  const [needsYouOpen, setNeedsYouOpen] = usePanelOpen("needs-you");
  const [runningOpen, setRunningOpen] = usePanelOpen("running");
  const [queuedOpen, setQueuedOpen] = usePanelOpen("queued");
  const [recentlyEndedOpen, setRecentlyEndedOpen] = usePanelOpen("recently-ended");
  const [otherOpen, setOtherOpen] = usePanelOpen("other");
  // **Folded until somebody asks.** What is over is read on purpose, the rule
  // the Board's own Done fold was built under. Done is here because the Board
  // is not, and every completed or cleared Job would otherwise have gone with
  // it — it is the one reading that surface had and this did not.
  const [doneOpen, setDoneOpen] = usePanelOpen("done", false);
  const setters: Record<BoardSection, (open: boolean) => void> = {
    "needs-you": setNeedsYouOpen,
    running: setRunningOpen,
    queued: setQueuedOpen,
    "recently-ended": setRecentlyEndedOpen,
    done: setDoneOpen,
    other: setOtherOpen,
  };
  const openSections: Partial<Record<BoardSection, boolean>> = {
    "needs-you": needsYouOpen,
    running: runningOpen,
    queued: queuedOpen,
    "recently-ended": recentlyEndedOpen,
    done: doneOpen,
    other: otherOpen,
  };
  const onSectionOpenChange = (section: BoardSection, open: boolean) => setters[section](open);

  // A press names a section; opening it (if folded) and scrolling to it happen once that open
  // state has committed, which is what the effect below waits for.
  const [jump, setJump] = useState<{ section: StripSection; at: number } | null>(null);
  const [tab, setTab] = useDashboardTab(useLayout("dashboard.tabs").first as DashboardTab);
  const onJump = (section: StripSection) => {
    setters[section](true);
    setTab(dashboardTabOf(section));
    setJump({ section, at: Date.now() });
  };
  // A notification's own landing, taken once Overview is on screen.
  useEffect(() => {
    if (land === undefined || land === null) return;
    onJump(land.section);
    onLanded?.();
    // `at` alone: the same section pressed twice is two landings.
  }, [land?.at]);

  useEffect(() => {
    if (jump === null) return;
    const panel = document.getElementById(overviewPanelId(jump.section));
    if (panel === null) return;
    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    panel.scrollIntoView({ behavior: reduced ? "auto" : "smooth", block: "start" });
  }, [jump]);

  // A Drone sent at a failing pull request is a request on the approval gate, as the composer's.
  const propose = (request: string) => void proposeRequest(request, [], state.repository);

  const pickedRepository = repositories.find((one) => one.root === state.repository) ?? null;
  // Fleet unreachable, still starting, or nothing served: the lists say so, as they always did.
  const fault = disconnected !== null || state.connection.state === "starting" || repositories.length === 0;

  // `[` `]` and Option with a digit switch filters, `?` lists the keys. Where Fleet cannot be reached the lists draw instead.
  const keys = useDashboardKeys(tab, setTab, !fault);

  return (
    <Boundary region="the overview" {...guarded}>
      <Nows.Provider value={nows}>
      <div className="armada-screen__overview">
        {fault ? (
          <OverviewLists
            jobs={state.jobs}
            stale={!live}
            now={now}
            workflows={state.holds.workflows}
            repositories={repositories}
            picked={state.repository}
            disconnected={disconnected}
            starting={state.connection.state === "starting"}
            selected={selected}
            openSections={openSections}
            onSectionOpenChange={onSectionOpenChange}
            onOpen={onOpen}
            onKill={onKill}
            onRedispatch={onRedispatch}
            onClear={onClear}
            {...(onPausing === undefined ? {} : { onPausing })}
            onCompose={onCompose}
            onCopied={onCopied}
            onCursor={onCursor}
          />
        ) : (
          <CommandCentral
            filter={tab}
            onFilter={setTab}
            state={state}
            now={now}
            picked={pickedRepository}
            nowViews={nowViews}
            nows={nows}
            bring={bring}
            onBrought={onBrought}
            onTell={onTell}
            onOpen={onOpen}
            onOpenSession={onOpenSession}
            onOpenLink={onOpenLink}
            onFix={onFix}
            onPropose={propose}
            onKill={onKill}
            onRedispatch={onRedispatch}
            onClear={onClear}
            onPausing={onPausing}
            onCursor={onCursor}
          />
        )}
        <KeySheet open={keys.sheet} onClose={keys.closeSheet} />
      </div>
      </Nows.Provider>
    </Boundary>
  );
}
