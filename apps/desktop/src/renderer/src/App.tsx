// Two screens and one piece of state between them: the board — what Fleet is, a
// form to propose a Job, and every Job — and one Job read whole. A row is the
// control that opens a detail; Escape and one button close it. No router: a
// list and a detail need which one is open and nothing else.
//
// Everything drawn comes from the state the main process publishes over the one
// connection. Nothing here fetches, and nothing here holds a copy of a Job that
// Fleet has not confirmed — a Job whose real state is not what the screen says
// is the failure that matters.
//
// # What this file is, now that three subjects have left it
//
// What is left is the window: which surface is open, what keeps that consistent
// as events arrive, and what is drawn. Three things that are not the window are
// beside it, each because it is a subject rather than a line of wiring —
// `commands.ts` for what the host is asked to do, `failing.ts` for which
// failure is on screen, and `palette.ts` for what the palette can reach.

import { useCallback, useEffect, useMemo, useState } from "react";
import { dockCardsOf, jobNumber, ofPicked, whyNotOpenedLink } from "@armada/screens";
import type { Outstanding } from "@armada/screens";
import type { HelmContext, JobSummary, LandCheckAt } from "@armada/protocol";
import { useDockAnswering } from "./dock-answering";
import { chippedJobId, contextOf, cursorRowFor, dismissed, HelmDock, helmReplying, NO_CHIP, opened, screenOf } from "@armada/helm";
import type { ChipState } from "@armada/helm";
import { Button, GuidanceProvider, GuideCatalogue, ProseLinks } from "@armada/components";

import { NOTHING_YET } from "../../shared/bridge";
import type { BridgeState } from "../../shared/bridge";
import { Boundary } from "@armada/shell";
import { Standing } from "./Standing";
import { useCopied, useSaid } from "@armada/shell";
import { FailureBlock } from "@armada/shell";
import { jobFailure } from "@armada/shell";
import { SweepButtons, SweepDialogs, sweepsOf, useRefreshKey, type Sweep } from "@armada/shell";
import { repositoryLabel } from "@armada/shell";
import { AskRepository } from "@armada/screens";
import { BridgeSettings, ModsSurface } from "@armada/settings";
import { Kit } from "@armada/manifest";
import { Reports } from "@armada/screens";
import { Composing } from "./Composing";
import { useEscapeLeavesJob, useNow, useReturnToRow, useSummoned } from "./app-effects";
import { aJobAct, ConfirmAct, type Confirming } from "./ConfirmAct";
import { PaletteMount } from "./PaletteMount";
import { FLEET_DOWN } from "./palette";
import { Overview, useDispatchBarKeys } from "./Overview";
import { CaptureLayer, type CaptureAim } from "./capture/Layer";
import { StudiosSurface } from "./StudiosSurface";
import type { SketchOpening } from "@armada/screens/src/draft/sketch";
import { nodeNamed, studioName } from "@armada/screens";
import type { OpenStudio } from "@armada/studios";
import { Worktrees } from "@armada/cleanup";
import { Manifest, useManifestEditing, useManifestForm } from "@armada/manifest";
import { Setup, useSetup } from "@armada/setup";
import { Locate, LocatedNotice, useLocate } from "@armada/setup";
import { JobDetail, LandCheckLogSheet } from "@armada/jobs";
import type { JobDraft } from "@armada/jobs/draft/held";
import { failingIn, raisedFailure } from "./failing";
import { Toasts, useRaised } from "./raised";
import {
  examine,
  openArtifact,
  openPullRequest,
  openFindingIssue,
  openRemarkLink,
  openServerLink,
  runSheetServers,
  openLink, restartFleet, changeFleetBuild,
  observeRun,
  observeCheckoutRun,
  pickRepository,
  chooseFolder,
  resolveFolder,
  addRepository,
  cloneRepository,
  askHelm,
  helmDebugInfo,
  startHelmFresh,
  pointHelm,
  getRunOutput,
  getCheckoutRunOutput,
  getCheckoutRunDiff,
  readManifestFile,
  saveManifestFile,
  editManifest,
  readManifestSpend,
  readRepositoryScan,
  readManifestProposals,
  editManifestProposal,
  writeManifestProposal,
  listRepositoryAllowedCommands,
  removeRepositoryAllowedCommand,
  listKitServers,
  readKitInventory,
  removeKitAllowedCommand,
  addKitServer,
  forgetKitServer,
  setKitServerReach,
  setManifestServerReach,
  listRuns,
  listCheckoutRuns,
  undoRun,
  undoCheckoutRun,
  explainCommand,
  readCheckOutput,
  readBrief,
  readRetro,
  agreeLesson,
  disagreeLesson,
  followCheckOutput,
  readFrame,
  frameSrc,
  readDiff,
  readEvidence,
  readRemarks,
  watchPulse,
  captureStudioNote,
  readHeld,
  readReports,
  slotActs,
  showAgain,
  startRun,
  startCheckoutRun,
  startCheckoutVerify,
  startServer,
  stopRun,
  stopCheckoutRun,
  stopServer,
  followLandCheck,
  useCommands,
  useWatching,
  watchRunSheet,
  watchCheckoutRunSheet,
  watchManifestDrift,
  watchOverview,
} from "./commands";
import { useAddedBinding, useAlerts } from "./added-steps";
import { useDrafted } from "./drafted";
import { hiddenSurfaces, MergeLineSurface } from "./merge-line";
import { LessonsSurface } from "./lessons";
import { ChecksSurface, useAsked } from "./checks-surface";
import { SessionsOwnership, SessionsSurface, sessionsHidden } from "./sessions"; import { useSessionsDraft } from "./sessions-draft";
import { useOpenSessionAsked } from "./open-session";
import { useTellAsked } from "./tell";
import { openingOf, useHistory, useJobTab } from "./history"; import { showingOf } from "./showing"; import { WorkflowCreatorSurface, workflowsWarned } from "./workflow-creator";
import { useWhereOpen } from "./where-open";
import { usePlanView } from "./remembered-views";
import { usePanelOpen } from "./panel-open";
import { useGuideListWidth } from "./guide-list-width";
import { fleetPanelOf } from "./left-column";
import { fleetBuildOf, useFleetBuild } from "./fleet-build";
import { copyDebugInfoFor, useCommandPalette } from "@armada/shell";
import { Shell, SURFACE, SURFACES, useAtFloor, useNarrow, useSurfaceKeys } from "@armada/shell";

/** Re-exported so nothing importing it has to learn a new path. */
export const WAITING: BridgeState = NOTHING_YET;

/**
 * What the window is handed that Fleet does not serve yet — `#1532`'s draft
 * schema, imported by its own path so the rule keeping it out of the main
 * process can still see the reach. **Only the mock ever fills it**: a real
 * Bridge mounts `<App />` with nothing, and every destination that takes a
 * draft derives what today's wire can answer instead. It goes at `#1545`.
 */
export type AppProps = { draft?: JobDraft };

export function App({ draft }: AppProps = {}) {
  const [state, setState] = useState<BridgeState>(WAITING);
  // What has been read and acknowledged. The count itself belongs to the
  // connection and is never reset from here — a drop that happened, happened.
  const [acknowledged, setAcknowledged] = useState(0);
  const now = useNow();
  const [copied, setCopied] = useCopied();
  // What the app is telling somebody, as a sentence it already wrote. Today
  // that is only an open that did not happen; a click ending in nothing on
  // screen is the defect the openable records were added against.
  const [telling, setTelling] = useSaid();
  // A link in a model's text, for every Prose in the window. A refusal is said, never a dead click.
  const openProseLink = useCallback(
    (address: string) =>
      void openLink(address).then((followed) => {
        const why = whyNotOpenedLink(followed);
        if (why !== null) setTelling(why);
      }),
    [setTelling],
  );
  // What a boundary could never catch: a throw in a handler, and a rejected
  // promise from a `void`-ed preload call.
  // **The whole of navigation.** A list and a detail need one piece of state,
  // not a router: which Job is open, or none. The row is the control that sets
  // it and Escape is what clears it.
  const [openJob, setOpenJob] = useState<string | null>(null);
  const asked = useAsked(openJob, state, () => goTo(SURFACE.mergeLine)); // Where the Checks page's requester links sent a person. `checks-surface.tsx`.
  // The section a pressed notification asked for. **A token rather than a
  // call**: the press may have arrived over the composer or over a Job, so
  // Overview is not mounted yet, and it opens and scrolls to the section once
  // it is. `at` is what makes a second press of the same section land.
  const [landing, setLanding] = useState<{ section: "needs-you"; at: number } | null>(null);
  // Whether the composer is open. It used to sit permanently above the list;
  // `New job` is what opens it now, so the surface is the list until somebody
  // asks for the form — **or until a moment being replayed hands the window a
  // request already half typed**, which is the one thing that can be true
  // before anybody has pressed anything. `drafted.tsx`.
  const [composing, setComposing] = useState(useDrafted().prompt !== undefined);
  const [composedFrom, setComposedFrom] = useState<SketchOpening>(); // A Sketch dispatched from a Studio.
  const [seed, setSeed] = useState<string>(); // Words typed into the Dashboard's quick box.
  useEffect(() => void (composing || (setComposedFrom(undefined), setSeed(undefined))), [composing]);
  // What has been reported against the Judge. Its own view: a report is filed
  // about one Job and the rate is read across all of them.
  const [auditing, setAuditing] = useState(false);
  // Whether the held worktrees are open. **Its own view for the reports' kind
  // of reason and not the same one**: what is decided there is which of a set
  // to give back, which no Job row can be asked, and putting a disk decision on
  // the Board would put a control nobody can act on beside rows that exist to
  // be acted on.
  const [clearing, setClearing] = useState(false);
  // Which bulk sweep is asking to be confirmed. Here rather than on Cleanup,
  // because the palette can ask for one from any surface.
  const [sweep, setSweep] = useState<Sweep | null>(null);
  // Whether Settings is open — a rail surface since #1089, the sheet it
  // replaced having lost its own opener when the status bar went (#1088).
  const [settingsShowing, setSettingsShowing] = useState(false); const [modding, setModding] = useState(false);
  // Whether Kit is open — a rail surface since #1275, at `⌘8`. Machine-wide,
  // and the rail's pick is what names its second tier.
  const [kitting, setKitting] = useState(false);
  // Whether the guide catalogue is open — a rail surface since #1602, and the
  // last row, so it carries no digit. Every guide, numbered and grouped,
  // readable without the screen that raised any of them.
  const [guiding, setGuiding] = useState(false);
  const [lining, setLining] = useState(false); // The merge line's own surface. `merge-line.tsx`.
  const [learning, setLearning] = useState(false); const [checking, setChecking] = useState(false); const [workflowing, setWorkflowing] = useState(false); // Lessons, every Job's retro items (`lessons.tsx`), and the Workflow creator (`workflow-creator.tsx`).
  const [sessioning, setSessioning] = useState(false); const [sessionOpen, setSessionOpen] = useState<string | null>(null); // Sessions (`sessions.tsx`), and the one open on it.
  const hidden = [...hiddenSurfaces(state), ...sessionsHidden(useSessionsDraft() !== undefined)]; // Left off the rail and the palette.
  // Whether the Manifest surface is open — Journey 9's *Running one*. **Its
  // own view, and it needs no Job to draw**: it is read off the file Fleet
  // already holds, which is what lets a person run this project's lint with
  // the Board empty.
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
  // The row the cursor goes back to when the detail closes. A keyboard that
  // opened a row and came back to the top of the document has lost its place.
  const [returning, setReturning] = useState<string | null>(null);
  // Which act is waiting to be confirmed. **Nothing destructive happens on one
  // press** — every one of them ends something, so each states what happens and
  // what survives first.
  const [confirming, setConfirming] = useState<Confirming | null>(null);
  // What a person typed into the restart confirmation, which is the one
  // confirmation that collects anything. **Held beside `confirming` rather than
  // inside it**: the act being confirmed is what the palette and the step
  // header both set, and neither of them knows about a field.
  const [restartNote, setRestartNote] = useState("");
  /** The Manifest reading a person put away. A later read draws again. */
  const [readingSeen, setReadingSeen] = useState<string | null>(null);
  // Whether the command palette is up, and where Overview's cursor is.
  // **The cursor is mirrored, not owned** — `OverviewLists` holds it and
  // reports it up (#1075), so the palette can title its context block with the
  // job its acts would act on. Two cursors would drift.
  const palette = useCommandPalette();
  const [cursor, setCursor] = useState<string | null>(null);
  // The Job chipped above Helm's message box — #1075. Opening a Job's detail
  // chips it and points Helm at its repository; leaving the Job or its own ×
  // drops the chip, and reopening the Job restores it. `helm-context.ts` is
  // the fold, tested on its own.
  const [chip, setChip] = useState<ChipState>(NO_CHIP);
  // Every command the window can send, and what it holds while one is out.
  // Two of them end somewhere this file owns, so both are answered to rather
  // than reached for: a redispatch opens its replacement, and a re-read
  // publishes what came back.
  const commands = useCommands({
    onOpen: setOpenJob,
    onRead: setState,
    jobs: state.jobs,
    onPaused: (jobId) => setConfirming({ act: "resume_job", jobId, pressed: true }),
  });
  // Whether the window is at `--window-floor`, `JobDetail`'s own reading —
  // Fleet settings is the same trailing layer and takes it the same way.
  const floor = useAtFloor();
  const narrow = useNarrow();
  // Where things are' own open choice — held locally so a press moves it at
  // once, `#927`'s round trip off the critical path of a toggle.
  const [whereOpen, pressWhereOpen] = useWhereOpen(state.preferences.where_things_are_open);
  // Graph or list on Plan, this window's.
  const [planView, pressPlanView] = usePlanView();
  // The left column's own fold, remembered across a restart — Bridge/1088.
  const [fleetOpen, setFleetOpen] = usePanelOpen("fleet");
  // The mock provides a fixture; a real window builds it from what Fleet reported.
  const fleetBuild = useFleetBuild() ?? fleetBuildOf(state.fleetBuild, state.jobs, (build, adopt) => void changeFleetBuild(build, adopt));
  // The catalogue list's width, remembered the same way the shell's column is.
  const [guideList, resizeGuideList] = useGuideListWidth();

  // The open Job, read out of the list rather than copied beside it. A Job that
  // leaves the list — superseded, or gone from a resync — closes its own detail
  // rather than leaving a row on screen that Fleet no longer has.
  const reading = openJob === null ? null : (state.jobs.find((job) => job.id === openJob) ?? null);
  const added = useAddedBinding(commands, reading?.id ?? null); const alerting = useAlerts(state, (jobId, to) => { asked.setOpening({ jobId, to }); setOpenJob(jobId); });
  // Main's log, opened from a Job that took main's red: the merge line head's own log, held here
  // because the Job's detail is not where that panel lives.
  const [mainLog, setMainLog] = useState<LandCheckAt | null>(null);
  useEffect(() => {
    void window.armada.state().then(setState);
    return window.armada.subscribe(setState);
  }, []);

  // What main is asked to hold open for the Job being read: the Job itself, what
  // it holds on this machine, and its turns.
  useWatching(openJob);

  // Opening or leaving a Job's detail folds the chip and, on opening, points
  // Helm at that Job's repository without moving the rail's own pick — #1075.
  // **Keyed on `openJob` alone.** `chip` is read as of the render this ran
  // in, not listed as a dependency: the chip's own × must never re-run this
  // and re-point Helm or un-dismiss what was just dismissed.
  useEffect(() => {
    const target = reading === null ? null : { id: reading.id, manifestId: reading.owner_manifest_id };
    const { state: next, point } = opened(chip, target);
    setChip(next);
    if (point !== null) pointHelm(point);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [openJob]);

  // `⌘1`…`⌘n`, the binding the contract publishes and nothing answered until
  // the Manifest surface needed `⌘5`. One roster, read by the rail, the
  // palette and now the keyboard.
  useSurfaceKeys(goTo); useDispatchBarKeys(() => goTo(SURFACE.overview)); // `n` and ⌘N: the Dashboard's dispatch bar.

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

  // Fleet's health and every repository's drift in scope. Held for the life
  // of the window rather than only while Overview is showing — Bridge/1088's
  // Fleet panel draws its Doctor read on every surface now.
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
  // Locate, held here so a clone outlives its dialog, and opened by itself on a rail Fleet listed empty.
  const locate = useLocate({
    onChooseFolder: chooseFolder,
    onResolveFolder: resolveFolder,
    onAdd: addRepository,
    onClone: cloneRepository,
    nothingServed: listed && repositories.length === 0,
    landed: state.located,
    onLocated: (one) => {
      pickRepository(one.root);
      goTo(SURFACE.manifest);
      setSettingUp(true);
    },
  });
  const editing = useManifestEditing({
    repository,
    showing: manifesting,
    reading: state.manifestReading,
    onReadFile: readManifestFile,
    onSaveFile: saveManifestFile,
  });
  // The forms beside it, held here for the same reason.
  const form = useManifestForm({
    repository,
    showing: manifesting,
    reading: state.manifestReading,
    onReadFile: readManifestFile,
    onEditManifest: editManifest,
    onReadSpend: readManifestSpend,
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

  useSummoned((jobId) => {
    setComposing(false);
    setAuditing(false);
    setClearing(false);
    setOpenJob(jobId);
    if (jobId === null) setLanding({ section: "needs-you", at: Date.now() });
  });

  useEscapeLeavesJob(openJob, close);

  useReturnToRow(returning, () => setReturning(null));

  // The Job every palette act acts on: the one read whole, or the one under
  // Overview's cursor. In that order, because a Job open on screen is
  // unambiguously what is in front of you.
  const onWhat = reading ?? state.jobs.find((job) => job.id === cursor);
  const live = state.connection.state === "connected";
  // Which failure is on screen, and which one `Copy debug info` would copy.
  // The order between them, and the reason there is one, are `failing.ts`.
  const { raised, lower, tell } = useRaised(commands.outcome);
  useTellAsked(tell); // The annotation layer's send that failed.
  const { statement, fleet, failing } = failingIn({
    connection: state.connection,
    bridge: state.bridge,
    readAt: state.readAt,
    outcome: commands.outcome,
    raised: raised.flatMap((one) => raisedFailure(one, state.bridge) ?? []),
    now,
  });
  const guarded = { bridge: state.bridge, onCopied: setCopied };
  function close(): void {
    setReturning(openJob);
    setOpenJob(null);
  }

  /** Send what the dialog collected. A pause or a resume stays up until Fleet takes it, to say what it refused. */
  function confirmed(what: Confirming): void {
    const stays = what.act === "pause_job" || what.act === "resume_job";
    if (!stays) setConfirming(null);
    void commands.confirmed(what, restartNote).then((sent) => {
      if (stays && sent) setConfirming(null);
    });
    setRestartNote("");
  }

  /**
   * Go to a place in the rail. **One function, because the rail and the palette
   * are two controls on one act** — a second copy is where one of them gets
   * left behind, which is how `auditing` came to survive a rail press.
   *
   * A clear-then-set rather than a branch per destination: a branch is where a
   * view gets left standing under the next one.
   */
  function openSession(id: string): void {
    goTo(SURFACE.sessions);
    setSessionOpen(id);
  }
  useOpenSessionAsked(openSession); // The annotation layer's Start session.

  function goTo(surfaceId: string): void {
    setOpenJob(null);
    setComposing(false);
    setAuditing(false);
    setClearing(surfaceId === SURFACE.worktrees);
    setManifesting(surfaceId === SURFACE.manifest);
    setSettingsShowing(surfaceId === SURFACE.settings); setModding(surfaceId === SURFACE.mods);
    setKitting(surfaceId === SURFACE.kit);
    setGuiding(surfaceId === SURFACE.guides);
    setLining(surfaceId === SURFACE.mergeLine);
    asked.setMergeFocus(undefined);
    setLearning(surfaceId === SURFACE.lessons); setWorkflowing(surfaceId === SURFACE.workflows); setChecking(surfaceId === SURFACE.checks);
    setStudying(surfaceId === SURFACE.studios);
    setSessioning(surfaceId === SURFACE.sessions);
    setSessionOpen(null);
    setOpenStudio(null);
    setStudioNode(null);
    if (surfaceId !== SURFACE.manifest) setPicked(null);
    if (surfaceId !== SURFACE.manifest) setSettingUp(false);
  }

  /**
   * Go back to the Studio a Job was dispatched from, landing on its own node —
   * #1362. **The Job closes**: this is leaving the Job for the whiteboard it
   * came from, not opening a second thing over it, and `goTo` is already the
   * clear-then-set every other destination goes through.
   *
   * **Read-only, like opening one from the list.** Rereading is not editing,
   * and Continue is what makes a Studio editable.
   */
  function openStudioFrom(studioId: string, nodeId: string): void {
    // A Studio belongs to one repository and the surface asks for one on All,
    // so the Job's own repository is picked first — otherwise the press lands
    // on the picker rather than on the whiteboard.
    const owner = repositories.find((one) => one.manifest?.id === reading?.owner_manifest_id);
    if (owner !== undefined && state.repository !== owner.root) pickRepository(owner.root);
    goTo(SURFACE.studios);
    setOpenStudio({ id: studioId, editable: false });
    setStudioNode(nodeId);
  }

  /** A repository nobody set up opens on Setup when picked: there is nothing else to do with it yet. `null` is All. */
  function pick(root: string | null): void {
    pickRepository(root);
    if (root === null || repositories.find((one) => one.root === root)?.manifest !== undefined) return;
    goTo(SURFACE.manifest);
    setSettingUp(true);
  }

  // What a new Job is proposed against: the picked repository's Manifest, absent until it has one.
  const pickedRepository = repositories.find((one) => one.root === state.repository) ?? null;
  const scoped = pickedRepository?.manifest;
  // All repositories: no one picked from a listing that has any. New job and the Manifest ask which.
  const all = pickedRepository === null && repositories.length > 0;
  // The Board's Jobs follow the pick. The status bar, the palette and held worktrees read every Job.
  const boardJobs = useMemo(() => ofPicked(state.jobs, pickedRepository), [state.jobs, pickedRepository]);
  // Where the person is, for Helm — #1075. `cursorRowFor` picks the Board's
  // or Overview's row by which screen is showing, so neither's stale row
  // reaches an ask made on the other.
  const helmScreen = screenOf({
    reading: reading !== null,
    clearing,
    manifesting,
    studying,
    kitting, workflowing,
    settling: settingsShowing,
  });
  const chippedJob = state.jobs.find((job) => job.id === chippedJobId(chip));
  const helmContext: HelmContext = contextOf({
    screen: helmScreen,
    picked: scoped?.id ?? null,
    chip: chippedJob?.id ?? null,
    cursor: cursorRowFor({ screen: helmScreen, overview: cursor }),
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
  // Helm's dock lists every repository's questions, whatever the pick. Answering is #936, Helm #944.
  const dockAnswering = useDockAnswering(commands);
  // "Discuss with Helm" points it at the card's own repository. The picker never moves for it.
  const onDiscussHelm = useCallback(
    (_question: Outstanding, job: JobSummary) => pointHelm(job.owner_manifest_id),
    [],
  );
  // `questions` is the dock's own block; `asks` is Helm's, drawn at the end of
  // its thread where the reply they stopped is — #1519.
  const { questions, asks } = useMemo(
    () => dockCardsOf(state.questions, state.jobs, repositories, now, { ...dockAnswering, onDiscuss: onDiscussHelm }),
    [state.questions, state.jobs, repositories, now, dockAnswering, onDiscussHelm],
  );
  // Back and forward are keys and nothing on screen — `history.ts`.
  const [jobAt, onJobWhere] = useJobTab(openJob);
  useHistory(
    { surface: showingOf({ clearing, manifesting, settingsShowing, modding, kitting, guiding, studying, lining, learning, workflowing, checking, sessioning }), job: openJob, ...jobAt, session: sessionOpen, studio: openStudio, studioNode },
    (place) => { goTo(place.surface); setOpenJob(place.job); if (place.job !== null) asked.setOpening({ jobId: place.job, to: openingOf(place) }); setSessionOpen(place.session); setOpenStudio(place.studio); setStudioNode(place.studioNode); },
    (place) => place.job === null || state.jobs.some((job) => job.id === place.job),
  );
  // Refresh is a key and a palette row, and nothing on screen.
  useRefreshKey(() => {
    if (live) void commands.refresh();
  });
  // What the Overview menu carried and nothing else did, now in the palette:
  // Reported is drawn there and nowhere else.
  const busy = live ? (commands.sweeping === null ? undefined : "waiting on Fleet") : FLEET_DOWN;
  const boardRows = [
    { id: "reports", label: "Reported" },
    ...sweepsOf(boardJobs).map((one) => ({
      ...one,
      ...(one.id === "forget" ? { destructive: true } : {}),
      ...(busy === undefined ? {} : { dormant: busy }),
    })),
  ];

  return (
    /* The guidance system, over the whole window — #1602, #1603. It holds what
       has been met, and the card it opens is a framed layer, so it belongs
       above every surface rather than inside the one that raised it. */
    <ProseLinks.Provider value={openProseLink}>
      <GuidanceProvider onReadAll={() => goTo(SURFACE.guides)}>
        <SessionsOwnership onOpen={openSession}>
        <Shell
          hidden={hidden} warned={workflowsWarned(state.health)}
          connection={state.connection}
          repositories={repositories}
          listed={listed}
          scope={state.repository}
          onScope={pick}
          onAddRepository={locate.onOpen}
          onOpenManifest={() => goTo(SURFACE.manifest)}
          onCompose={() => setComposing(true)}
          onSearch={palette.onOpen}
          questions={questions}
          asking={asks.length}
          helm={
            <HelmDock
              asks={asks}
              helm={state.helm}
              repositories={repositories}
              jobs={state.jobs}
              workflows={state.holds.workflows}
              live={live}
              chip={chippedJob === undefined ? undefined : { jobHandle: jobNumber(chippedJob), title: chippedJob.title }}
              onRemoveChip={() => setChip(dismissed)}
              onAsk={(text, context) => void askHelm(text, context)}
              context={helmContext}
              studio={
                shownStudio === null
                  ? undefined
                  : {
                      name: studioName(shownStudio),
                      ...(studioNode === null ? {} : { node: nodeNamed(shownStudio, studioNode, state.jobs, {
                            servers: state.servers.servers,
                            now,
                          }) }),
                    }
              }
              onSwitch={(manifestId) => pointHelm(manifestId)}
              onReadRecord={helmDebugInfo}
              onCopied={setCopied}
              onSaid={setTelling}
              onApprove={commands.approve}
            />
          }
          // The dock's head carries this, beside Close — the owner's note of
          // 18 Sep 2026. It ends the conversation the whole dock is showing, so
          // it belongs to the dock rather than to the row above the message box,
          // where three controls wrapped onto a second line at the dock's width.
          // Refused while a reply is being written: Fleet's own rule, read
          // through the same helper the thread below reads it with.
          helmAction={
            <Button
              variant="ghost"
              size="sm"
              onClick={() => void startHelmFresh()}
              disabled={helmReplying(state.helm)}
            >
              Start fresh
            </Button>
          }
          fleet={{
            ...fleetPanelOf(state.connection, statement, state.health, now, state.readAt),
            ...(fleetBuild === undefined ? {} : { build: fleetBuild }),
            open: fleetOpen,
            onOpenChange: setFleetOpen,
          }}
          // Which row the rail marks — `showing.ts`.
          showing={showingOf({
            clearing, manifesting, settingsShowing, modding, kitting, guiding, studying, lining, learning, workflowing, checking, sessioning,
          })}
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
              givenBack={commands.givenBack}
              onGivenBack={commands.setGivenBack}
              taken={commands.taken}
              located={<LocatedNotice locating={locate} repositories={repositories} />}
            />

            {/* One Job, read whole, in place of the board. Reviewing and deciding
                is one loop, so the detail is not a panel beside the list — and the
                list is what Escape and the rail's own Job Board row both return to. */}
            {reading !== null ? (
              // Keyed by the Job, so a render that threw on one Job is not the
              // failure notice drawn over the next one opened.
              <Boundary key={reading.id} region="the job detail" {...guarded}>
                <JobDetail
                  job={reading}
                  {...(added === undefined ? {} : { added })}
                  {...(asked.opening?.jobId === reading.id ? { opening: asked.opening.to } : {})}
                  onWhere={(tab, item) => onJobWhere(reading.id, tab, item)}
                  // Every Job, not the picked repository's: a member dispatched
                  // by this one is still its member while the rail is filtered.
                  board={state.jobs}
                  {...(draft === undefined ? {} : { draft })}
                  // The proposer call this window has out, and the one act on it.
                  // **Read by the lead's wait region on a Job at `proposing`** —
                  // nothing on the wire links a call to a Job, so this is the
                  // window's own and `detail-props.ts` says what that costs.
                  proposing={state.proposing}
                  onStopProposer={() => void commands.stopProposal()}
                  onReadDiff={readDiff}
                  onOpenArtifact={openArtifact}
                  onOpenPullRequest={openPullRequest}
                  // The replacement opens over the board, the way a Studio's Job
                  // node does — the same state, so Escape still returns here.
                  onOpenJob={setOpenJob}
                  onOpenMainLog={(at) => {
                    const owner = repositories.find((one) => one.manifest?.id === reading.owner_manifest_id) ?? repositories[0];
                    if (owner !== undefined) setMainLog({ root: owner.root, ...at });
                  }}
                  onOpenStudio={openStudioFrom}
                  onReadCheckOutput={readCheckOutput} checks={asked.host}
                  onReadBrief={readBrief}
                  onReadRetro={readRetro}
                  onAgreeLesson={agreeLesson}
                  onDisagreeLesson={disagreeLesson}
                  onReadFrame={readFrame}
                  onFrameSrc={frameSrc}
                  onNeedMaterial={readEvidence}
                  onNeedRemarks={readRemarks}
                  onNeedPulse={watchPulse}
                  watched={state.watched}
                  workflows={state.holds.workflows}
                  manifests={state.holds.manifests}
                  // Every question waiting on a person. A Job that dispatched a
                  // wave answers its Jobs' where they are read — the Board's own
                  // rows, above, say which Jobs those are. #1544.
                  questions={state.questions}
                  repositories={repositories}
                  stale={!live}
                  now={now}
                  acting={commands.acting === reading.id}
                  actingAct={commands.acting === reading.id ? (commands.actingAct ?? undefined) : undefined}
                  rerunningChecks={commands.rerunningChecks === reading.id}
                  approving={state.approving.includes(reading.id)}
                  deciding={commands.deciding === reading.id}
                  decidingAct={
                    commands.deciding === reading.id ? (commands.decidingAct ?? undefined) : undefined
                  }
                  answered={commands.answeredOn(reading.id)}
                  observed={state.observed}
                  journalled={state.journalled}
                  followed={state.followed}
                  onFollowCheckOutput={followCheckOutput}
                  resources={state.resources}
                  jobDrones={state.jobDrones}
                  history={state.history}
                  examination={state.examination}
                  // The one act here that changes nothing. It costs no model
                  // call, and its answer arrives on the published state rather
                  // than coming back — so a window reloaded mid-look still draws
                  // what Fleet found.
                  onExamine={examine}
                  // Held on the row, then confirmed here: the owner asked for both.
                  onKillProcess={(jobId, { pid, command }) =>
                    setConfirming({ act: "kill_process", jobId, pid, command })
                  }
                  onKillProcesses={(jobId, count) => setConfirming({ act: "kill_processes", jobId, count })}
                  // A plan task's own acts, straight through: each is ahead of
                  // its route, so the answer is Not implemented naming the issue.
                  onTaskAct={(act, jobId, taskId, edit) => commands.taskAct(act, jobId, taskId, edit)}
                  recorded={{
                    footprint: state.footprint,
                    handed: state.handed,
                    evidence: state.evidence,
                    diff: state.diff,
                    remarks: state.remarks,
                  }}
                  onAct={(what, jobId, droneId) => setConfirming(aJobAct(what, jobId, droneId))}
                  // Held on the header, so already confirmed: it sends what the dialog's own confirm sends.
                  onActHeld={(what, jobId, droneId) => void commands.act(what, jobId, undefined, droneId)}
                  onRedirect={(jobId, said, droneId) => void commands.redirect(jobId, said, droneId)}
                  onAnswer={(jobId, questionId, chose) =>
                    void commands.answer(jobId, questionId, chose)
                  }
                  onAnswerCommand={(...answer) => void commands.answerCommand(...answer)}
                  // A read beside the act it informs. It moves nothing, so it
                  // goes straight through rather than under `acting`.
                  onExplainCommand={explainCommand}
                  onAnswerJudge={(jobId, askedAt, answer, note) =>
                    void commands.answerJudge(jobId, askedAt, answer, note)
                  }
                  onSetWhenBlocked={(jobId, whenBlocked) =>
                    void commands.setWhenBlocked(jobId, whenBlocked)
                  }
                  onSetWhenRefused={(jobId, whenRefused) =>
                    void commands.setWhenRefused(jobId, whenRefused)
                  }
                  onSetModel={(jobId, model) => void commands.setModel(jobId, model)}
                  onSetReviewModel={(jobId, model) => void commands.setReviewModel(jobId, model)}
                  onRemoveAllowedCommand={(jobId, run) => void commands.removeAllowedCommand(jobId, run)}
                  models={state.holds.models}
                  machineCap={state.limits?.concurrency ?? null}
                  onOverrule={(jobId, reason) => void commands.overrule(jobId, reason)}
                  // The card's Send it back: the restart act, with the note typed there.
                  onSendBack={(jobId, note) => void commands.act("restart_step", jobId, note)}
                  onRaiseCap={(jobId, micros) => void commands.raiseCap(jobId, micros)}
                  onRaiseTurnCap={(jobId, turns) => void commands.raiseTurns(jobId, turns)}
                  onRerun={(jobId) => void commands.rerun(jobId)}
                  onRerunChecks={(jobId) => void commands.rerunChecks(jobId)}
                  onReport={commands.report}
                  onAddTask={commands.addTask}
                  onDropTask={commands.dropTask}
                  onMovePlan={commands.movePlan}
                  onEditJob={commands.editJob} onSetLandingTarget={commands.setLandingTarget} onToProposer={commands.toProposer}
                  onShowAgain={showAgain} onChooseTriggerFix={commands.chooseTriggerFix} onReadRepairDiff={commands.readRepairDiff} {...alerting}
                  onHoldAct={(jobId, act, by) => (act === "rerun" ? commands.rerunTrigger(jobId, by) : commands.skipTrigger(jobId, by))}
                  onApprove={commands.approve} onListBranches={commands.listBranches}
                  {...commands.mergeProps(reading.id)}
                  onRerunFailedChecks={(jobId) => void commands.rerunFailedChecks(jobId)}
                  onInvestigateFailedChecks={(jobId) => void commands.investigateFailedChecks(jobId)}
                  onQueueAfterFinding={(jobId, finding) => void commands.queueAfterFinding(jobId, finding)}
                  onFileFindingIssue={(jobId, finding, title, body) => void commands.fileFindingIssue(jobId, finding, title, body)}
                  onApproveReview={(jobId) => void commands.decide(jobId, "approve")}
                  onApproveWave={(jobId, jobs) => void commands.approveWave(jobId, { jobs })}
                  onRequestChanges={(jobId, note, walk) => void commands.decide(jobId, "changes", note, walk)}
                  onRemoveWalkNote={(jobId, noteId) => void window.armada.removeWalkNote(jobId, noteId)}
                  onReject={(jobId) => void commands.decide(jobId, "reject")}
                  onTakeUpRemarks={(jobId, remarks) => void commands.takeUpRemarks(jobId, remarks)}
                  onDismissFinding={(jobId, finding, reason) => void commands.dismissFinding(jobId, finding, reason)}
                  onOpenRemarkLink={(jobId, remarkId) => void openRemarkLink(jobId, remarkId)}
                  onOpenFindingIssue={(jobId, finding) => void openFindingIssue(jobId, finding)}
                  onCopied={setCopied}
                  onSaid={setTelling}
                  whereOpen={whereOpen}
                  onOpenWhere={pressWhereOpen}
                  planView={planView}
                  onPlanView={pressPlanView}
                  // `n` — the same composer every contextual surface opens.
                  onCompose={() => setComposing(true)}
                  // The run sheet — Journey 9 — and the servers it starts.
                  rehearsal={{
                    runSheet: state.runSheet,
                    runFollowed: state.runFollowed,
                    servers: state.servers,
                    onWatchRunSheet: watchRunSheet,
                    onObserveRun: observeRun,
                    onStartRun: startRun,
                    onStopRun: stopRun,
                    onUndoRun: undoRun,
                    onListRuns: listRuns,
                    onGetRunOutput: getRunOutput,
                    ...runSheetServers,
                  }}
                />
                {mainLog === null ? null : (
                  <LandCheckLogSheet
                    at={mainLog}
                    followed={state.landFollowed}
                    onFollow={followLandCheck}
                    floor={floor}
                    onClose={() => setMainLog(null)}
                  />
                )}
              </Boundary>
            ) : auditing ? (
              /* Read across every Job rather than through one. The rate is the
                 point, and a listing reached from a Job would show only the
                 reports somebody already had reason to open. */
              <Boundary region="the filed reports" {...guarded}>
                <Reports
                  reports={state.reports}
                  onWant={readReports}
                  onClose={() => setAuditing(false)}
                  onCopied={setCopied}
                />
              </Boundary>
            ) : lining ? (<MergeLineSurface state={state} {...guarded} onOpenLink={openProseLink} onOpenJob={setOpenJob} onFix={(fix) => void commands.fixMain(fix)} {...(asked.mergeFocus === undefined ? {} : { focus: asked.mergeFocus })} />) : workflowing ? (<WorkflowCreatorSurface state={state} {...guarded} />) : checking ? (<ChecksSurface state={state} onOpenJob={(jobId, to) => { asked.setOpening(to === undefined ? null : { jobId, to }); setOpenJob(jobId); }} onOpenMergeLine={(branch) => { goTo(SURFACE.mergeLine); asked.setMergeFocus(branch); }} {...guarded} />) : sessioning ? (<SessionsSurface openId={sessionOpen} onOpen={openSession} goes={{ onOpenJob: setOpenJob, onGoTo: goTo, onOpenLink: openProseLink }} held={{ held: state.held, onWant: readHeld }} />) : learning ? (
              <LessonsSurface repository={state.repository} onOpenJob={setOpenJob} {...guarded} />
            ) : clearing ? (
              /* What Fleet is holding disk for, read across every Job at once.
                 The half of the reclaim rule that is a person's: Fleet has
                 already taken back everything it could prove nobody needs, and
                 this is where the rest is given back, one tile at a time. */
              <Boundary region="Cleanup" {...guarded}>
                <Worktrees
                  held={state.held}
                  // Read for the handle that names a worktree outside the
                  // pool, and a `depended_on` reason's blocker.
                  jobs={state.jobs}
                  onWant={readHeld}
                  // The app's one `now`, because two clocks in one window drift.
                  now={now}
                  onClose={() => setClearing(false)}
                  onCopied={setCopied}
                  // A slot's Job opens over Cleanup, and Escape comes back here.
                  onOpenJob={setOpenJob}
                  {...slotActs}
                  actions={
                    <SweepButtons jobs={boardJobs} live={live} sweeping={commands.sweeping} onAsk={setSweep} />
                  }
                />
              </Boundary>
            ) : manifesting && all ? (
              <AskRepository
                repositories={repositories}
                title="Pick a repository to open its Manifest"
                next="The Board lists every repository's Jobs. A Manifest belongs to one, and picking it focuses the Board there."
                onPick={pick}
              />
            ) : manifesting ? (
              /* Everything this repository's Manifest declares, and one press
                 that runs one of them in the checkout as it is on disk. No Job
                 exists and none is created: the whole point of the surface is
                 that a person can run this project's lint without one. */
              <Boundary region="the manifest" {...guarded}>
                <Manifest
                  // Another repository is another page: its runs, its allows and a dismissed Verify are not this one's.
                  key={state.repository ?? ""}
                  sheet={state.checkoutRunSheet}
                  followed={state.checkoutRunFollowed}
                  picked={picked}
                  now={now}
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
                  // The one call this surface shares with a Job's own sheet:
                  // `start_server` has taken an optional Job since it landed,
                  // and no Job means the main checkout.
                  onStartServer={(name) => startServer(name)}
                  onStopServer={stopServer}
                  onOpenServerLink={openServerLink}
                  settingUp={settingUp}
                  onSettingUp={setSettingUp}
                  setUp={repository === undefined || scoped !== undefined}
                  setup={
                    <Setup
                      setting={setting}
                      now={now}
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
              </Boundary>
            ) : composing ? (
              <Composing
                state={state}
                commands={commands}
                live={live}
                all={all}
                repositories={repositories}
                scoped={scoped}
                onPick={pick}
                onClose={() => setComposing(false)}
                onSaid={setTelling}
                onCopied={setCopied}
                sketch={composedFrom}
                seed={seed}
              />
            ) : studying ? (
              <StudiosSurface
                state={state}
                live={live}
                repositories={repositories}
                manifestId={scoped?.id}
                all={all}
                onPick={pick}
                // Nothing set up anywhere: the Manifest surface is where a
                // repository is picked, and `pick` opens Setup on one with no
                // Manifest — so this hands over to that route rather than
                // cutting a second one from here.
                onSetUp={() => goTo(SURFACE.manifest)}
                open={openStudio}
                onOpenChange={setOpenStudio}
                selectedNode={studioNode}
                onSelectNode={setStudioNode}
                // A Job node opens its Job over the Studio, the way a Board row
                // opens one over the list — and Escape comes back here, because
                // `close` clears the Job and leaves the surface alone.
                onOpenJob={setOpenJob}
                onDispatchSketch={(from, drawn) => (setComposedFrom({ said: "", produced_by: from, drawn }), setComposing(true))}
                // A server node reads the live holder and counts its uptime on
                // the clock the rest of the app already ticks on — #1345.
                now={now}
                onCopied={setCopied}
              />
            ) : kitting ? (
              /* What a person already has, and then Kit's servers with both
                 tiers and what a Drone dispatched here resolves. #1275, #1491.

                 On All repositories the ask takes the servers' place and the
                 reading stays: what somebody already has is this machine's and
                 answers for every repository, so it needs no pick. */
              <Boundary region="Kit" {...guarded}>
                <Kit
                  // Another repository is another second tier. The machine-wide
                  // half is the same; what it resolves to is not.
                  key={state.repository ?? ""}
                  repository={
                    all || pickedRepository === null
                      ? null
                      : repositoryLabel(pickedRepository, repositories)
                  }
                  ask={
                    <AskRepository
                      repositories={repositories}
                      title="Pick a repository to see what its Drones are handed"
                      next="Kit is this machine's, and the same set everywhere. Which servers a Drone gets is a repository's own word over it, so Kit is read against one."
                      onPick={pick}
                    />
                  }
                  onReadKitInventory={readKitInventory}
                  onRemoveKitAllowedCommand={removeKitAllowedCommand}
                  onListKitServers={listKitServers}
                  onAddKitServer={addKitServer}
                  onForgetKitServer={forgetKitServer}
                  onSetKitServerReach={setKitServerReach}
                  onSetManifestServerReach={setManifestServerReach}
                />
              </Boundary>
            ) : guiding ? (
              /* The list of guides and the one open beside it — the catalogue
                 half of #1602, rearranged. It draws off the guide table alone,
                 so it needs nothing from Fleet and works disconnected.
                 No pane: the two columns are the screen and each scrolls
                 itself, which is what lets the folded sheet be flush to it.
                 The list's width is the window's to remember, the same way the
                 shell's own left column is — the note of 25 Sep 2026. */
              <Boundary region="Guides" {...guarded}>
                <GuideCatalogue narrow={narrow} floor={floor} listWidth={guideList} onResizeList={resizeGuideList} />
              </Boundary>
            ) : modding ? (<Boundary region="Mods" {...guarded}><ModsSurface /></Boundary>) : settingsShowing ? (
              <Boundary region="Settings" {...guarded}>
                <BridgeSettings
                  limits={state.limits}
                  live={live}
                  health={state.health}
                  onSave={commands.saveLimits}
                  preferences={state.preferences} onSavePreference={(save) => window.armada.savePreference(save)}
                  onReadGuides={() => goTo(SURFACE.guides)}
                />
              </Boundary>
            ) : (
              <>
                <Overview
                  state={state}
                  now={now}
                  live={live}
                  repositories={repositories}
                  disconnected={live ? null : statement.headline}
                  selected={openJob}
                  onOpen={setOpenJob}
                  onKill={(jobId) => setConfirming({ act: "kill_job", jobId })}
                  // Recently ended's own two, reusing the same confirmation
                  // `JobDetail`'s header already goes through for both acts —
                  // `reclaim_worktree` is that header's own word for Clear.
                  onRedispatch={(jobId) => setConfirming({ act: "redispatch", jobId })}
                  onClear={(jobId) => setConfirming({ act: "reclaim_worktree", jobId })}
                  onPausing={(act, jobId) => setConfirming({ act, jobId })}
                  onCompose={() => setComposing(true)}
                  onCopied={setCopied}
                  onCursor={setCursor}
                  land={landing}
                  onLanded={() => setLanding(null)}
                  onOpenLink={openProseLink} onFix={(fix) => void commands.fixMain(fix)}
                  onOpenSession={openSession}
                  nowViews={draft?.calls} nows={draft?.now} onTell={tell}
                  onQuickCompose={(words) => (setSeed(words), setComposing(true))}
                />

                {/* Never merged into the lists as a placeholder: a surface that
                    shows nine of ten Jobs and says so is honest, one that shows
                    nine is not. One bad row is not a broken board, and hiding it
                    is worse than drawing it broken. */}
                {state.unreadable.map((row) => (
                  <FailureBlock
                    key={row.job_id ?? row.fault}
                    failure={jobFailure(row, state.bridge)}
                    onCopied={setCopied}
                  />
                ))}
              </>
            )}
          </div>
        </Shell>

        {/* Studio capture — #1290. Last, so its layer paints over every surface
            and every overlay the shell draws under it. */}
        <CaptureLayer aim={captureAim} onCapture={captureStudioNote} />

        <ConfirmAct
          confirming={confirming}
          held={state.held}
          jobs={state.jobs}
          refused={commands.pauseSaid ?? undefined}
          onWant={readHeld}
          cleanupOpen={clearing}
          restartNote={restartNote}
          onRestartNote={setRestartNote}
          onCancel={() => {
            setConfirming(null);
            setRestartNote("");
          }}
          onConfirm={confirmed}
        />

        <SweepDialogs
          jobs={boardJobs}
          asking={sweep}
          sweeping={commands.sweeping}
          onDone={() => setSweep(null)}
          onClearTerminal={(jobIds) => void commands.clearTerminal(jobIds)}
          onForgetTerminal={(jobIds) => void commands.forgetTerminal(jobIds)}
        />

        <Locate locating={locate} />

        <PaletteMount
          open={palette.open}
          onClose={palette.onClose}
          reading={reading}
          shownStudio={shownStudio}
          on={onWhat}
          surfaces={SURFACES.filter((one) => !hidden.includes(one.id))}
          jobs={state.jobs}
          checkoutRunSheet={state.checkoutRunSheet}
          cursor={cursor}
          failing={failing}
          live={live}
          board={boardRows}
          acts={{
            openJob: setOpenJob,
            closeJob: close,
            compose: () => setComposing(true),
            surface: goTo,
            run: (entryId) => {
              goTo(SURFACE.manifest);
              setPicked(entryId);
            },
            copyDebugInfo: () => {
              if (failing !== null) copyDebugInfoFor(failing, setCopied);
            },
            confirm: (what, jobId) => setConfirming({ act: what, jobId }),
            openSetting: (id) => {
              if (id === "fleet_settings") goTo(SURFACE.settings);
            },
            refresh: () => void commands.refresh(),
            board: (id) => {
              if (id !== "reports") return setSweep(id === "clear" ? "clear" : "forget");
              // Reports close back to Overview, so that is where they open over.
              goTo(SURFACE.overview);
              setAuditing(true);
            },
          }}
          onConfirmAct={(id, jobId) => {
            if (id === "kill") setConfirming({ act: "kill_job", jobId });
          }}
        />

        <Toasts
          raised={raised}
          bridge={state.bridge}
          onLower={lower}
          copied={copied}
          said={telling}
          onCopied={setCopied}
        />
        </SessionsOwnership>
      </GuidanceProvider>
    </ProseLinks.Provider>
  );
}
