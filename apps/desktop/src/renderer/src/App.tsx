// The window: the Cockpit, where every Session is opened, and the pages around it. No router — one
// flag for each page that is open, and the Cockpit when none is.
//
// Everything drawn comes from the state the main process publishes over the one
// connection. Nothing here fetches, and nothing here holds a copy of what
// Fleet has not confirmed.
//
// # What this file is, now that three subjects have left it
//
// What is left is the window: which surface is open, what keeps that consistent
// as events arrive, and what is drawn. Three things that are not the window are
// beside it, each because it is a subject rather than a line of wiring —
// `commands.ts` for what the host is asked to do, `failing.ts` for which
// failure is on screen, and `palette.ts` for what the palette can reach.

import { useCallback, useEffect, useMemo, useState } from "react";
import { ArmadaMark } from "@armada/brand";
import { heldOfPicked, whyNotOpenedLink } from "@armada/screens";
import type { HelmContext } from "@armada/protocol";
import { useDockCards } from "./dock-answering";
import { contextOf, HelmDock, screenOf } from "@armada/helm";
import { UsageSurface } from "./usage-surface";
import { AccountPage, Button, GuidanceProvider, GuideCatalogue, ProseLinks, useNowWhile } from "@armada/components";

import { NOTHING_YET } from "../../shared/bridge";
import type { BridgeState } from "../../shared/bridge";
import { Boundary } from "@armada/shell";
import { Standing } from "./Standing";
import { useCopied, useSaid } from "@armada/shell";
import { useRefreshKey } from "@armada/shell";
import { sessionsHidden } from "./sessions";
import { fleetPanelClocked } from "./left-column";
import { repositoryLabel } from "@armada/shell";
import { AskRepository } from "@armada/screens";
import { openSettingsAt } from "@armada/settings";
import { SystemSurface } from "./system-surface";
import { entryForSurface, entryOfSettingsRow } from "./system-entries";
import { openSystemEntry, systemEntryNow } from "./system-entry";
import { useLandings, useSummoned, useUiSounds } from "./app-effects";
import { Overview } from "./Overview"; import { DoctorSurfaceNow, ManifestNow, SetupNow, StudiosSurfaceNow, WorktreesNow } from "./clocked-screens";
import { CallAlerts } from "./cockpit/CallAlerts";
import { DeckHost, DeckPlaces } from "./cockpit/deck/DeckHost";
import { HELM_HOME, SLEEP_HOME, useCallingHomes } from "./cockpit/deck/homes";
import { opensFor, placesOff } from "./cockpit/deck/system-page";
import { focusPanel } from "./cockpit/deck/keys";
import { deckNow } from "./cockpit/deck/store";
import { PaletteMount } from "./PaletteMount";
import { useOneBar } from "./one-bar";
import { DispatchingProvider, useDispatch } from "./dispatch-panel/dispatching";
import { useHelmBox } from "./helm-box";
import { RefsHost, useHelmRefs } from "./helm-refs";
import type { Host as RefsHostProps } from "./helm-refs";
import { useHelmHosted } from "./helm-session";
import { openAddressInPanel } from "./shared-panel/store";
import { useSessions } from "./sessions-draft";
import { useThoughtRouting } from "./thought-routing";
import { DispatchSheet, TitleDispatchedStrip } from "./dispatch-panel/entries";
import { CaptureLayer, type CaptureAim } from "./capture/Layer";
import { nodeNamed, studioName } from "@armada/screens";
import type { OpenStudio } from "@armada/studios";
import { useManifestEditing, useManifestForm } from "@armada/manifest";
import { startSetupSession, useSetup } from "@armada/setup";
import { Locate, LocatedNotice, useLocate } from "@armada/setup";
import { said } from "@armada/screens/src/copy";
import { share } from "@armada/screens/src/share";
import { sessionCheckId } from "@armada/screens/src/manifest-checks";
import { failingIn, raisedFailure } from "./failing";
import { Toasts, useRaised } from "./raised";
import { DispatchFailure, useDispatchFailure } from "./failure-session";
import { SketchSharing } from "./sketch-sharing";
import { openServerLink, openLink, restartFleet, changeFleetBuild, observeCheckoutRun, pickRepository, chooseFolder, resolveFolder, addRepository, cloneRepository, helmDebugInfo, startHelmFresh, pointHelm, getCheckoutRunOutput, getCheckoutRunDiff, readManifestFile, saveManifestFile, editManifest, readRepositoryScan, readManifestProposals, editManifestProposal, writeManifestProposal, listRepositoryAllowedCommands, removeRepositoryAllowedCommand, listKitServers, readKitInventory, removeKitAllowedCommand, addKitServer, forgetKitServer, setKitServerReach, setManifestServerReach, listCheckoutRuns, undoCheckoutRun, setupSessionFleet, captureStudioNote, readHeld, slotActs, bayActs, startCheckoutRun, startCheckoutVerify, startServer, stopCheckoutRun, stopServer, useCommands, watchCheckoutRunSheet, watchManifestDrift, watchOverview } from "./commands";
import { hiddenSurfaces, MergeLineSurface } from "./merge-line";
import { useMergeByHand } from "./merge-by-hand";
import { LessonsSurface } from "./lessons";
import { useTitleRow } from "./title-row";
import { DownshiftAlert } from "./downshift";
import { ChecksSurface, useAsked } from "./checks-surface";
import { LoadoutPanel } from "./session-loadout";
import { ReviewsSurface } from "./pr-review/surface"; import { reviewsHidden, useReviewStore } from "./pr-review/store";
import { SessionsOwnership } from "./sessions";
import { askToOpenSessionInCockpit, useOpenSessionAsked } from "./open-session"; import { useOpenRetroAsked } from "./open-retro"; import { useOpenHolderAsked } from "./open-holder";
import { useTellAsked } from "./tell";
import { useHistory } from "./history"; import { showingOf } from "./showing"; import { WorkflowCreatorSurface, workflowsWarned } from "./workflow-creator";
import { usePanelOpen } from "./panel-open";
import { useGuideListWidth } from "./guide-list-width";
import { firstContactOf, machineOf } from "./settings-surface";
import { fleetBuildOf, useFleetBuild } from "./fleet-build";
import { useBuildFailed } from "./build-failed";
import { stateOf } from "./state-feed";
import { copyDebugInfoFor, useCommandPalette } from "@armada/shell";
import { askToOpenHelm, Shell, SURFACE, SURFACES, useAtFloor, useLayout, useNarrow, useSurfaceKeys } from "@armada/shell";

/** Re-exported so nothing importing it has to learn a new path. */
export const WAITING: BridgeState = NOTHING_YET;

export function App() {
  const [state, setState] = useState<BridgeState>(WAITING);
  // What has been read and acknowledged. The count itself belongs to the
  // connection and is never reset from here — a drop that happened, happened.
  const [acknowledged, setAcknowledged] = useState(0);
  const [copied, setCopied] = useCopied();
  // What the app is telling somebody, as a sentence it already wrote. Today
  // that is only an open that did not happen; a click ending in nothing on
  // screen is the defect the openable records were added against.
  const [telling, setTelling] = useSaid();
  // A link in a model's text, for every Prose in the window. A pull request opens in the side panel; a refusal is said, never a dead click.
  const openProseLink = useCallback(
    (address: string) =>
      openAddressInPanel(address) || void openLink(address).then((followed) => {
        const why = whyNotOpenedLink(followed);
        if (why !== null) setTelling(why);
      }),
    [setTelling],
  );
  const asked = useAsked(); // Where the Checks page's requester links sent a person. `checks-surface.tsx`.
  const lands = useLandings(); // The section a pressed notification asked for, and the call an alert did.
  // Whether the held worktrees are open. **Its own view for the reports' kind of reason and not the
  // same one**: what is decided there is which of a set to give back.
  const [clearing, setClearing] = useState(false);
  // Whether Settings is open — a rail surface since #1089, the sheet it
  // replaced having lost its own opener when the status bar went (#1088).
  const [settingsShowing, setSettingsShowing] = useState(false); const [modding, setModding] = useState(false);
  // Whether Kit is open — a rail surface since #1275, at `⌘8`. Machine-wide,
  // and the rail's pick is what names its second tier.
  const [kitting, setKitting] = useState(false);
  // Whether the account's page is open: from the title row's menu and from Settings. `AccountPage`.
  const [accounting, setAccounting] = useState(false);
  // Whether the guide catalogue is open — a rail surface since #1602, and the
  // last row, so it carries no digit. Every guide, numbered and grouped,
  // readable without the screen that raised any of them.
  const [guiding, setGuiding] = useState(false);
  const [lining, setLining] = useState(false); // The merge line's own surface. `merge-line.tsx`.
  const [learning, setLearning] = useState(false); const [checking, setChecking] = useState(false); const [doctoring, setDoctoring] = useState(false); const [workflowing, setWorkflowing] = useState(false); // Lessons (`lessons.tsx`), and the Workflow creator (`workflow-creator.tsx`).
  const [usaging, setUsaging] = useState(false); // Usage, where the tokens went.
  const [reviewing, setReviewing] = useState(false); // Reviews, the pull requests waiting on him (`pr-review/`).
  // Left off the rail and the palette. Jobs keeps its place in the tab order so `⌘3` onward stay where they are; it has no page.
  const hidden = [...hiddenSurfaces(state), ...sessionsHidden(), SURFACE.jobs, ...reviewsHidden(useReviewStore() !== undefined)];
  // Whether the Manifest surface is open — Journey 9's *Running one*. It is read off the file Fleet
  // already holds, which is what lets a person run this project's lint with nothing running.
  const [manifesting, setManifesting] = useState(false);
  // Whether the Studios surface is open, which Studio is open on it, and the node selected there —
  // #1287. The last two are Helm's context as well as the screen's.
  const [studying, setStudying] = useState(false);
  const [openStudio, setOpenStudio] = useState<OpenStudio | null>(null);
  const [studioNode, setStudioNode] = useState<string | null>(null);
  // What Studio capture lands on — #1290. **It outlives the surface**: what is
  // wrong is on the Board or Overview, not on the whiteboard, so the aim is the
  // Studio last open and continued rather than the one a surface is drawing.
  const [captureAim, setCaptureAim] = useState<CaptureAim>(null);
  // The Check or Command the palette picked, or `null`. **It selects rather
  // than runs**, which is what Journey 9's own table says the palette does.
  const [picked, setPicked] = useState<string | null>(null);
  // Whether Setup is the Manifest surface's view. Its own flag: the Manifest surface's own views are three.
  const [settingUp, setSettingUp] = useState(false);
  // Set up with a session is out to Fleet: its press is pending until the Session opens or is refused.
  const [startingSession, setStartingSession] = useState(false);
  /** The Manifest reading a person put away. A later read draws again. */
  const [readingSeen, setReadingSeen] = useState<string | null>(null);
  // Whether the command palette is up.
  const palette = useCommandPalette();
  const openSetting = (id: string) => openSettingsAt(id, () => {
    const entry = entryOfSettingsRow(id);
    if (entry !== undefined) openSystemEntry(entry);
    goTo(SURFACE.settings);
  });
  const openModelSetting = () => openSetting("settings:model");
  const titleRow = useTitleRow(openModelSetting);
  // Every command the window can send, and what it holds while one is out. A re-read publishes what came back.
  const commands = useCommands({ onRead: setState });
  // Whether the window is at `--window-floor`, Fleet settings' own reading.
  const floor = useAtFloor();
  const narrow = useNarrow();
  // The left column's own fold, remembered across a restart — Bridge/1088.
  const [fleetOpen, setFleetOpen] = usePanelOpen("fleet");
  // The mock provides a fixture; a real window builds it from what Fleet reported.
  const fleetBuild = useFleetBuild() ?? fleetBuildOf(state.fleetBuild, (build, adopt) => void changeFleetBuild(build, adopt));
  // The catalogue list's width, remembered the same way the shell's column is.
  const [guideList, resizeGuideList] = useGuideListWidth();

  useEffect(() => {
    // Every publish arrives as new objects, equal or not. What did not change is kept as it was, so a publish that
    // carries nothing new draws nothing, and one that moves a Session leaves every other part of the window alone.
    // `readAt` moves on every poll and is read only to say how stale an unreachable Fleet's picture is, so while Fleet
    // answers the window keeps the one it has: a poll that changed nothing else must not draw the whole window.
    const take = (next: BridgeState) =>
      setState((was) => share(was, next.connection.state === "connected" && was.readAt !== null && next.readAt !== null ? { ...next, readAt: was.readAt } : next));
    const source = stateOf(window.armada);
    void source.state().then(take);
    return source.subscribe(take);
  }, []);

  // `⌘1`…`⌘n`, the binding the contract publishes and nothing answered until
  // the Manifest surface needed `⌘5`. One roster, read by the rail, the
  // palette and now the keyboard.
  // `⌘1` to `⌘6` are the title bar's tabs and `⌘,` is Settings.
  const layoutRail = useLayout("rail");
  useSurfaceKeys((surface) => goTo(opensFor(surface, placesOff(hidden, layoutRail.all))));

  // What this repository's Manifest declares, held open while the surface that
  // draws it is showing, **the palette is up** or a Studio is open, whose Run
  // starts from it. Closing the palette over a Studio dropped the read and took
  // that Run off (2 Oct 2026), so the one wish here names all three.
  const studyingOne = studying && openStudio !== null;
  useEffect(() => {
    watchCheckoutRunSheet(manifesting || checking || palette.open || studyingOne);
  }, [manifesting, checking, palette.open, studyingOne]);

  // Drift is the surface's own free read on opening, and the palette lists
  // nothing off it. Verify is not here: it is only ever pressed.
  useEffect(() => {
    watchManifestDrift(manifesting);
  }, [manifesting]);

  // Fleet's health. Held for the life of the window rather than only while
  // Overview is showing — Bridge/1088's Fleet panel draws its Doctor read on
  // every surface now.
  useEffect(() => {
    watchOverview(true);
    return () => watchOverview(false);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // The Manifest file and an edit of it. **Held here rather than by the
  // screen**, which unmounts whenever the rail moves: an unsaved correction the
  // screen owned would be gone after one look at the Board.
  // The repository the rail picked. Main holds it, so every read below is taken after main moved.
  const repositories = state.holds.repositories ?? [];
  const repository = state.repository ?? undefined;
  // Whether Fleet has answered the listing. `models` is what says the holdings were read.
  const listed = state.connection.state === "connected" && state.holds.models !== null;
  // Locate, held here so a clone outlives its dialog. A Fleet that serves nothing opens the Cockpit's prompt, not this.
  const locate = useLocate({
    onChooseFolder: chooseFolder,
    onResolveFolder: resolveFolder,
    onAdd: addRepository,
    onClone: cloneRepository,
    landed: state.located,
    onLocated: (one) => {
      pickRepository(one.root);
      goTo(SURFACE.overview);
    },
  });
  const editing = useManifestEditing({
    repository,
    showing: manifesting,
    reading: state.manifestReading,
    onReadFile: readManifestFile,
    onSaveFile: saveManifestFile,
  });
  // The forms beside it, held here for the same reason. A form edit's pull request the forge would not auto-merge is read off the merge line.
  const merging = useMergeByHand(state, openProseLink);
  const form = useManifestForm({
    ...(merging.mergeByHand === undefined ? {} : { mergeByHand: merging.mergeByHand }),
    repository,
    showing: manifesting,
    reading: state.manifestReading,
    onReadFile: readManifestFile,
    onEditManifest: editManifest,
  });

  // Setup, held here for the forms' reason: ticks and the open proposal outlive a look away.
  const setting = useSetup({
    repository,
    showing: manifesting && settingUp,
    onReadScan: readRepositoryScan,
    onReadProposals: readManifestProposals,
    onEditProposal: editManifestProposal,
    onWriteProposal: writeManifestProposal,
  });

  // A notification pressed, or an alert's call: the Cockpit, the overlays down first.
  useSummoned(
    () => (setClearing(false), goTo(SURFACE.overview)),
    (act, sessionId) => {
      if (act.kind === "surface") goTo(act.id);
      else if (act.kind === "link") openProseLink(act.address);
      else if (act.kind === "check") asked.setCheckFocus(sessionCheckId(sessionId, act.run));
      else asked.setLoadoutFocus(sessionId);
    },
  );

  const live = state.connection.state === "connected"; const dispatching = useDispatch({ state, repositories, openSession, openAutopilot: (root) => (state.repository !== root && pickRepository(root), goTo(SURFACE.overview)) }); // One draft for the Cockpit's bar and the panel grown from the title bar.
  const newSession = () => (dispatching.choose("session"), dispatching.openNew()); // A New session entry opens the same panel.
  const sessions = useSessions();
  const helmBox = useHelmBox(state.helm);
  const refsHost: RefsHostProps = { state, repositories, openSession, openLink: openProseLink };
  const helmRefs = useHelmRefs(refsHost);
  const helmHosted = useHelmHosted({
    manifestId: state.helm.state === "none" ? undefined : state.helm.manifestId,
    goes: { onGoTo: goTo, onOpenLink: openProseLink, onOpenCheck: (sessionId, run) => asked.setCheckFocus(sessionCheckId(sessionId, run)), onOpenLoadout: asked.setLoadoutFocus },
    held: { held: state.held, onWant: readHeld },
    openSession,
    words: helmBox.words,
  });
  const routing = useThoughtRouting({ state, repositories, dispatching, openSession, sketchesOf: helmBox.sketchesOf, carryOf: helmBox.carryOf });
  const oneBar = useOneBar({ live, dispatching }); // `n`, ⌘N and ⌘K: the title bar's one bar.
  const firstContact = firstContactOf(state.settings); useUiSounds(state.settings); // Whether a guide opens itself, once Fleet has answered.
  // Which failure is on screen, and which one `Copy debug info` would copy.
  // The order between them, and the reason there is one, are `failing.ts`.
  const { raised, lower, tell, raiseBuildFailed } = useRaised(commands.outcome);
  const dispatchFailure = useDispatchFailure(repositories, openSession, state);
  useTellAsked(tell); // The annotation layer's send that failed.
  useBuildFailed(state.fleetBuild, raiseBuildFailed); // A restart Fleet reports as failed.
  // The only reading of the time App makes: how long an unreachable Fleet has been silent, in its failure.
  const silentFor = useNowWhile(state.connection.state === "unreachable");
  const { statement, fleet, failing } = failingIn({
    connection: state.connection,
    bridge: state.bridge,
    readAt: state.readAt,
    outcome: commands.outcome,
    raised: raised.flatMap((one) => raisedFailure(one, state.bridge) ?? []),
    now: silentFor,
  });
  const guarded = { bridge: state.bridge, onCopied: setCopied };

  /**
   * Go to a place in the rail. **One function, because the rail and the palette
   * are two controls on one act** — a second copy is where one of them gets
   * left behind,
   *
   * A clear-then-set rather than a branch per destination: a branch is where a
   * view gets left standing under the next one.
   */
  function openSession(id: string): void {
    // Helm's own Session is the dock: it points at its repository and opens, and no page is shown.
    const helm = sessions.find((one) => one.id === id && one.helm === true);
    if (helm !== undefined) {
      if (helm.manifestId !== undefined) pointHelm(helm.manifestId);
      askToOpenHelm();
      return;
    }
    // A Session is viewed on the Cockpit only: it is selected there, once the deck holds it.
    goTo(SURFACE.overview);
    askToOpenSessionInCockpit(id);
  }
  /** Setup's reading handed to a new Session in the repository being set up, and that Session opened. */
  function setUpWithSession(held: typeof setting.held, root: string): void {
    if (held.state !== "open") return;
    setStartingSession(true);
    void startSetupSession(held, root, setupSessionFleet, said).then((answer) => {
      setStartingSession(false);
      if (!answer.ok) setTelling(answer.saying);
      if (answer.id !== undefined) openSession(answer.id);
    });
  }
  useOpenSessionAsked(openSession); useOpenHolderAsked({ session: openSession }); useOpenRetroAsked(() => goTo(SURFACE.lessons)); // The annotation layer's Start session; a Session's Retro press.

  function goTo(place: string): void {
    // The Sessions page is the Cockpit's list: ⌘3, a Sessions link and the palette arrive there with focus on it. Jobs has no page.
    // Kit, Mods, Settings, Guides and Workflows are one page now: each opens its entry, and the page is Kit's.
    const entry = entryForSurface(place, systemEntryNow());
    if (entry !== undefined) openSystemEntry(entry);
    const surfaceId = entry !== undefined ? SURFACE.kit : place === SURFACE.sessions || place === SURFACE.jobs ? SURFACE.overview : place;
    if (place === SURFACE.sessions) window.requestAnimationFrame(() => focusPanel("list", deckNow().cursor));
    setClearing(surfaceId === SURFACE.worktrees);
    setManifesting(surfaceId === SURFACE.manifest);
    setSettingsShowing(surfaceId === SURFACE.settings); setModding(surfaceId === SURFACE.mods);
    setKitting(surfaceId === SURFACE.kit);
    setAccounting(surfaceId === SURFACE.account);
    setGuiding(surfaceId === SURFACE.guides);
    setLining(surfaceId === SURFACE.mergeLine);
    asked.setMergeFocus(undefined); asked.setCheckFocus(undefined); asked.setLoadoutFocus(undefined);
    setLearning(surfaceId === SURFACE.lessons); setWorkflowing(surfaceId === SURFACE.workflows); setChecking(surfaceId === SURFACE.checks); setDoctoring(surfaceId === SURFACE.doctor);
    setStudying(surfaceId === SURFACE.studios);
    setReviewing(surfaceId === SURFACE.reviews);
    setUsaging(surfaceId === SURFACE.usage);
    setOpenStudio(null);
    setStudioNode(null);
    if (surfaceId !== SURFACE.manifest) setPicked(null);
    if (surfaceId !== SURFACE.manifest) setSettingUp(false);
  }

  /** A repository is picked the same with or without a Manifest: a Job starts in any folder, and Setup is opened from the repository, never put in front of it. `null` is All. */
  function pick(root: string | null): void {
    pickRepository(root);
  }

  // The picked repository's Manifest, absent until it has one.
  const pickedRepository = repositories.find((one) => one.root === state.repository) ?? null;
  const scoped = pickedRepository?.manifest;
  // A call no Session's row holds pulses the control that already shows it: Sleep's menu and Helm's button here.
  const callingHomes = useCallingHomes(state, pickedRepository);
  // All repositories: no one picked from a listing that has any. The Manifest asks which.
  const all = pickedRepository === null && repositories.length > 0;
  // Worktree Slots follow the pick too.
  const pickedHeld = useMemo(
    () => (state.held.state === "read" ? { ...state.held, held: heldOfPicked(state.held.held, pickedRepository) } : state.held),
    [state.held, pickedRepository],
  );
  // Where the person is, for Helm — #1075.
  const helmScreen = screenOf({
    clearing,
    manifesting,
    studying,
    kitting, workflowing,
    settling: settingsShowing,
  });
  const helmContext: HelmContext = contextOf({
    screen: helmScreen,
    picked: scoped?.id ?? null,
    studio: openStudio?.id ?? null,
    node: studioNode,
  });
  const shownStudio = state.studio.state === "read" && state.studio.studio.id === openStudio?.id ? state.studio.studio : null;
  // Aimed during render rather than in an effect: it is a value this render
  // already knows, and an effect would leave capture a frame behind the Studio.
  const continued =
    openStudio?.editable === true && shownStudio !== null
      ? { id: shownStudio.id, name: studioName(shownStudio) }
      : null;
  if (continued !== null && (continued.id !== captureAim?.id || continued.name !== captureAim.name)) {
    setCaptureAim(continued);
  }
  // Helm's dock lists every repository's questions, whatever the pick.
  const asks = useDockCards(state.questions, repositories, commands);
  const atCockpit = showingOf({ clearing, manifesting, settingsShowing, modding, kitting, guiding, studying, lining, learning, workflowing, checking, doctoring, accounting, reviewing, usaging }) === SURFACE.overview; // Its call is already in front.
  const showingNow = showingOf({ clearing, manifesting, settingsShowing, modding, kitting, guiding, studying, lining, learning, workflowing, checking, doctoring, accounting, reviewing, usaging });
  // Back and forward are keys and nothing on screen — `history.ts`.
  const { back, replaceNext } = useHistory(
    { surface: showingNow, studio: openStudio, studioNode },
    (place) => { goTo(place.surface); setOpenStudio(place.studio); setStudioNode(place.studioNode); },
  );
  // Refresh is a key and a palette row, and nothing on screen.
  useRefreshKey(() => {
    if (live) void commands.refresh();
  });

  return (
    /* The guidance system, over the whole window — #1602, #1603. It holds what
       has been met, and the card it opens is a framed layer, so it belongs
       above every surface rather than inside the one that raised it. */
    <ProseLinks.Provider value={openProseLink}>
      <RefsHost.Provider value={refsHost}>
      <GuidanceProvider onReadAll={() => goTo(SURFACE.guides)} {...(firstContact === undefined ? {} : { firstContact })}>
        <SessionsOwnership onOpen={openSession}>
        <SketchSharing onCopied={setCopied} onSaid={setTelling}>
        <DispatchingProvider value={dispatching}>
        <DispatchFailure.Provider value={dispatchFailure}>
        <Shell
          connection={state.connection}
          repositories={repositories}
          listed={listed}
          scope={state.repository}
          onScope={pick}
          onAddRepository={locate.onOpen}
          onOpenManifest={() => goTo(SURFACE.manifest)}
          onCompose={() => newSession()}
          onSearch={palette.onOpen}
          {...titleRow}
          {...(titleRow.sleep === undefined ? {} : { sleep: { ...titleRow.sleep, calling: callingHomes.has(SLEEP_HOME) } })}
          helmCalling={callingHomes.has(HELM_HOME)}
          wordmark={<ArmadaMark size={20} title="Armada" />}
          // The Cockpit's picker names it there; every other surface has none, so the row does.
          {...(atCockpit || repositories.length === 0 ? {} : { repository: pickedRepository === null ? "All repositories" : repositoryLabel(pickedRepository, repositories) })}
          places={<DeckPlaces state={state} picked={pickedRepository} showing={showingNow} hidden={hidden} away={showingNow === SURFACE.overview && !atCockpit} onGoTo={goTo} onBack={back} onReplaceNext={replaceNext} />}
          asking={asks.length}
          helm={
            <>
            <HelmDock
              asks={asks}
              routing={routing}
              {...(helmBox.box === undefined ? {} : { box: helmBox.box })}
              refs={helmRefs}
              {...(helmHosted.hosted === undefined ? {} : { hosted: helmHosted.hosted })}
              helm={state.helm}
              repositories={repositories}
              live={live}
              onAsk={helmBox.asked}
              outgoing={helmBox.outgoing}
              context={helmContext}
              studio={
                shownStudio === null
                  ? undefined
                  : {
                      name: studioName(shownStudio),
                      ...(studioNode === null ? {} : { node: nodeNamed(shownStudio, studioNode, { servers: state.servers.servers }) }),
                    }
              }
              onSwitch={(manifestId) => pointHelm(manifestId)}
              onReadRecord={helmDebugInfo}
              onCopied={setCopied}
              onSaid={setTelling}
            />
            {helmBox.pad}
            {helmHosted.sheets}
            </>
          }
          // The dock's head carries this, beside Close — the owner's note of
          // 18 Sep 2026. It ends the conversation the whole dock is showing, so
          // it belongs to the dock rather than to the row above the message box,
          // where three controls wrapped onto a second line at the dock's width.
          // Held while a turn runs: Stop is the Session's own control for that.
          helmAction={
            <Button
              variant="ghost"
              size="sm"
              onClick={() => void startHelmFresh()}
              disabled={helmHosted.hosted?.working === true}
            >
              Start fresh
            </Button>
          }
          fleet={{
            ...fleetPanelClocked(state.connection, statement, state.health, state.readAt),
            ...(fleetBuild === undefined ? {} : { build: fleetBuild }), onOpenDoctor: () => goTo(SURFACE.doctor),
            open: fleetOpen,
            onOpenChange: setFleetOpen,
          }}
          onSurface={goTo}
        >
          {/* Real CSS, not utilities: nothing Tailwind spells emits a rule in
              this app, so the class that bounds this box lives in the app's own
              stylesheet where it can be read, and in the components' one so a
              story can check it. `.armada-screen__mounted` says why. */}
          <div className="armada-screen__mounted">
            <Standing
              fleet={fleet} connection={state.connection} bridge={state.bridge} onRestartFleet={restartFleet}
              // **Not while the file is on screen**, which draws the same
              // reading beside the text it is about. Twice at once is two places
              // to read one refusal and one to dismiss while the other stands.
              manifestReading={
                manifesting && editing.view === "file" ? null : state.manifestReading
              }
              readingSeen={readingSeen}
              onReadingSeen={setReadingSeen}
              onCopied={setCopied}
              missed={state.missed}
              acknowledged={acknowledged}
              onAcknowledged={setAcknowledged}
              located={<LocatedNotice locating={locate} repositories={repositories} />}
            />

            {usaging ? (<Boundary region="Usage" {...guarded}><UsageSurface state={state} /></Boundary>) : reviewing ? (<Boundary region="Reviews" {...guarded}><ReviewsSurface picked={pickedRepository} onOpenLink={openProseLink} onOpenSession={openSession} /></Boundary>) : doctoring ? (<Boundary region="Doctor" {...guarded}><DoctorSurfaceNow health={state.health} connection={state.connection} said={statement.headline} onCheck={() => watchOverview(true)} /></Boundary>) : lining ? (<MergeLineSurface state={state} {...guarded} onOpenLink={openProseLink} onOpenSession={openSession} onSendToSession={(id, text) => void commands.sendToSession(id, text)} {...(asked.mergeFocus === undefined ? {} : { focus: asked.mergeFocus })} />) : checking ? (<ChecksSurface key={state.repository ?? ""} state={state} onOpenMergeLine={(branch) => { goTo(SURFACE.mergeLine); asked.setMergeFocus(branch); }} {...guarded} />) : learning ? (
              <LessonsSurface repository={state.repository} {...guarded} />
            ) : clearing ? (
              /* What Fleet is holding disk for. The half of the reclaim rule that is a person's: Fleet has
                 already taken back everything it could prove nobody needs, and this is where the rest is
                 given back, one tile at a time. */
              <Boundary region="Cleanup" {...guarded}>
                <WorktreesNow
                  held={pickedHeld}
                  onWant={readHeld}
                  onClose={() => setClearing(false)}
                  onCopied={setCopied}
                  {...slotActs}
                />
              </Boundary>
            ) : manifesting && all ? (
              <AskRepository
                repositories={repositories}
                title="Pick a repository to open its Manifest"
                next="A Manifest belongs to one repository. Picking it focuses the Cockpit there."
                onPick={pick}
              />
            ) : manifesting ? (
              /* Everything this repository's Manifest declares, and one press that runs one of them in the
                 checkout as it is on disk: a person can run this project's lint with nothing else running. */
              <Boundary region="the manifest" {...guarded}>
                <>
                <ManifestNow
                  // Another repository is another page: its runs, its allows and a dismissed Verify are not this one's.
                  key={state.repository ?? ""}
                  sheet={state.checkoutRunSheet}
                  followed={state.checkoutRunFollowed}
                  picked={picked}
                  onSaid={setTelling}
                  editing={editing}
                  form={form}
                  onObserveRun={observeCheckoutRun}
                  onStartRun={startCheckoutRun}
                  drift={state.manifestDrift}
                  onStartVerify={startCheckoutVerify}
                  onStopRun={stopCheckoutRun}
                  onUndoRun={undoCheckoutRun}
                  onListRuns={listCheckoutRuns}
                  onGetRunOutput={getCheckoutRunOutput}
                  onGetRunDiff={getCheckoutRunDiff}
                  onListRepositoryAllowedCommands={listRepositoryAllowedCommands}
                  onRemoveRepositoryAllowedCommand={removeRepositoryAllowedCommand}
                  // No Job means the main checkout.
                  onStartServer={(name) => startServer(name)}
                  onStopServer={stopServer}
                  onOpenServerLink={openServerLink}
                  settingUp={settingUp}
                  onSettingUp={setSettingUp}
                  setUp={repository === undefined || scoped !== undefined}
                  startingSession={startingSession}
                  {...(setting.held.state !== "open" || repository === undefined
                    ? {}
                    : { onSetUpWithSession: () => setUpWithSession(setting.held, repository) })}
                  setup={
                    <SetupNow
                      setting={setting}
                      sheet={state.checkoutRunSheet}
                      onStartVerify={startCheckoutVerify}
                      onStopRun={stopCheckoutRun}
                      onOpenEdit={() => {
                        setSettingUp(false);
                        editing.onView("form");
                      }}
                      floor={floor}
                    />
                  }
                />
                {merging.panel}
                </>
              </Boundary>
            ) : studying ? (
              <StudiosSurfaceNow
                state={state}
                live={live}
                repositories={repositories}
                manifestId={scoped?.id}
                all={all}
                // Nothing set up anywhere: the Manifest surface is where a
                // repository is picked and Setup is opened from it — so this
                // hands over to that route rather than cutting a second one
                // from here.
                onSetUp={() => goTo(SURFACE.manifest)}
                open={openStudio}
                onOpenChange={setOpenStudio}
                selectedNode={studioNode}
                onSelectNode={setStudioNode}
                // A server node reads the live holder and counts its uptime on
                // the window's one clock — #1345.
                onCopied={setCopied}
              />
            ) : kitting ? (
              /* System: Kit, Fleet and Bridge in one sidebar, the picked entry beside it. Kit's reading is this
                 machine's and answers for every repository; the pick only names Kit's second tier. */
              <SystemSurface
                // Another repository is another second tier. The machine-wide half is the same; what it resolves to is not.
                key={state.repository ?? ""}
                repository={all || pickedRepository === null ? null : repositoryLabel(pickedRepository, repositories)}
                settings={state.settings}
                modelChoices={state.holds.models}
                live={live}
                guarded={guarded}
                onReadGuides={() => goTo(SURFACE.guides)}
                onOpenAccount={() => goTo(SURFACE.account)}
                workflows={<WorkflowCreatorSurface state={state} {...guarded} />}
                guides={<GuideCatalogue narrow={narrow} floor={floor} listWidth={guideList} onResizeList={resizeGuideList} />}
                warned={workflowsWarned(state.health)}
                onReadKitInventory={readKitInventory}
                onRemoveKitAllowedCommand={removeKitAllowedCommand}
                onListKitServers={listKitServers}
                onAddKitServer={addKitServer}
                onForgetKitServer={forgetKitServer}
                onSetKitServerReach={setKitServerReach}
                onSetManifestServerReach={setManifestServerReach}
                // Read off the bridge itself, since only the mock has them yet.
                onInstallKitPiece={window.armada.installKitPiece}
                onRemoveKitPiece={window.armada.removeKitPiece}
                onRestoreKitPiece={window.armada.restoreKitPiece}
              />
            ) : accounting ? (
              <Boundary region="Account" {...guarded}><AccountPage onSettings={() => goTo(SURFACE.settings)} machine={machineOf(state.settings)} /></Boundary>
            ) : (
              <Overview
                state={state}
                repositories={repositories}
                disconnected={live ? null : statement.headline}
                onCopied={setCopied}
                bring={lands.bringing}
                onBrought={lands.brought}
                onOpenLink={openProseLink}
                onSendToSession={(id, text) => void commands.sendToSession(id, text)}
                onOpenSession={openSession}
                onWantHeld={readHeld}
                onPickRepository={pick}
                onAddRepository={locate.onOpen}
                onGoTo={goTo}
                onOpenCheck={(sessionId, run) => asked.setCheckFocus(sessionCheckId(sessionId, run))}
                onOpenLoadout={asked.setLoadoutFocus}
                onReleaseSlot={(manifestId, slot, holder) => slotActs.onChangeSlotPool(manifestId, { act: "release", slot, holder })}
                onRescueSlot={slotActs.onRescueSlot}
                bayActs={bayActs}
                onTell={tell}
              />
            )}
          </div>
        </Shell>

        {/* Studio capture — #1290. Last, so its layer paints over every surface
            and every overlay the shell draws under it. */}
        <CaptureLayer aim={captureAim} onCapture={captureStudioNote} />

        <Locate locating={locate} />

        <DispatchSheet />
        <TitleDispatchedStrip />

        <PaletteMount
          open={palette.open} onClose={palette.onClose} dispatch={oneBar}
          shownStudio={shownStudio}
          surfaces={SURFACES.filter((one) => !hidden.includes(one.id))}
          checkoutRunSheet={state.checkoutRunSheet}
          failing={failing}
          live={live}
          repositories={repositories.map((one) => ({ root: one.root, label: repositoryLabel(one, repositories) }))}
          picked={state.repository}
          acts={{
            pickRepository: pick,
            compose: () => newSession(),
            surface: goTo,
            run: (entryId) => {
              goTo(SURFACE.manifest);
              setPicked(entryId);
            },
            copyDebugInfo: () => {
              if (failing !== null) copyDebugInfoFor(failing, setCopied);
            },
            openSetting,
            refresh: () => void commands.refresh(),
          }}
        />

        <Toasts
          raised={raised}
          bridge={state.bridge}
          onLower={lower}
          copied={copied}
          said={telling}
          onCopied={setCopied}
          onDispatchSession={dispatchFailure}
        >
          <DownshiftAlert choose={openModelSetting} />
          <DeckHost state={state} picked={pickedRepository} here={atCockpit} hidden={hidden} onGoTo={goTo} onOpenSession={openSession} />
          {asked.loadoutFocus === undefined ? null : <LoadoutPanel id={asked.loadoutFocus} onClose={() => asked.setLoadoutFocus(undefined)} />}
          <CallAlerts state={state} picked={pickedRepository} onCockpit={atCockpit} onShow={(key) => (goTo(SURFACE.overview), lands.bring(key))} />
        </Toasts>
        </DispatchFailure.Provider>
        </DispatchingProvider>
        </SketchSharing>
        </SessionsOwnership>
      </GuidanceProvider>
      </RefsHost.Provider>
    </ProseLinks.Provider>
  );
}
