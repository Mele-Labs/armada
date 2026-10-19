import { contextBridge, ipcRenderer, webUtils } from "electron";

import { CHANNELS } from "../shared/bridge";
import { autopilotPreload } from "./autopilot";
import { sketchSharePreload } from "./sketch-share";
import type { AskLessonRead, AskTurn, LessonReviewRead } from "@armada/screens/src/lessons/review-wire";
import type { LessonSessionRead } from "@armada/screens/src/lessons/session-wire";
import { ANNOTATE_FLAG, ANNOTATION_CHANNELS } from "../shared/annotations";
import type { Annotation, AnnotationsDevApi, Box } from "../shared/annotations";
import type { PanelGo } from "@armada/studios/api";
import type { BridgeState, HistoryStep, Summons } from "../shared/bridge";
import type { CaptureAimed, CaptureAnswer, CaptureHeld, CaptureQueueKey, CaptureWheel, CaptureWindowState } from "../shared/capture-window";
import type { CaptureOpened, ClashSide } from "@armada/protocol";
import type { ChangeSlotPool, RescueSlot, SketchToKeep } from "@armada/protocol";
import type { BridgeApi } from "../shared/api";
import type { RescueOutcome } from "@armada/cleanup/api";
import type { Pattern } from "../shared/haptics";
import type { StateChange } from "../shared/state-change";
import type {
  CheckOutputRead,
  LessonAnswer,
  LessonsRead,
  RetroRead,
  RetroSubject,
  FrameRead,
  Outcome,
} from "@armada/protocol";
import type { ModChecked } from "@armada/protocol";
import type { ForgetReviewSearch, OpenReviewWalk, PullReviewLists, PullThreadResolved, RememberedPlace, RememberedPlaces, ResolvePullThread, ReviewRun, ReviewRuns, ReviewTools, ReviewWalk, SaveReviewTool, SaveWalkPlace, StartReviewRun, StartReviewSearch, WalkPlaceFound, WritePullRequest } from "@armada/protocol";
import type { ReviewWalkNote, ShownReviewWalk, ShownReviewWalkOutcome } from "../shared/review-walk";
import type { ChosenHold, SessionHeld, SessionHoldsReleased } from "@armada/protocol";
import type { ModelDownshiftState, PlanUsage, SignedOut, SleepState, TimelineAsk, TimelineEvent } from "@armada/protocol";
import type { FlightControls } from "@armada/protocol";
import type { DropRunFact, ImportFolders, ImportRuns, ModInstalled, RunMemory, RunMemoryPlace, SetRunMemoryPlace } from "@armada/protocol";
import type { AnswerSessionAsk, AnswerWaiting, ApprovePlan, ChangeWaiting, CheckoutCaughtUp, CheckoutDiff, CheckoutSentAsPullRequest, ClaimPullRequest, DismissConcern, DismissWaiting, JudgeSession, PullRequestClaimed, PullRequestFilePatch, PullRequestPage, PullRequestState, RenameSession, SendConcern, SendPlanChanges, NoteSent, SendSessionMessage, SendSessionNote, SessionRunMessage, SessionRunUnderway, SessionLoadout, SessionSubagent, SessionWait, StartAfter, TuneSession, WaitReading } from "@armada/protocol";
import type { ArtifactRead, LoadoutItemAsk, LoadoutItemBody, PageBounds } from "@armada/screens/src/draft/sessions";
import type { PullRequestPress, PullRequestWrite, SessionActed } from "../shared/api/sessions";
import type { RequestRepositoryRead } from "../shared/api/dispatch";
import type { SleepActed } from "../shared/api/sleep";
import type { AccountsActed } from "../shared/api/accounts";
import type { PlanUsageRead } from "../shared/api/plan-usage";
import type { ModelDownshiftRead } from "../shared/api/model-downshift";
import type { FleetTimelineRead } from "../shared/api/timeline";
import type { BayHistoryRead } from "../shared/api/bay-history";
import type { UsageRulesRead, UsageSummaryRead } from "../shared/api/usage-summary";
import type { FlightControlsAnswer, SessionShellAnswer } from "../shared/api/flight-controls";
import type { OnboardingAnswer } from "../shared/api/onboarding";
import type { ClashFileAnswer, WriteClashesRead } from "../shared/api/clashes";
import type { ReviewActed } from "../shared/api/reviews";
import type { RepositoryAreasRead, SlotTouchesRead } from "../shared/api/territory";
import type { IssueNumberRead, IssuesRead, PullRequestNumberRead } from "../shared/api/issues";
import type { HelmCarry } from "../shared/api/helm";
import type { HelmContext, HelmDebugRead, ThoughtRead, ThoughtTarget } from "@armada/protocol";
import type { StudioCapture, StudioNodeByHand, StudioPosition, StudioPromotion } from "@armada/protocol";
import type { StudioAnswer } from "@armada/screens/src/studio-reads";
import type { BuildSource, Followed, FleetRestart, LandCheckAt } from "@armada/protocol";
import type { RunOutputRead } from "@armada/protocol";
import type { CheckoutRunListRead, StartCheckoutRun } from "@armada/protocol";
import type { EditManifest, SaveManifestFile } from "@armada/protocol";
import type {
  ManifestEditAnswer,
  ManifestFileRead,
  ManifestSaveAnswer,
} from "@armada/screens/src/editing";
import type { CheckoutRunDiffRead } from "@armada/protocol";
import type { RepositoryAllowedCommandsRead } from "@armada/screens/src/manifest-allows";
import type { KitAllowedCommandsRead, KitInventoryRead, KitServersRead } from "@armada/screens/src/manifest-kit";
import type {
  ReadingTrigger,
  RemovingTrigger,
  SavingTrigger,
  TriggerDefinitionRead,
  TriggerRemoveAnswer,
  TriggerSaveAnswer,
  TriggersRead,
} from "../shared/triggers";
import type { SavingWorkflow, WorkflowDefinitionRead, WorkflowSaveAnswer, WorkflowsRead } from "../shared/workflows";
import type { AddKitServer, ManifestReach, ReachesDrones } from "@armada/protocol";
import type { PhoneAnswer, PhoneRequest, SettingsFileOpened } from "@armada/settings/api";
import type { LocateAnswer } from "@armada/screens/src/locate-reads";
import type { EditManifestProposal, WriteManifestProposal } from "@armada/protocol";
import type {
  ManifestProposalsRead,
  ProposalAnswer,
  RepositoryScanRead,
} from "@armada/screens/src/setup-reads";
import type {
  HelmCallAnswer,
  SavePreference,
  SaveSettings,
} from "@armada/protocol";

// The whole surface the renderer is allowed to see.
//
// **A preload surface is not the API the UI is meant to use, it is the API the
// UI is physically capable of using.** So every entry is one operation with a
// typed return — no raw `ipcRenderer`, no `require`, no filesystem handle, and
// no arbitrary-channel `invoke`, which would hand the renderer the whole main
// process under a thin name.
//
// The two kills are two entries on purpose. One capability taking "which kill"
// as an argument would be a surface that reads as one act and performs two.
const api: BridgeApi = {
  state: (): Promise<BridgeState> => ipcRenderer.invoke(CHANNELS.state),

  subscribeChanges: (onChange: (change: StateChange<BridgeState>) => void): (() => void) => {
    const handler = (_event: unknown, change: StateChange<BridgeState>): void => onChange(change);
    ipcRenderer.on(CHANNELS.changed, handler);
    return () => {
      ipcRenderer.removeListener(CHANNELS.changed, handler);
    };
  },
  syncState: (): Promise<StateChange<BridgeState>> => ipcRenderer.invoke(CHANNELS.syncState),
  onWalkFocus: (onFocus: (focused: boolean) => void): (() => void) => {
    const handler = (_event: unknown, focused: unknown): void => onFocus(focused === true);
    ipcRenderer.on(CHANNELS.walkFocused, handler);
    return () => {
      ipcRenderer.removeListener(CHANNELS.walkFocused, handler);
    };
  },

  // Bytes never round-trip through the request's own JSON channel as base64 —
  // this writes them to a staging file and hands back the path a message carries.
  stageAttachment: (bytes: ArrayBuffer, filename: string, mimeType: string): Promise<{ path: string }> =>
    ipcRenderer.invoke(CHANNELS.stageAttachment, bytes, filename, mimeType),

  searchFiles: (query: string, manifestId?: string): Promise<string[]> =>
    ipcRenderer.invoke(CHANNELS.searchFiles, query, manifestId),

  readRequestRepository: (words: string): Promise<RequestRepositoryRead> =>
    ipcRenderer.invoke(CHANNELS.readRequestRepository, words),

  changeSlotPool: (manifestId: string, change: ChangeSlotPool): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.changeSlotPool, manifestId, change),
  rescueSlot: (manifestId: string, rescue: RescueSlot): Promise<RescueOutcome> =>
    ipcRenderer.invoke(CHANNELS.rescueSlot, manifestId, rescue),

  // One held helm call. The note is a person's own words, read only on a
  // refusal; `undefined` crosses as `undefined`.
  answerHelmCall: (call: string, answer: HelmCallAnswer, note?: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.answerHelmCall, call, answer, note),
  // A person's Bridge preferences. **Fleet-wide, and no Job id crosses this
  // channel** — the only act on this surface that names none.
  savePreference: (save: SavePreference): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.savePreference, save),

  // settings.json, by key; `null` removes one. **Fleet-wide**, `savePreference`'s reason.
  saveSettings: (changes: SaveSettings["changes"]): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.saveSettings, changes),
  // The file in the configured editor. **No path crosses**: main opens the one Fleet named.
  openSettingsFile: (): Promise<SettingsFileOpened> => ipcRenderer.invoke(CHANNELS.openSettingsFile),

  // The Phone Gateway, on loopback, one named operation at a time. Main makes the call.
  phone: (request: PhoneRequest): Promise<PhoneAnswer> => ipcRenderer.invoke(CHANNELS.phone, request),

  // The mods on this machine. **Fleet-wide**, and the stylesheet comes from `validateMod` alone.
  validateMod: (name: string): Promise<ModChecked | null> =>
    ipcRenderer.invoke(CHANNELS.validateMod, name),
  setModEnabled: (name: string, enabled: boolean): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.setModEnabled, name, enabled),
  promoteMod: (name: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.promoteMod, name),

  // One running Check's log, as it is written. Read-only like the entry above,
  // and its own because it is its own socket.
  followCheckOutput: (jobId: string | null, kept: string | null): Promise<void> =>
    ipcRenderer.invoke(CHANNELS.followCheckOutput, jobId, kept),

  // One merge line Check's log, by the line's three names. Read-only like the entry above.
  followLandCheck: (at: LandCheckAt | null): Promise<void> =>
    ipcRenderer.invoke(CHANNELS.followLandCheck, at),

  // A rehearsal in the main checkout — the Manifest surface.
  watchCheckoutRunSheet: (want: boolean): Promise<void> =>
    ipcRenderer.invoke(CHANNELS.watchCheckoutRunSheet, want),

  observeCheckoutRun: (runId: string | null): Promise<void> =>
    ipcRenderer.invoke(CHANNELS.observeCheckoutRun, runId),

  // A name and nothing else.
  startCheckoutRun: (body: StartCheckoutRun): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.startCheckoutRun, body),

  stopCheckoutRun: (runId: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.stopCheckoutRun, runId),

  undoCheckoutRun: (runId: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.undoCheckoutRun, runId),

  listCheckoutRuns: (): Promise<CheckoutRunListRead> =>
    ipcRenderer.invoke(CHANNELS.listCheckoutRuns),

  getCheckoutRunOutput: (runId: string): Promise<RunOutputRead> =>
    ipcRenderer.invoke(CHANNELS.getCheckoutRunOutput, runId),

  // A read, and one run's only: main composes the route, and the id is
  // checked against the checkout's own runs by Fleet before any tree is read.
  getCheckoutRunDiff: (runId: string): Promise<CheckoutRunDiffRead> =>
    ipcRenderer.invoke(CHANNELS.getCheckoutRunDiff, runId),

  // Drift, held open by the Manifest surface; Verify, pressed on it.
  watchManifestDrift: (want: boolean): Promise<void> =>
    ipcRenderer.invoke(CHANNELS.watchManifestDrift, want),

  // Overview's two reads, held open by that surface. A boolean: main names every route.
  watchOverview: (want: boolean): Promise<void> => ipcRenderer.invoke(CHANNELS.watchOverview, want),

  startCheckoutVerify: (workspace?: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.startCheckoutVerify, workspace),

  // The Manifest file. **A write into the repository**, and still no path: the
  // renderer hands over text and what it started from, and Fleet decides
  // where it lands and whether the disk still matches.
  readManifestFile: (): Promise<ManifestFileRead> => ipcRenderer.invoke(CHANNELS.readManifestFile),

  saveManifestFile: (body: SaveManifestFile): Promise<ManifestSaveAnswer> =>
    ipcRenderer.invoke(CHANNELS.saveManifestFile, body),
  // Edits by key. Still no path: Fleet names the file.
  editManifest: (body: EditManifest): Promise<ManifestEditAnswer> =>
    ipcRenderer.invoke(CHANNELS.editManifest, body),
  // Setup: Scan, the proposals, one edit, Write — one operation each.
  readRepositoryScan: (): Promise<RepositoryScanRead> => ipcRenderer.invoke(CHANNELS.readRepositoryScan),
  readManifestProposals: (): Promise<ManifestProposalsRead> =>
    ipcRenderer.invoke(CHANNELS.readManifestProposals),
  editManifestProposal: (body: EditManifestProposal): Promise<ProposalAnswer> =>
    ipcRenderer.invoke(CHANNELS.editManifestProposal, body),
  writeManifestProposal: (body: WriteManifestProposal): Promise<ProposalAnswer> =>
    ipcRenderer.invoke(CHANNELS.writeManifestProposal, body),

  // A repository-wide always-allow — Fleet's own table since protocol 13.5.
  // Neither takes a path or a job id: Fleet names the repository.
  listRepositoryAllowedCommands: (): Promise<RepositoryAllowedCommandsRead> =>
    ipcRenderer.invoke(CHANNELS.listRepositoryAllowedCommands),

  removeRepositoryAllowedCommand: (run: string): Promise<RepositoryAllowedCommandsRead> =>
    ipcRenderer.invoke(CHANNELS.removeRepositoryAllowedCommand, run),

  readKitInventory: (): Promise<KitInventoryRead> => ipcRenderer.invoke(CHANNELS.readKitInventory),
  removeKitAllowedCommand: (run: string): Promise<KitAllowedCommandsRead> =>
    ipcRenderer.invoke(CHANNELS.removeKitAllowedCommand, run),
  listKitServers: (): Promise<KitServersRead> => ipcRenderer.invoke(CHANNELS.listKitServers),
  addKitServer: (adding: AddKitServer): Promise<KitServersRead> =>
    ipcRenderer.invoke(CHANNELS.addKitServer, adding),
  forgetKitServer: (name: string): Promise<KitServersRead> =>
    ipcRenderer.invoke(CHANNELS.forgetKitServer, name),
  setKitServerReach: (name: string, drones: ReachesDrones): Promise<KitServersRead> =>
    ipcRenderer.invoke(CHANNELS.setKitServerReach, name, drones),
  setManifestServerReach: (name: string, reach: ManifestReach | null): Promise<KitServersRead> =>
    ipcRenderer.invoke(CHANNELS.setManifestServerReach, name, reach),

  readWorkflows: (): Promise<WorkflowsRead> => ipcRenderer.invoke(CHANNELS.readWorkflows),
  readWorkflowDefinition: (workflowId: string, source: string): Promise<WorkflowDefinitionRead> =>
    ipcRenderer.invoke(CHANNELS.readWorkflowDefinition, workflowId, source),
  saveWorkflow: (saving: SavingWorkflow): Promise<WorkflowSaveAnswer> =>
    ipcRenderer.invoke(CHANNELS.saveWorkflow, saving),

  readTriggers: (): Promise<TriggersRead> => ipcRenderer.invoke(CHANNELS.readTriggers),
  readTrigger: (reading: ReadingTrigger): Promise<TriggerDefinitionRead> =>
    ipcRenderer.invoke(CHANNELS.readTrigger, reading),
  saveTrigger: (saving: SavingTrigger): Promise<TriggerSaveAnswer> =>
    ipcRenderer.invoke(CHANNELS.saveTrigger, saving),
  removeTrigger: (removing: RemovingTrigger): Promise<TriggerRemoveAnswer> =>
    ipcRenderer.invoke(CHANNELS.removeTrigger, removing),
  // Start a declared server, for this Job's worktree or, with no Job, the
  // main checkout — the capability the Manifest surface shares, which is why
  // `jobId` was optional here before that surface existed.
  pickRepository: (root: string | null): Promise<void> => ipcRenderer.invoke(CHANNELS.pickRepository, root),

  // Locate: the OS folder dialog, a clone parent as Fleet will spell it, and a repository added or cloned.
  chooseFolder: (): Promise<string | null> => ipcRenderer.invoke(CHANNELS.chooseFolder),
  resolveFolder: (path: string): Promise<string | null> => ipcRenderer.invoke(CHANNELS.resolveFolder, path),
  addRepository: (path: string): Promise<LocateAnswer> => ipcRenderer.invoke(CHANNELS.addRepository, path),
  cloneRepository: (url: string, parent: string): Promise<LocateAnswer> => ipcRenderer.invoke(CHANNELS.cloneRepository, url, parent),
  createProject: (parent: string, name: string): Promise<LocateAnswer> => ipcRenderer.invoke(CHANNELS.createProject, parent, name),

  startServer: (name: string, jobId?: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.startServer, name, jobId),

  stopServer: (serverId: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.stopServer, serverId),

  // The third entry that reaches outside the app. A server id and the link's
  // own address, `openRemarkLink`'s reason: main checks the address against
  // what it is already holding for that server before it opens anything, so
  // no string a click handler composed reaches `shell.openExternal` unchecked.
  openServerLink: (serverId: string, url: string): Promise<Followed> =>
    ipcRenderer.invoke(CHANNELS.openServerLink, serverId, url),

  // A link in a model's text. The address crosses because nothing else could
  // name it; main opens `http(s):` only. `main/links.ts`.
  openLink: (address: string): Promise<Followed> => ipcRenderer.invoke(CHANNELS.openLink, address),
  restartFleet: (): Promise<FleetRestart> => ipcRenderer.invoke(CHANNELS.restartFleet),
  // Restart Fleet and Bridge onto main or the preview. Fleet starts the script and answers at once;
  // the outcome arrives as `fleetBuild` on the published state.
  changeFleetBuild: (build: BuildSource, adopt: boolean): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.changeFleetBuild, build, adopt),

  readSessionCheckOutput: (run: number): Promise<CheckOutputRead> =>
    ipcRenderer.invoke(CHANNELS.readSessionCheckOutput, run),
  stopSessionCheck: (run: number, force: boolean): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.stopSessionCheck, run, force),
  // A Job's retro and the Lessons listing, read when a surface opens and on focus.
  readRetro: (subject: RetroSubject): Promise<RetroRead> =>
    ipcRenderer.invoke(CHANNELS.readRetro, subject),
  readLessons: (state: "open" | "accepted"): Promise<LessonsRead> =>
    ipcRenderer.invoke(CHANNELS.readLessons, state),
  // The owner's answer to one retro item, by its id. Each is one operation, not a channel.
  agreeLesson: (lessonId: string): Promise<LessonAnswer> => ipcRenderer.invoke(CHANNELS.agreeLesson, lessonId),
  disagreeLesson: (lessonId: string): Promise<LessonAnswer> =>
    ipcRenderer.invoke(CHANNELS.disagreeLesson, lessonId),
  // The guided review: the model's queue over the open items, and one question about one item.
  // Neither names a manifest or a port: main narrows the first to this window's pick.
  startLessonSession: (lessonId: string): Promise<LessonSessionRead> =>
    ipcRenderer.invoke(CHANNELS.startLessonSession, lessonId),
  reviewLessons: (): Promise<LessonReviewRead> => ipcRenderer.invoke(CHANNELS.reviewLessons),
  askLesson: (lessonId: string, question: string, history: AskTurn[]): Promise<AskLessonRead> =>
    ipcRenderer.invoke(CHANNELS.askLesson, lessonId, question, history),

  // Sessions Fleet hosts. **Each names a session by its id and nothing else of Fleet's**: the
  // repository a new one starts in is the window's own pick, read in main, and a pull request is
  // named by its number against the session that holds it. No path and no port crosses.
  getSleep: (): Promise<SleepActed> => ipcRenderer.invoke(CHANNELS.getSleep),
  // A repository's open issues, by its Manifest (#91), and the Autopilot that works them (#2092).
  listIssues: (manifestId: string, refresh: boolean): Promise<IssuesRead> => ipcRenderer.invoke(CHANNELS.listIssues, manifestId, refresh),
  getIssue: (manifestId: string, reference: string): Promise<IssueNumberRead> => ipcRenderer.invoke(CHANNELS.getIssue, manifestId, reference),
  getPullRequest: (manifestId: string, number: number): Promise<PullRequestNumberRead> =>
    ipcRenderer.invoke(CHANNELS.getPullRequest, manifestId, number),
  ...autopilotPreload,
  ...sketchSharePreload,
  getPlanUsage: (): Promise<PlanUsageRead> => ipcRenderer.invoke(CHANNELS.getPlanUsage),
  onPlanUsageChanged: (onChanged: (usage: PlanUsage) => void): (() => void) => {
    const handler = (_event: unknown, usage: PlanUsage): void => onChanged(usage);
    ipcRenderer.on(CHANNELS.planUsageChanged, handler);
    return () => {
      ipcRenderer.removeListener(CHANNELS.planUsageChanged, handler);
    };
  },
  getModelDownshift: (): Promise<ModelDownshiftRead> => ipcRenderer.invoke(CHANNELS.getModelDownshift),
  acceptDownshift: (): Promise<ModelDownshiftRead> => ipcRenderer.invoke(CHANNELS.acceptDownshift),
  restoreDownshift: (): Promise<ModelDownshiftRead> => ipcRenderer.invoke(CHANNELS.restoreDownshift),
  keepDownshift: (): Promise<ModelDownshiftRead> => ipcRenderer.invoke(CHANNELS.keepDownshift),
  settleDownshift: (): Promise<ModelDownshiftRead> => ipcRenderer.invoke(CHANNELS.settleDownshift),
  onModelDownshiftChanged: (onChanged: (state: ModelDownshiftState) => void): (() => void) => {
    const handler = (_event: unknown, state: ModelDownshiftState): void => onChanged(state);
    ipcRenderer.on(CHANNELS.modelDownshiftChanged, handler);
    return () => {
      ipcRenderer.removeListener(CHANNELS.modelDownshiftChanged, handler);
    };
  },
  getFleetTimeline: (window?: TimelineAsk): Promise<FleetTimelineRead> => ipcRenderer.invoke(CHANNELS.getFleetTimeline, window),
  getSlotHistory: (manifestId: string, slot: number): Promise<BayHistoryRead> => ipcRenderer.invoke(CHANNELS.getSlotHistory, manifestId, slot),
  getUsageSummary: (tzOffsetMinutes: number): Promise<UsageSummaryRead> => ipcRenderer.invoke(CHANNELS.getUsageSummary, tzOffsetMinutes),
  getUsageRules: (): Promise<UsageRulesRead> => ipcRenderer.invoke(CHANNELS.getUsageRules),
  getFlightControls: (): Promise<FlightControlsAnswer> => ipcRenderer.invoke(CHANNELS.getFlightControls),
  saveFlightControls: (controls: FlightControls): Promise<FlightControlsAnswer> => ipcRenderer.invoke(CHANNELS.saveFlightControls, controls),
  runSessionShell: (sessionId: string, command: string): Promise<SessionShellAnswer> => ipcRenderer.invoke(CHANNELS.runSessionShell, sessionId, command),
  listImportFolders: (): Promise<OnboardingAnswer<ImportFolders>> => ipcRenderer.invoke(CHANNELS.listImportFolders),
  getImportRuns: (path: string): Promise<OnboardingAnswer<ImportRuns>> => ipcRenderer.invoke(CHANNELS.getImportRuns, path),
  installAgentMod: (): Promise<OnboardingAnswer<ModInstalled>> => ipcRenderer.invoke(CHANNELS.installAgentMod),
  getRunMemory: (repository: string): Promise<OnboardingAnswer<RunMemory>> => ipcRenderer.invoke(CHANNELS.getRunMemory, repository),
  getRunMemoryPlace: (repository: string): Promise<OnboardingAnswer<RunMemoryPlace>> => ipcRenderer.invoke(CHANNELS.getRunMemoryPlace, repository),
  setRunMemoryPlace: (body: SetRunMemoryPlace): Promise<OnboardingAnswer<RunMemoryPlace>> => ipcRenderer.invoke(CHANNELS.setRunMemoryPlace, body),
  dropRunFact: (body: DropRunFact): Promise<OnboardingAnswer<RunMemory>> => ipcRenderer.invoke(CHANNELS.dropRunFact, body),
  onTimelineAppended: (onAppended: (event: TimelineEvent) => void): (() => void) => {
    const handler = (_event: unknown, event: TimelineEvent): void => onAppended(event);
    ipcRenderer.on(CHANNELS.timelineAppended, handler);
    return () => {
      ipcRenderer.removeListener(CHANNELS.timelineAppended, handler);
    };
  },
  getWriteClashes: (): Promise<WriteClashesRead> => ipcRenderer.invoke(CHANNELS.getWriteClashes),
  readClashFile: (sessionId: string, other: { kind: ClashSide; id: string }, path: string): Promise<ClashFileAnswer> =>
    ipcRenderer.invoke(CHANNELS.readClashFile, sessionId, other, path),
  getReviewLists: (): Promise<ReviewActed<PullReviewLists>> => ipcRenderer.invoke(CHANNELS.getReviewLists),
  readReviewPage: (manifestId: string, number: number): Promise<ReviewActed<PullRequestPage>> => ipcRenderer.invoke(CHANNELS.readReviewPage, manifestId, number),
  readReviewFile: (manifestId: string, number: number, path: string): Promise<ReviewActed<PullRequestFilePatch>> => ipcRenderer.invoke(CHANNELS.readReviewFile, manifestId, number, path),
  writeReview: (manifestId: string, write: WritePullRequest): Promise<ReviewActed<PullRequestPage>> => ipcRenderer.invoke(CHANNELS.writeReview, manifestId, write),
  resolveReviewThread: (manifestId: string, resolve: ResolvePullThread): Promise<ReviewActed<PullThreadResolved>> => ipcRenderer.invoke(CHANNELS.resolveReviewThread, manifestId, resolve),
  listReviewTools: (manifestId: string): Promise<ReviewActed<ReviewTools>> => ipcRenderer.invoke(CHANNELS.listReviewTools, manifestId),
  saveReviewTool: (manifestId: string, save: SaveReviewTool): Promise<ReviewActed<ReviewTools>> => ipcRenderer.invoke(CHANNELS.saveReviewTool, manifestId, save),
  startReviewRun: (manifestId: string, start: StartReviewRun): Promise<ReviewActed<ReviewRun>> => ipcRenderer.invoke(CHANNELS.startReviewRun, manifestId, start),
  getReviewRuns: (manifestId: string, number: number): Promise<ReviewActed<ReviewRuns>> => ipcRenderer.invoke(CHANNELS.getReviewRuns, manifestId, number),
  onReviewRun: (onRun: (run: ReviewRun) => void): (() => void) => {
    const handler = (_event: unknown, run: ReviewRun): void => onRun(run);
    ipcRenderer.on(CHANNELS.reviewRunChanged, handler);
    return () => {
      ipcRenderer.removeListener(CHANNELS.reviewRunChanged, handler);
    };
  },
  startReviewSearch: (start: StartReviewSearch): Promise<ReviewActed<ReviewRun>> => ipcRenderer.invoke(CHANNELS.startReviewSearch, start),
  getReviewSearches: (): Promise<ReviewActed<ReviewRuns>> => ipcRenderer.invoke(CHANNELS.getReviewSearches),
  findWalkPlace: (manifestId: string, number: number): Promise<ReviewActed<WalkPlaceFound>> => ipcRenderer.invoke(CHANNELS.findWalkPlace, manifestId, number),
  forgetReviewSearch: (forget: ForgetReviewSearch): Promise<ReviewActed<ReviewRuns>> => ipcRenderer.invoke(CHANNELS.forgetReviewSearch, forget),
  listWalkPlaces: (): Promise<ReviewActed<RememberedPlaces>> => ipcRenderer.invoke(CHANNELS.listWalkPlaces),
  saveWalkPlace: (manifestId: string, save: SaveWalkPlace): Promise<ReviewActed<RememberedPlace>> => ipcRenderer.invoke(CHANNELS.saveWalkPlace, manifestId, save),
  forgetWalkPlace: (manifestId: string, id: string): Promise<ReviewActed<RememberedPlaces>> => ipcRenderer.invoke(CHANNELS.forgetWalkPlace, manifestId, id),
  openReviewWalk: (manifestId: string, open: OpenReviewWalk): Promise<ReviewActed<ReviewWalk>> => ipcRenderer.invoke(CHANNELS.openReviewWalk, manifestId, open),
  closeReviewWalk: (manifestId: string, id: string): Promise<ReviewActed<ReviewWalk>> => ipcRenderer.invoke(CHANNELS.closeReviewWalk, manifestId, id),
  showReviewWalk: (shown: ShownReviewWalk): Promise<ShownReviewWalkOutcome> => ipcRenderer.invoke(CHANNELS.showReviewWalk, shown),
  onReviewWalk: (onWalk: (walk: ReviewWalk) => void): (() => void) => {
    const handler = (_event: unknown, walk: ReviewWalk): void => onWalk(walk);
    ipcRenderer.on(CHANNELS.reviewWalkChanged, handler);
    return () => {
      ipcRenderer.removeListener(CHANNELS.reviewWalkChanged, handler);
    };
  },
  onReviewWalkNote: (onNote: (note: ReviewWalkNote) => void): (() => void) => {
    const handler = (_event: unknown, note: ReviewWalkNote): void => onNote(note);
    ipcRenderer.on(CHANNELS.reviewWalkNote, handler);
    return () => {
      ipcRenderer.removeListener(CHANNELS.reviewWalkNote, handler);
    };
  },
  onReviewWalkClosed: (onClosed: (walkId: string) => void): (() => void) => {
    const handler = (_event: unknown, walkId: string): void => onClosed(walkId);
    ipcRenderer.on(CHANNELS.reviewWalkClosed, handler);
    return () => {
      ipcRenderer.removeListener(CHANNELS.reviewWalkClosed, handler);
    };
  },
  getRepositoryAreas: (): Promise<RepositoryAreasRead> => ipcRenderer.invoke(CHANNELS.getRepositoryAreas),
  getSlotTouches: (): Promise<SlotTouchesRead> => ipcRenderer.invoke(CHANNELS.getSlotTouches),
  getAccounts: (): Promise<AccountsActed> => ipcRenderer.invoke(CHANNELS.getAccounts),
  startSignIn: (): Promise<AccountsActed> => ipcRenderer.invoke(CHANNELS.startSignIn),
  signOut: (accountId: number): Promise<AccountsActed<SignedOut>> => ipcRenderer.invoke(CHANNELS.signOut, accountId),
  setRepositoryProfile: (root: string, accountId: number | null): Promise<AccountsActed> =>
    ipcRenderer.invoke(CHANNELS.setRepositoryProfile, root, accountId),
  setSleep: (on: boolean): Promise<SleepActed> => ipcRenderer.invoke(CHANNELS.setSleep, on),
  overrideSleep: (id: string, text: string): Promise<SleepActed> => ipcRenderer.invoke(CHANNELS.overrideSleep, id, text),
  reviewSleep: (id: string): Promise<SleepActed> => ipcRenderer.invoke(CHANNELS.reviewSleep, id),
  onSleepChanged: (onChanged: (state: SleepState) => void): (() => void) => {
    const handler = (_event: unknown, state: SleepState): void => onChanged(state);
    ipcRenderer.on(CHANNELS.sleepChanged, handler);
    return () => {
      ipcRenderer.removeListener(CHANNELS.sleepChanged, handler);
    };
  },
  startSession: (title?: string, root?: string, request?: string, tuning?: { model?: string; effort?: string }, after?: StartAfter): Promise<SessionActed> => ipcRenderer.invoke(CHANNELS.startSession, title, root, request, tuning, after),
  readSessionWait: (sessionId: string, request: string): Promise<SessionActed<WaitReading>> => ipcRenderer.invoke(CHANNELS.readSessionWait, sessionId, request),
  changeWaiting: (change: ChangeWaiting): Promise<SessionActed<SessionWait>> => ipcRenderer.invoke(CHANNELS.changeWaiting, change),
  startWaitingNow: (sessionId: string): Promise<SessionActed> => ipcRenderer.invoke(CHANNELS.startWaitingNow, sessionId),
  sendSessionMessage: (send: SendSessionMessage): Promise<SessionActed> =>
    ipcRenderer.invoke(CHANNELS.sendSessionMessage, send),
  sendSessionNote: (send: SendSessionNote): Promise<SessionActed<NoteSent>> => ipcRenderer.invoke(CHANNELS.sendSessionNote, send),
  answerSessionAsk: (answer: AnswerSessionAsk): Promise<SessionActed> =>
    ipcRenderer.invoke(CHANNELS.answerSessionAsk, answer),
  answerWaiting: (answer: AnswerWaiting): Promise<SessionActed> => ipcRenderer.invoke(CHANNELS.answerWaiting, answer),
  claimPullRequest: (claim: ClaimPullRequest): Promise<SessionActed<PullRequestClaimed>> => ipcRenderer.invoke(CHANNELS.claimPullRequest, claim),
  dismissWaiting: (dismiss: DismissWaiting): Promise<SessionActed> => ipcRenderer.invoke(CHANNELS.dismissWaiting, dismiss),
  approvePlan: (approve: ApprovePlan): Promise<SessionActed> => ipcRenderer.invoke(CHANNELS.approvePlan, approve),
  sendPlanChanges: (changes: SendPlanChanges): Promise<SessionActed> => ipcRenderer.invoke(CHANNELS.sendPlanChanges, changes),
  judgeSession: (judge: JudgeSession): Promise<SessionActed> => ipcRenderer.invoke(CHANNELS.judgeSession, judge),
  sendConcern: (send: SendConcern): Promise<SessionActed> => ipcRenderer.invoke(CHANNELS.sendConcern, send),
  dismissConcern: (dismiss: DismissConcern): Promise<SessionActed> => ipcRenderer.invoke(CHANNELS.dismissConcern, dismiss),
  tuneSession: (tune: TuneSession): Promise<SessionActed> => ipcRenderer.invoke(CHANNELS.tuneSession, tune),
  runSessionCommand: (sessionId: string, command: string): Promise<SessionActed> =>
    ipcRenderer.invoke(CHANNELS.runSessionCommand, sessionId, command),
  renameSession: (rename: RenameSession): Promise<SessionActed> => ipcRenderer.invoke(CHANNELS.renameSession, rename),
  forkSession: (sessionId: string): Promise<SessionActed> => ipcRenderer.invoke(CHANNELS.forkSession, sessionId),
  forkSessionAt: (sessionId: string, at: string): Promise<SessionActed> => ipcRenderer.invoke(CHANNELS.forkSessionAt, sessionId, at),
  closeSession: (sessionId: string): Promise<SessionActed> => ipcRenderer.invoke(CHANNELS.closeSession, sessionId),
  stopSession: (sessionId: string): Promise<SessionActed> => ipcRenderer.invoke(CHANNELS.stopSession, sessionId),
  retryHandoff: (sessionId: string): Promise<SessionActed> => ipcRenderer.invoke(CHANNELS.retryHandoff, sessionId),
  pauseSession: (sessionId: string): Promise<SessionActed> => ipcRenderer.invoke(CHANNELS.pauseSession, sessionId),
  resumeSession: (sessionId: string): Promise<SessionActed> => ipcRenderer.invoke(CHANNELS.resumeSession, sessionId),
  setSessionWorkflow: (sessionId: string, workflow?: string): Promise<SessionActed> => ipcRenderer.invoke(CHANNELS.setSessionWorkflow, sessionId, workflow),
  releaseSessionHolds: (sessionId: string, toSessionId?: string, rows?: ChosenHold[]): Promise<SessionActed<SessionHoldsReleased>> =>
    ipcRenderer.invoke(CHANNELS.releaseSessionHolds, sessionId, toSessionId, rows),
  handToSession: (sessionId: string, group: string): Promise<SessionActed<SessionHeld>> => ipcRenderer.invoke(CHANNELS.handToSession, sessionId, group),
  moveSession: (sessionId: string, manifestId: string): Promise<SessionActed> =>
    ipcRenderer.invoke(CHANNELS.moveSession, sessionId, manifestId),
  retroSession: (sessionId: string): Promise<Outcome> => ipcRenderer.invoke(CHANNELS.retroSession, sessionId),
  watchSession: (sessionId: string): Promise<void> => ipcRenderer.invoke(CHANNELS.watchSession, sessionId),
  readSessionFile: (sessionId: string, file: string): Promise<FrameRead> =>
    ipcRenderer.invoke(CHANNELS.readSessionFile, sessionId, file),
  readSessionSubagent: (sessionId: string, subagentId: string): Promise<SessionActed<SessionSubagent>> =>
    ipcRenderer.invoke(CHANNELS.readSessionSubagent, sessionId, subagentId),
  readSessionLoadout: (sessionId: string): Promise<SessionActed<SessionLoadout>> => ipcRenderer.invoke(CHANNELS.readSessionLoadout, sessionId),
  readLoadoutItem: (sessionId: string, item: LoadoutItemAsk): Promise<SessionActed<LoadoutItemBody>> => ipcRenderer.invoke(CHANNELS.readLoadoutItem, sessionId, item),
  openSessionFile: (sessionId: string, path: string): Promise<Followed> =>
    ipcRenderer.invoke(CHANNELS.openSessionFile, sessionId, path),
  openSessionWindow: (sessionId: string, url: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.openSessionWindow, sessionId, url),
  getSessionWindows: (): Promise<readonly string[]> => ipcRenderer.invoke(CHANNELS.getSessionWindows),
  onSessionWindows: (onChanged: (addresses: readonly string[]) => void): (() => void) => {
    const handler = (_event: unknown, addresses: unknown): void =>
      onChanged(Array.isArray(addresses) ? addresses.filter((one): one is string => typeof one === "string") : []);
    ipcRenderer.on(CHANNELS.sessionWindowsChanged, handler);
    return () => {
      ipcRenderer.removeListener(CHANNELS.sessionWindowsChanged, handler);
    };
  },
  readSessionArtifact: (sessionId: string, path: string): Promise<ArtifactRead> =>
    ipcRenderer.invoke(CHANNELS.readSessionArtifact, sessionId, path),
  showSessionPage: (sessionId: string, address: string, bounds: PageBounds): Promise<Followed> =>
    ipcRenderer.invoke(CHANNELS.showSessionPage, sessionId, address, bounds),
  moveSessionPage: (bounds: PageBounds): Promise<void> => ipcRenderer.invoke(CHANNELS.moveSessionPage, bounds),
  hideSessionPage: (): Promise<void> => ipcRenderer.invoke(CHANNELS.hideSessionPage),
  onSessionPageEscape: (on: () => void): (() => void) => {
    const handler = (): void => on();
    ipcRenderer.on(CHANNELS.sessionPageEscape, handler);
    return () => {
      ipcRenderer.removeListener(CHANNELS.sessionPageEscape, handler);
    };
  },
  pressPullRequest: (
    sessionId: string,
    number: number,
    press: PullRequestPress,
  ): Promise<SessionActed<PullRequestState>> =>
    ipcRenderer.invoke(CHANNELS.pressPullRequest, sessionId, number, press),
  // A pull request no Session holds, named by its repository's Manifest.
  pressRepositoryPullRequest: (
    manifestId: string,
    number: number,
    press: PullRequestPress,
  ): Promise<SessionActed<PullRequestState>> =>
    ipcRenderer.invoke(CHANNELS.pressRepositoryPullRequest, manifestId, number, press),
  readPullRequestPage: (sessionId: string, number: number): Promise<SessionActed<PullRequestPage>> =>
    ipcRenderer.invoke(CHANNELS.readPullRequestPage, sessionId, number),
  readPullRequestFile: (sessionId: string, number: number, path: string): Promise<SessionActed<PullRequestFilePatch>> =>
    ipcRenderer.invoke(CHANNELS.readPullRequestFile, sessionId, number, path),
  writePullRequest: (sessionId: string, number: number, write: PullRequestWrite): Promise<SessionActed<PullRequestPage>> =>
    ipcRenderer.invoke(CHANNELS.writePullRequest, sessionId, number, write),
  // A shell block run from a Session's thread: one call to run, one to stop, one subscription for what runs print.
  startSessionRun: (sessionId: string, command: string, cols?: number, rows?: number): Promise<SessionActed<SessionRunUnderway>> =>
    ipcRenderer.invoke(CHANNELS.startSessionRun, sessionId, command, cols, rows),
  // The one pull a held checkout is offered: the repository's Manifest and no command.
  catchUpCheckout: (manifestId: string): Promise<SessionActed<CheckoutCaughtUp>> =>
    ipcRenderer.invoke(CHANNELS.catchUpCheckout, manifestId),
  // A held checkout's uncommitted files as a pull request: the repository's Manifest and no path.
  sendCheckoutAsPullRequest: (manifestId: string): Promise<SessionActed<CheckoutSentAsPullRequest>> =>
    ipcRenderer.invoke(CHANNELS.sendCheckoutAsPullRequest, manifestId),
  // One uncommitted file of a held checkout as a diff: the Manifest and the path.
  readCheckoutDiff: (manifestId: string, path: string): Promise<SessionActed<CheckoutDiff>> =>
    ipcRenderer.invoke(CHANNELS.readCheckoutDiff, manifestId, path),
  stopSessionRun: (runId: string): Promise<SessionActed<SessionRunUnderway>> =>
    ipcRenderer.invoke(CHANNELS.stopSessionRun, runId),
  writeSessionRun: (runId: string, data: string): Promise<SessionActed<SessionRunUnderway>> =>
    ipcRenderer.invoke(CHANNELS.writeSessionRun, runId, data),
  resizeSessionRun: (runId: string, cols: number, rows: number): Promise<SessionActed<SessionRunUnderway>> =>
    ipcRenderer.invoke(CHANNELS.resizeSessionRun, runId, cols, rows),
  onSessionRunMessage: (on: (runId: string, message: SessionRunMessage) => void): (() => void) => {
    const handler = (_: unknown, runId: string, message: SessionRunMessage): void => on(runId, message);
    ipcRenderer.on(CHANNELS.sessionRunMessage, handler);
    return () => {
      ipcRenderer.removeListener(CHANNELS.sessionRunMessage, handler);
    };
  },

  // What fleet is holding disk for, and the test each one did not pass. The
  // second read here with no job id, and for a different reason: what is being
  // decided is which of a set to give back, which no per-job field could ask.
  // Read-only — `reclaimWorktree`, `deleteBranch` and `forgetJob` are the
  // three acts, one job at a time, and they already exist above. A piloted
  // job's checkout is not in the answer at all: fleet drops it, so nothing
  // here can offer a directory a person is standing in.
  readHeld: (want: boolean): Promise<void> =>
    ipcRenderer.invoke(CHANNELS.readHeld, want),

  // A repository's Studios, and one open — #1287. Two reads a surface holds, four acts, each one
  // operation with its own ids: nothing here names a route or reaches a node on another Studio.
  watchStudios: (manifestId: string | readonly string[] | null): Promise<void> =>
    ipcRenderer.invoke(CHANNELS.watchStudios, manifestId),
  watchStudio: (studioId: string | null): Promise<void> => ipcRenderer.invoke(CHANNELS.watchStudio, studioId),
  createStudio: (manifestId: string): Promise<StudioAnswer> =>
    ipcRenderer.invoke(CHANNELS.createStudio, manifestId),
  renameStudio: (studioId: string, name: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.renameStudio, studioId, name),
  // Four kinds and no more: the type is as narrow as the act, so the surface
  // this bridge gains is a note, a link, a sketch or a file rather than any node.
  addStudioNode: (studioId: string, node: StudioNodeByHand, position: StudioPosition, within: string | null): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.addStudioNode, studioId, node, position, within),
  pathOfFile: (file: File): string => webUtils.getPathForFile(file),
  addStudioPicture: (studioId: string, bytes: Uint8Array, position: StudioPosition): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.addStudioPicture, studioId, bytes, position),
  addStudioSketch: (studioId: string, drawing: SketchToKeep, position: StudioPosition, within: string | null): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.addStudioSketch, studioId, drawing, position, within),
  saveStudioSketch: (studioId: string, nodeId: string, drawing: SketchToKeep): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.saveStudioSketch, studioId, nodeId, drawing),
  // `within` is the frame it was put down in, and `null` the board — #1620.
  moveStudioNode: (studioId: string, nodeId: string, position: StudioPosition, within: string | null): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.moveStudioNode, studioId, nodeId, position, within),
  // The frame is main's: this hands over what was pointed at and nothing else,
  // so the capability added here is a Note on a Studio and not a screenshot.
  captureStudioNote: (studioId: string, said: string, capture: StudioCapture): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.captureStudioNote, studioId, said, capture),
  // Reading one, where the capture above takes one: the bytes come back to be
  // drawn and nothing about the file's place on disk crosses with them.
  readStudioFrame: (studioId: string, nodeId: string, picture?: string): Promise<FrameRead> =>
    ipcRenderer.invoke(CHANNELS.readStudioFrame, studioId, nodeId, picture),
  removeStudioNodes: (studioId: string, nodeIds: readonly string[]): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.removeStudioNodes, studioId, [...nodeIds]),
  decideStudioEdge: (studioId: string, edgeId: string, accepted: boolean): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.decideStudioEdge, studioId, edgeId, accepted),
  promoteOnStudio: (studioId: string, promotion: StudioPromotion): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.promoteOnStudio, studioId, promotion),
  startStudioRun: (studioId: string, name: string, position: StudioPosition): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.startStudioRun, studioId, name, position),
  startStudioServer: (studioId: string, name: string, position: StudioPosition): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.startStudioServer, studioId, name, position),

  // The capture window — #1294. A server id and one of that server's own links;
  // main resolves the address off the live holder, as `openServerLink` does.
  openCaptureWindow: (serverId: string, url: string): Promise<CaptureOpened> =>
    ipcRenderer.invoke(CHANNELS.openCaptureWindow, serverId, url),
  // **This half is the capture window's own bar talking to its own window.**
  // Main answers each for the window the call came from, so nothing here names
  // one — and the page below the bar holds no preload and reaches none of it.
  captureWindow: {
    read: (onChanged: (state: CaptureWindowState) => void): (() => void) => {
      // Main pushes on `did-finish-load`, which can land before this listener does, so the state is
      // also asked for and drawn unless a push has already said something newer.
      let pushed = false;
      const handler = (_event: unknown, state: CaptureWindowState): void => {
        pushed = true;
        onChanged(state);
      };
      ipcRenderer.on(CHANNELS.captureWindowChanged, handler);
      void ipcRenderer.invoke(CHANNELS.captureWindowRead).then((state: CaptureWindowState | null) => {
        if (state !== null && !pushed) onChanged(state);
      });
      return () => ipcRenderer.removeListener(CHANNELS.captureWindowChanged, handler);
    },
    arm: (on: boolean): Promise<CaptureWindowState | null> => ipcRenderer.invoke(CHANNELS.captureWindowArm, on),
    aim: (x: number, y: number): Promise<CaptureAimed | null> => ipcRenderer.invoke(CHANNELS.captureWindowAim, x, y),
    hold: (x: number, y: number): Promise<CaptureHeld | null> => ipcRenderer.invoke(CHANNELS.captureWindowHold, x, y),
    release: (): Promise<void> => ipcRenderer.invoke(CHANNELS.captureWindowRelease),
    save: (said: string): Promise<Outcome> => ipcRenderer.invoke(CHANNELS.captureWindowSave, said),
    reload: (): Promise<void> => ipcRenderer.invoke(CHANNELS.captureWindowReload),
    followRefused: (): Promise<void> => ipcRenderer.invoke(CHANNELS.captureWindowFollowRefused),
    approve: (): Promise<Outcome> => ipcRenderer.invoke(CHANNELS.captureWindowApprove),
    scroll: (wheel: CaptureWheel): void => ipcRenderer.send(CHANNELS.captureWindowScroll, wheel),
    showAsk: (on: boolean): Promise<void> => ipcRenderer.invoke(CHANNELS.captureWindowShowAsk, on),
    answerAsk: (answer: CaptureAnswer): Promise<Outcome> => ipcRenderer.invoke(CHANNELS.captureWindowAnswerAsk, answer),
    pick: (at: number): Promise<void> => ipcRenderer.invoke(CHANNELS.captureWindowPick, at),
    panel: (open: boolean): Promise<void> => ipcRenderer.invoke(CHANNELS.captureWindowPanel, open),
    openOwner: (): Promise<void> => ipcRenderer.invoke(CHANNELS.captureWindowOpenOwner),
    walkNext: (): Promise<void> => ipcRenderer.invoke(CHANNELS.captureWindowWalkNext),
    walkBack: (): Promise<void> => ipcRenderer.invoke(CHANNELS.captureWindowWalkBack),
    walkAutoplay: (on: boolean): Promise<void> => ipcRenderer.invoke(CHANNELS.captureWindowWalkAutoplay, on),
    walkReplay: (): Promise<void> => ipcRenderer.invoke(CHANNELS.captureWindowWalkReplay),
    walkGuide: (shown: boolean): Promise<void> => ipcRenderer.invoke(CHANNELS.captureWindowWalkGuide, shown),
    // Another walk of the window's, by its place in the queue: main reads which walk off its own queue.
    walkTo: (at: number): Promise<void> => ipcRenderer.invoke(CHANNELS.captureWindowWalkTo, at),
    // How far down the bar's own menu or tooltip draws, or null: main lets the bar's view down over the page's column that far.
    overhang: (height: number | null): void => ipcRenderer.send(CHANNELS.captureWindowOverhang, height),
    // A key main heard while the page had the keyboard, one of six names. It reaches nothing.
    onKey: (onKey: (key: CaptureQueueKey) => void): (() => void) => {
      const handler = (_event: unknown, key: CaptureQueueKey): void => onKey(key);
      ipcRenderer.on(CHANNELS.captureWindowKey, handler);
      return () => ipcRenderer.removeListener(CHANNELS.captureWindowKey, handler);
    },
  },
  // The panel beside a walk window's page — `capture-window.md`, *The panel beside the page*. The call
  // names nothing: main answers for the panel the call came from, and refuses any other document.
  walkPanel: {
    openInBridge: (): Promise<void> => ipcRenderer.invoke(CHANNELS.walkPanelOpenInBridge),
    // The Job of the walk on screen, told as it changes and asked for once, as `captureWindow.read` is.
    subject: (onChanged: (sessionId: string | undefined) => void): (() => void) => {
      let pushed = false;
      const handler = (_event: unknown, sessionId: string | null): void => {
        pushed = true;
        onChanged(sessionId ?? undefined);
      };
      ipcRenderer.on(CHANNELS.walkPanelSubjectChanged, handler);
      void ipcRenderer.invoke(CHANNELS.walkPanelSubject).then((sessionId: string | null) => {
        if (!pushed) onChanged(sessionId ?? undefined);
      });
      return () => ipcRenderer.removeListener(CHANNELS.walkPanelSubjectChanged, handler);
    },
    goInBridge: (to: PanelGo): Promise<void> => ipcRenderer.invoke(CHANNELS.walkPanelGoInBridge, to),
  },

  // What a Studio node points at — #1406. `openPullRequest`'s reason: a Studio
  // id and a node id, never an address the renderer composed.
  openStudioNode: (studioId: string, nodeId: string): Promise<Followed> =>
    ipcRenderer.invoke(CHANNELS.openStudioNode, studioId, nodeId),

  // Where a pressed notification says to go. **The only entry here that the
  // renderer does not initiate** — every other one is the window asking, and
  // this is main handing over a press that happened outside it, possibly with
  // no window on screen at the time.
  //
  // `subscribe`'s shape, and for its reason: a listener plus the function that
  // removes it, so a component that mounts twice does not leave one behind. It
  // reaches nothing and grants nothing: what arrives is a Job id this window
  // already draws, or `null` for the set.
  onSummoned: (onGo: (to: Summons) => void): (() => void) => {
    const handler = (_event: unknown, to: Summons): void => onGo(to);
    ipcRenderer.on(CHANNELS.summoned, handler);
    return () => {
      ipcRenderer.removeListener(CHANNELS.summoned, handler);
    };
  },

  // A swipe or browser key main heard. `onSummoned`'s shape: a listener and its remover; two words arrive.
  onHistory: (onStep: (step: HistoryStep) => void): (() => void) => {
    const handler = (_event: unknown, step: HistoryStep): void => onStep(step);
    ipcRenderer.on(CHANNELS.history, handler);
    return () => {
      ipcRenderer.removeListener(CHANNELS.history, handler);
    };
  },

  askHelm: (text: string, context?: HelmContext, carry?: HelmCarry): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.askHelm, text, context, carry),
  helmDebugInfo: (): Promise<HelmDebugRead> => ipcRenderer.invoke(CHANNELS.helmDebugInfo),
  readThought: (text: string): Promise<ThoughtRead> => ipcRenderer.invoke(CHANNELS.readThought, text),
  recordThoughtSent: (manifestId: string, text: string, target: ThoughtTarget, carry?: HelmCarry): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.recordThoughtSent, manifestId, text, target, carry),
  startHelmFresh: (): Promise<Outcome> => ipcRenderer.invoke(CHANNELS.startHelmFresh),
  pointHelm: (manifestId: string): Promise<void> => ipcRenderer.invoke(CHANNELS.pointHelm, manifestId),

  tap: (pattern: Pattern): void => ipcRenderer.send(CHANNELS.tap, pattern),
};

contextBridge.exposeInMainWorld("armada", api);

// The dev-only annotation layer, #1226 — never on `armada`, which is Fleet's seam.
// Main passes the flag only when the app is not packaged.
if (process.argv.includes(ANNOTATE_FLAG)) {
  const annotations: AnnotationsDevApi = {
    list: (): Promise<Annotation[]> => ipcRenderer.invoke(ANNOTATION_CHANNELS.list),
    save: (note: Annotation): Promise<void> => ipcRenderer.invoke(ANNOTATION_CHANNELS.save, note),
    remove: (id: string): Promise<void> => ipcRenderer.invoke(ANNOTATION_CHANNELS.remove, id),
    root: (): Promise<string | null> => ipcRenderer.invoke(ANNOTATION_CHANNELS.root),
    capture: (box: Box): Promise<ArrayBuffer | null> => ipcRenderer.invoke(ANNOTATION_CHANNELS.capture, box),
    saveShot: (id: string, png: ArrayBuffer): Promise<void> => ipcRenderer.invoke(ANNOTATION_CHANNELS.saveShot, id, png),
    shot: (id: string): Promise<ArrayBuffer | null> => ipcRenderer.invoke(ANNOTATION_CHANNELS.shot, id),
  };
  contextBridge.exposeInMainWorld("armadaDev", annotations);
}
