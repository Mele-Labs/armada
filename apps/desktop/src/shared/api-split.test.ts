// The old, monolithic shapes of BridgeApi, BridgeState, NOTHING_YET and CHANNELS, frozen as they
// were before the API split (comments stripped). The composed names must stay assignable both ways.
// Kept, not deleted: a slice-by-slice equality cannot be asserted against anything but the whole.

import { describe, expect, it } from "vitest";
import type { StateChange } from "./state-change";
import type { KitFileOpened } from "@armada/manifest/api";
import type { PhoneAnswer, PhoneRequest, SettingsFileOpened } from "@armada/settings/api";
import type { AskLessonRead, AskTurn, LessonReviewRead } from "@armada/screens/src/lessons/review-wire";
import type { LessonSessionRead } from "@armada/screens/src/lessons/session-wire";
import type { PanelGo, PanelHeldNote } from "@armada/studios/api";
import type {
  ChangeWaiting,
  SessionWait,
  WaitReading,
  ChosenHold,
  SessionHeld,
  SessionHoldsReleased,
  CaptureOpened,
  CheckOutputRead,
  ClashSide,
  LessonAnswer,
  LessonsRead,
  RetroRead,
  RetroSubject,
  HelmCallAnswer,
  Followed,
  FleetRestart,
  FrameRead,
  Outcome,
  RunOutputRead,
  CheckoutRunListRead,
  SavePreference,
  SaveSettings,
  SettingsList,
  StartCheckoutRun,
  ChangeSlotPool,
  LandCheckAt,
  RescueSlot,
  EditManifest,
  SaveManifestFile,
  SketchToKeep,
  HelmContext,
  HelmDebugRead,
  CheckoutRunDiffRead,
  AddKitServer,
  ManifestReach,
  ReachesDrones,
  StudioCapture,
  StudioNodeByHand,
  StudioPosition,
  StudioPromotion,
  EditManifestProposal,
  WriteManifestProposal,
  BridgeIdentity,
  Connection,
  HeldWorktrees,
  ThoughtRead,
  ThoughtTarget,
  Holdings,
  FollowedLog,
  BuildSource,
  FleetBuildReport,
  FleetLimits,
  MergeLines,
  ModChecked,
  ModList,
  Preferences,
  RepositorySummary,
  ManifestReading,
  FollowedLandLog,
  ServerList,
  CheckoutRunFollowed,
  CheckoutRunSheetRead,
  ManifestDriftRead,
} from "@armada/protocol";
import type {
  CaptureAimed,
  CaptureAnswer,
  CaptureHeld,
  CaptureQueueKey,
  CaptureWheel,
  CaptureWindowState,
} from "./capture-window";
import type {
  Pattern,
} from "./haptics";
import type {
  RescueOutcome,
} from "@armada/cleanup/api";
import type {
  ManifestEditAnswer,
  ManifestFileRead,
  ManifestSaveAnswer,
} from "@armada/screens/src/editing";
import type { ArtifactRead, LoadoutItemAsk, LoadoutItemBody, PageBounds } from "@armada/screens/src/draft/sessions";
import type {
  RepositoryAllowedCommandsRead,
} from "@armada/screens/src/manifest-allows";
import type {
  KitAllowedCommandsRead,
  KitInventoryRead,
  KitServersRead,
} from "@armada/screens/src/manifest-kit";
import type {
  SavingWorkflow,
  WorkflowDefinitionRead,
  WorkflowSaveAnswer,
  WorkflowsRead,
} from "./workflows";
import type {
  ReadingTrigger,
  RemovingTrigger,
  SavingTrigger,
  TriggerDefinitionRead,
  TriggerRemoveAnswer,
  TriggerSaveAnswer,
  TriggersRead,
} from "./triggers";
import type {
  LocateAnswer,
} from "@armada/screens/src/locate-reads";
import type {
  StudioAnswer,
  StudioRead,
  StudiosRead,
} from "@armada/screens/src/studio-reads";
import type {
  ManifestProposalsRead,
  ProposalAnswer,
  RepositoryScanRead,
} from "@armada/screens/src/setup-reads";
import type { HealthRead } from "@armada/screens/src/overview-reads";
import type {
  Outstanding,
} from "@armada/screens/src/outstanding";
import type { AnswerSessionAsk, AnswerWaiting, ApprovePlan, CheckoutCaughtUp, CheckoutDiff, CheckoutSentAsPullRequest, ClaimPullRequest, DismissConcern, DismissWaiting, JudgeSession, PullRequestClaimed, PullRequestFilePatch, PullRequestPage, PullRequestState, RenameSession, SendConcern, SendPlanChanges, NoteSent, SendSessionMessage, SendSessionNote, SessionRow, SessionRunMessage, SessionRunUnderway, SessionLoadout, SessionSubagent, TuneSession } from "@armada/protocol";
import type { RequestRepositoryRead } from "./api/dispatch";
import type { PullRequestPress, PullRequestWrite, SessionActed, SessionsRead } from "./api/sessions";
import type { SleepActed } from "./api/sleep";
import type { AccountsActed } from "./api/accounts";
import type { PlanUsageRead } from "./api/plan-usage";
import type { ModelDownshiftRead } from "./api/model-downshift";
import type { FleetTimelineRead } from "./api/timeline";
import type { BayHistoryRead } from "./api/bay-history";
import type { UsageRulesRead, UsageSummaryRead } from "./api/usage-summary";
import type { ClashFileAnswer, WriteClashesRead } from "./api/clashes";
import type { RepositoryAreasRead, SlotTouchesRead } from "./api/territory";
import type { IssueNumberRead, IssuesRead, PullRequestNumberRead } from "./api/issues";
import type { HelmAim, HelmCarry } from "./api/helm";
import { AUTOPILOT_CHANNELS, type AutopilotApi } from "./api/autopilot";
import { SKETCH_SHARE_CHANNELS, type SketchShareApi } from "./api/sketch-share";
import { REVIEWS_CHANNELS, type ReviewsApi } from "./api/reviews";
import { FLIGHT_CONTROLS_CHANNELS, type FlightControlsApi } from "./api/flight-controls";
import { ONBOARDING_CHANNELS, type OnboardingApi } from "./api/onboarding";
import type { ModelDownshiftState, PlanUsage, SignedOut, SleepState, TimelineAsk, TimelineEvent } from "@armada/protocol";
import type { BridgeApi } from "./api";
import { CHANNELS, NOTHING_YET } from "./bridge";
import type { BridgeState, HistoryStep, Summons } from "./bridge";

type OldBridgeState = {
    connection: Connection;
    bridge: BridgeIdentity;
    fleetBuild: FleetBuildReport | null;
    limits: FleetLimits | null;
    preferences: Preferences;
    settings: SettingsList | null;
    mods: ModList | null;
    manifestReading: ManifestReading | null;
    missed: number;
    readAt: number | null;
    holds: Holdings;
    repository: string | null;
    located: {
        repository: RepositorySummary;
        at: number;
    } | null;
    followed: FollowedLog;
    held: HeldWorktrees;
    servers: ServerList;
    mergeLines: MergeLines | null;
    landFollowed: FollowedLandLog;
    checkoutRunSheet: CheckoutRunSheetRead;
    checkoutRunFollowed: CheckoutRunFollowed;
    manifestDrift: ManifestDriftRead;
    health: HealthRead;
    questions: Outstanding[];
    helm: HelmAim;
    studios: StudiosRead;
    studio: StudioRead;
    sessions: SessionsRead;
    sessionThreads: Record<string, SessionRow[]>;
    sessionThreadsReading: string[];
};

type OldBridgeApi = {
    state: () => Promise<OldBridgeState>;
    subscribeChanges: (onChange: (change: StateChange<OldBridgeState>) => void) => () => void;
    syncState: () => Promise<StateChange<OldBridgeState>>;
    onWalkFocus: (onFocus: (focused: boolean) => void) => () => void;
    stageAttachment: (bytes: ArrayBuffer, filename: string, mimeType: string) => Promise<{
        path: string;
    }>;
    searchFiles: (query: string) => Promise<string[]>;
    readRequestRepository: (words: string) => Promise<RequestRepositoryRead>;
    changeSlotPool: (manifestId: string, change: ChangeSlotPool) => Promise<Outcome>;
    rescueSlot: (manifestId: string, rescue: RescueSlot) => Promise<RescueOutcome>;
    answerHelmCall: (call: string, answer: HelmCallAnswer, note?: string) => Promise<Outcome>;
    savePreference: (save: SavePreference) => Promise<Outcome>;
    saveSettings: (changes: SaveSettings["changes"]) => Promise<Outcome>;
    openSettingsFile: () => Promise<SettingsFileOpened>;
    validateMod: (name: string) => Promise<ModChecked | null>;
    setModEnabled: (name: string, enabled: boolean) => Promise<Outcome>;
    promoteMod: (name: string) => Promise<Outcome>;
    phone: (request: PhoneRequest) => Promise<PhoneAnswer>;
    followCheckOutput: (jobId: string | null, kept: string | null) => Promise<void>;
    followLandCheck: (at: LandCheckAt | null) => Promise<void>;
    watchCheckoutRunSheet: (want: boolean) => Promise<void>;
    observeCheckoutRun: (runId: string | null) => Promise<void>;
    startCheckoutRun: (body: StartCheckoutRun) => Promise<Outcome>;
    stopCheckoutRun: (runId: string) => Promise<Outcome>;
    undoCheckoutRun: (runId: string) => Promise<Outcome>;
    listCheckoutRuns: () => Promise<CheckoutRunListRead>;
    getCheckoutRunOutput: (runId: string) => Promise<RunOutputRead>;
    getCheckoutRunDiff: (runId: string) => Promise<CheckoutRunDiffRead>;
    watchManifestDrift: (want: boolean) => Promise<void>;
    watchOverview: (want: boolean) => Promise<void>;
    startCheckoutVerify: (workspace?: string) => Promise<Outcome>;
    readManifestFile: () => Promise<ManifestFileRead>;
    saveManifestFile: (body: SaveManifestFile) => Promise<ManifestSaveAnswer>;
    editManifest: (body: EditManifest) => Promise<ManifestEditAnswer>;
    readRepositoryScan: () => Promise<RepositoryScanRead>;
    readManifestProposals: () => Promise<ManifestProposalsRead>;
    editManifestProposal: (body: EditManifestProposal) => Promise<ProposalAnswer>;
    writeManifestProposal: (body: WriteManifestProposal) => Promise<ProposalAnswer>;
    listRepositoryAllowedCommands: () => Promise<RepositoryAllowedCommandsRead>;
    removeRepositoryAllowedCommand: (run: string) => Promise<RepositoryAllowedCommandsRead>;
    readKitInventory: () => Promise<KitInventoryRead>;
    removeKitAllowedCommand: (run: string) => Promise<KitAllowedCommandsRead>;
    openKitFile: (path: string, how: "editor" | "finder") => Promise<KitFileOpened>;
    listKitServers: () => Promise<KitServersRead>;
    addKitServer: (adding: AddKitServer) => Promise<KitServersRead>;
    forgetKitServer: (name: string) => Promise<KitServersRead>;
    setKitServerReach: (name: string, drones: ReachesDrones) => Promise<KitServersRead>;
    setManifestServerReach: (name: string, reach: ManifestReach | null) => Promise<KitServersRead>;
    readWorkflows: () => Promise<WorkflowsRead>;
    readWorkflowDefinition: (workflowId: string, source: string) => Promise<WorkflowDefinitionRead>;
    saveWorkflow: (saving: SavingWorkflow) => Promise<WorkflowSaveAnswer>;
    readTriggers: () => Promise<TriggersRead>;
    readTrigger: (reading: ReadingTrigger) => Promise<TriggerDefinitionRead>;
    saveTrigger: (saving: SavingTrigger) => Promise<TriggerSaveAnswer>;
    removeTrigger: (removing: RemovingTrigger) => Promise<TriggerRemoveAnswer>;
    pickRepository: (root: string | null) => Promise<void>;
    chooseFolder: () => Promise<string | null>;
    resolveFolder: (path: string) => Promise<string | null>;
    addRepository: (path: string) => Promise<LocateAnswer>;
    cloneRepository: (url: string, parent: string) => Promise<LocateAnswer>;
    startServer: (name: string, jobId?: string) => Promise<Outcome>;
    stopServer: (serverId: string) => Promise<Outcome>;
    openServerLink: (serverId: string, url: string) => Promise<Followed>;
    openLink: (address: string) => Promise<Followed>;
    restartFleet: () => Promise<FleetRestart>;
    changeFleetBuild: (build: BuildSource, adopt: boolean) => Promise<Outcome>;
    readSessionCheckOutput: (run: number) => Promise<CheckOutputRead>;
    stopSessionCheck: (run: number, force: boolean) => Promise<Outcome>;
    readRetro: (subject: RetroSubject) => Promise<RetroRead>;
    readLessons: (state: "open" | "accepted") => Promise<LessonsRead>;
    agreeLesson: (lessonId: string) => Promise<LessonAnswer>;
    disagreeLesson: (lessonId: string) => Promise<LessonAnswer>;
    startLessonSession: (lessonId: string) => Promise<LessonSessionRead>;
    reviewLessons: () => Promise<LessonReviewRead>;
    askLesson: (lessonId: string, question: string, history: AskTurn[]) => Promise<AskLessonRead>;
    readHeld: (want: boolean) => Promise<void>;
    watchStudios: (manifestId: string | readonly string[] | null) => Promise<void>;
    watchStudio: (studioId: string | null) => Promise<void>;
    createStudio: (manifestId: string) => Promise<StudioAnswer>;
    renameStudio: (studioId: string, name: string) => Promise<Outcome>;
    addStudioNode: (studioId: string, node: StudioNodeByHand, position: StudioPosition, within: string | null) => Promise<Outcome>;
    pathOfFile: (file: File) => string;
    addStudioPicture: (studioId: string, bytes: Uint8Array, position: StudioPosition) => Promise<Outcome>;
    addStudioSketch: (studioId: string, drawing: SketchToKeep, position: StudioPosition, within: string | null) => Promise<Outcome>;
    saveStudioSketch: (studioId: string, nodeId: string, drawing: SketchToKeep) => Promise<Outcome>;
    moveStudioNode: (studioId: string, nodeId: string, position: StudioPosition, within: string | null) => Promise<Outcome>;
    removeStudioNodes: (studioId: string, nodeIds: readonly string[]) => Promise<Outcome>;
    decideStudioEdge: (studioId: string, edgeId: string, accepted: boolean) => Promise<Outcome>;
    captureStudioNote: (studioId: string, said: string, capture: StudioCapture) => Promise<Outcome>;
    readStudioFrame: (studioId: string, nodeId: string, picture?: string) => Promise<FrameRead>;
    promoteOnStudio: (studioId: string, promotion: StudioPromotion) => Promise<Outcome>;
    startStudioRun: (studioId: string, name: string, position: StudioPosition) => Promise<Outcome>;
    startStudioServer: (studioId: string, name: string, position: StudioPosition) => Promise<Outcome>;
    openStudioNode: (studioId: string, nodeId: string) => Promise<Followed>;
    openCaptureWindow: (serverId: string, url: string) => Promise<CaptureOpened>;
    captureWindow: {
        read: (onChanged: (state: CaptureWindowState) => void) => () => void;
        arm: (on: boolean) => Promise<CaptureWindowState | null>;
        aim: (x: number, y: number) => Promise<CaptureAimed | null>;
        hold: (x: number, y: number) => Promise<CaptureHeld | null>;
        release: () => Promise<void>;
        save: (said: string) => Promise<Outcome>;
        reload: () => Promise<void>;
        followRefused: () => Promise<void>;
        approve: () => Promise<Outcome>;
        scroll: (wheel: CaptureWheel) => void;
        showAsk: (on: boolean) => Promise<void>;
        answerAsk: (answer: CaptureAnswer) => Promise<Outcome>;
        pick: (at: number) => Promise<void>;
        panel: (open: boolean) => Promise<void>;
        openOwner: () => Promise<void>;
        walkNext: () => Promise<void>;
        walkBack: () => Promise<void>;
        walkAutoplay: (on: boolean) => Promise<void>;
        walkReplay: () => Promise<void>;
        walkGuide: (shown: boolean) => Promise<void>;
        walkTo: (at: number) => Promise<void>;
        sendNotes: () => Promise<Outcome>;
        overhang: (height: number | null) => void;
        onKey: (onKey: (key: CaptureQueueKey) => void) => () => void;
    };
    walkPanel: {
        openInBridge: () => Promise<void>;
        subject: (onChanged: (sessionId: string | undefined) => void) => () => void;
        goInBridge: (to: PanelGo) => Promise<void>;
        held: (onChanged: (notes: PanelHeldNote[]) => void) => () => void;
        editNote: (id: string, said: string) => Promise<Outcome>;
        removeNote: (id: string) => Promise<Outcome>;
        sendNotes: () => Promise<Outcome>;
    };
    onSummoned: (onGo: (to: Summons) => void) => () => void;
    onHistory: (onStep: (step: HistoryStep) => void) => () => void;
    askHelm: (text: string, context?: HelmContext, carry?: HelmCarry) => Promise<Outcome>;
    helmDebugInfo: () => Promise<HelmDebugRead>;
    readThought: (text: string) => Promise<ThoughtRead>;
    recordThoughtSent: (manifestId: string, text: string, target: ThoughtTarget, carry?: HelmCarry) => Promise<Outcome>;
    startHelmFresh: () => Promise<Outcome>;
    pointHelm: (manifestId: string) => Promise<void>;
    tap: (pattern: Pattern) => void;
    startSession: (title?: string) => Promise<SessionActed>;
    sendSessionMessage: (send: SendSessionMessage) => Promise<SessionActed>;
    sendSessionNote: (send: SendSessionNote) => Promise<SessionActed<NoteSent>>;
    answerSessionAsk: (answer: AnswerSessionAsk) => Promise<SessionActed>;
    answerWaiting: (answer: AnswerWaiting) => Promise<SessionActed>;
    claimPullRequest: (claim: ClaimPullRequest) => Promise<SessionActed<PullRequestClaimed>>;
    readSessionWait: (sessionId: string, request: string) => Promise<SessionActed<WaitReading>>;
    changeWaiting: (change: ChangeWaiting) => Promise<SessionActed<SessionWait>>;
    startWaitingNow: (sessionId: string) => Promise<SessionActed>;
    dismissWaiting: (dismiss: DismissWaiting) => Promise<SessionActed>;
    approvePlan: (approve: ApprovePlan) => Promise<SessionActed>;
    sendPlanChanges: (changes: SendPlanChanges) => Promise<SessionActed>;
    judgeSession: (judge: JudgeSession) => Promise<SessionActed>;
    sendConcern: (send: SendConcern) => Promise<SessionActed>;
    dismissConcern: (dismiss: DismissConcern) => Promise<SessionActed>;
    getSleep: () => Promise<SleepActed>;
    getPlanUsage: () => Promise<PlanUsageRead>;
    getWriteClashes: () => Promise<WriteClashesRead>;
    readClashFile: (sessionId: string, other: { kind: ClashSide; id: string }, path: string) => Promise<ClashFileAnswer>;
    getRepositoryAreas: () => Promise<RepositoryAreasRead>;
    getSlotTouches: () => Promise<SlotTouchesRead>;
    getFleetTimeline: (ask?: TimelineAsk) => Promise<FleetTimelineRead>;
    onTimelineAppended: (onAppended: (event: TimelineEvent) => void) => () => void;
    getSlotHistory: (manifestId: string, slot: number) => Promise<BayHistoryRead>;
    getUsageSummary: (tzOffsetMinutes: number) => Promise<UsageSummaryRead>;
    getUsageRules: () => Promise<UsageRulesRead>;
    onPlanUsageChanged: (onChanged: (usage: PlanUsage) => void) => () => void;
    getModelDownshift: () => Promise<ModelDownshiftRead>;
    acceptDownshift: () => Promise<ModelDownshiftRead>;
    restoreDownshift: () => Promise<ModelDownshiftRead>;
    keepDownshift: () => Promise<ModelDownshiftRead>;
    settleDownshift: () => Promise<ModelDownshiftRead>;
    onModelDownshiftChanged: (onChanged: (state: ModelDownshiftState) => void) => () => void;
    listIssues: (manifestId: string, refresh: boolean) => Promise<IssuesRead>;
    getIssue: (manifestId: string, reference: string) => Promise<IssueNumberRead>;
    getPullRequest: (manifestId: string, number: number) => Promise<PullRequestNumberRead>;
    setSleep: (on: boolean) => Promise<SleepActed>;
    overrideSleep: (id: string, text: string) => Promise<SleepActed>;
    reviewSleep: (id: string) => Promise<SleepActed>;
    getAccounts: () => Promise<AccountsActed>;
    startSignIn: () => Promise<AccountsActed>;
    signOut: (accountId: number) => Promise<AccountsActed<SignedOut>>;
    setRepositoryProfile: (root: string, accountId: number | null) => Promise<AccountsActed>;
    onSleepChanged: (onChanged: (state: SleepState) => void) => () => void;
    tuneSession: (tune: TuneSession) => Promise<SessionActed>;
    runSessionCommand: (sessionId: string, command: string) => Promise<SessionActed>;
    renameSession: (rename: RenameSession) => Promise<SessionActed>;
    forkSession: (sessionId: string) => Promise<SessionActed>;
    forkSessionAt: (sessionId: string, at: string) => Promise<SessionActed>;
    retroSession: (sessionId: string) => Promise<Outcome>;
    closeSession: (sessionId: string) => Promise<SessionActed>;
    stopSession: (sessionId: string) => Promise<SessionActed>;
    retryHandoff: (sessionId: string) => Promise<SessionActed>;
    pauseSession: (sessionId: string) => Promise<SessionActed>;
    resumeSession: (sessionId: string) => Promise<SessionActed>;
    setSessionAutoLand: (sessionId: string, on: boolean) => Promise<SessionActed>;
    setSessionWorkflow: (sessionId: string, workflow?: string) => Promise<SessionActed>;
    releaseSessionHolds: (sessionId: string, toSessionId?: string, rows?: ChosenHold[]) => Promise<SessionActed<SessionHoldsReleased>>;
    handToSession: (sessionId: string, group: string) => Promise<SessionActed<SessionHeld>>;
    moveSession: (sessionId: string, manifestId: string) => Promise<SessionActed>;
    watchSession: (sessionId: string) => Promise<void>;
    readSessionFile: (sessionId: string, file: string) => Promise<FrameRead>;
    readSessionSubagent: (sessionId: string, subagentId: string) => Promise<SessionActed<SessionSubagent>>;
    readSessionLoadout: (sessionId: string) => Promise<SessionActed<SessionLoadout>>;
    readLoadoutItem: (sessionId: string, item: LoadoutItemAsk) => Promise<SessionActed<LoadoutItemBody>>;
    openSessionFile: (sessionId: string, path: string) => Promise<Followed>;
    openSessionWindow: (sessionId: string, url: string) => Promise<Outcome>;
    getSessionWindows: () => Promise<readonly string[]>;
    onSessionWindows: (onChanged: (addresses: readonly string[]) => void) => () => void;
    readSessionArtifact: (sessionId: string, path: string) => Promise<ArtifactRead>;
    showSessionPage: (sessionId: string, address: string, bounds: PageBounds) => Promise<Followed>;
    moveSessionPage: (bounds: PageBounds) => Promise<void>;
    hideSessionPage: () => Promise<void>;
    onSessionPageEscape: (on: () => void) => () => void;
    pressPullRequest: (sessionId: string, number: number, press: PullRequestPress) => Promise<SessionActed<PullRequestState>>;
    pressRepositoryPullRequest: (manifestId: string, number: number, press: PullRequestPress) => Promise<SessionActed<PullRequestState>>;
    readPullRequestPage: (sessionId: string, number: number) => Promise<SessionActed<PullRequestPage>>;
    readPullRequestFile: (sessionId: string, number: number, path: string) => Promise<SessionActed<PullRequestFilePatch>>;
    writePullRequest: (sessionId: string, number: number, write: PullRequestWrite) => Promise<SessionActed<PullRequestPage>>;
    startSessionRun: (sessionId: string, command: string, cols?: number, rows?: number) => Promise<SessionActed<SessionRunUnderway>>;
    catchUpCheckout: (manifestId: string) => Promise<SessionActed<CheckoutCaughtUp>>;
    sendCheckoutAsPullRequest: (manifestId: string) => Promise<SessionActed<CheckoutSentAsPullRequest>>;
    readCheckoutDiff: (manifestId: string, path: string) => Promise<SessionActed<CheckoutDiff>>;
    stopSessionRun: (runId: string) => Promise<SessionActed<SessionRunUnderway>>;
    writeSessionRun: (runId: string, data: string) => Promise<SessionActed<SessionRunUnderway>>;
    resizeSessionRun: (runId: string, cols: number, rows: number) => Promise<SessionActed<SessionRunUnderway>>;
    onSessionRunMessage: (on: (runId: string, message: SessionRunMessage) => void) => () => void;
} & AutopilotApi & SketchShareApi & ReviewsApi & FlightControlsApi & OnboardingApi;

const OLD_NOTHING_YET: OldBridgeState = {
    connection: { state: "reading" },
    bridge: { auditPath: null, fleetProtocol: null },
    fleetBuild: null,
    missed: 0,
    readAt: null,
    holds: { workflows: [], manifests: [], models: null, repositories: [] },
    repository: null,
    servers: { servers: [] },
    studios: { state: "none" },
    studio: { state: "none" },
    manifestReading: null,
    checkoutRunSheet: { state: "none" },
    checkoutRunFollowed: { state: "none" },
    manifestDrift: { state: "none" },
    located: null,
      questions: [],
    helm: { state: "none" },
    settings: null,
    limits: null,
    preferences: { where_things_are_open: false },
    mods: null,
    health: { state: "none" },
    held: { state: "none" },
    followed: { state: "none" },
    mergeLines: null,
    landFollowed: { state: "none" },
    sessions: { state: "none" },
    sessionThreads: {},
    sessionThreadsReading: [],
};

const OLD_CHANNELS = {
    state: "bridge:state",
    changed: "bridge:changed",
    syncState: "bridge:sync-state",
    walkFocused: "bridge:walk-focused",
    stageAttachment: "bridge:stage-attachment",
    searchFiles: "bridge:search-files",
    readRequestRepository: "bridge:read-request-repository",
    changeSlotPool: "bridge:change-slot-pool",
    rescueSlot: "bridge:rescue-slot",
    answerHelmCall: "bridge:answer-helm-call",
    savePreference: "bridge:save-preference",
    saveSettings: "bridge:save-settings",
    openSettingsFile: "bridge:open-settings-file",
    validateMod: "bridge:validate-mod",
    setModEnabled: "bridge:set-mod-enabled",
    promoteMod: "bridge:promote-mod",
    phone: "bridge:phone",
    followCheckOutput: "bridge:follow-check-output",
    followLandCheck: "bridge:follow-land-check",
    watchCheckoutRunSheet: "bridge:watch-checkout-run-sheet",
    observeCheckoutRun: "bridge:observe-checkout-run",
    startCheckoutRun: "bridge:start-checkout-run",
    stopCheckoutRun: "bridge:stop-checkout-run",
    undoCheckoutRun: "bridge:undo-checkout-run",
    listCheckoutRuns: "bridge:list-checkout-runs",
    getCheckoutRunOutput: "bridge:get-checkout-run-output",
    getCheckoutRunDiff: "bridge:get-checkout-run-diff",
    watchManifestDrift: "bridge:watch-manifest-drift",
    watchOverview: "bridge:watch-overview",
    startCheckoutVerify: "bridge:start-checkout-verify",
    readManifestFile: "bridge:read-manifest-file",
    saveManifestFile: "bridge:save-manifest-file",
    editManifest: "bridge:edit-manifest",
    readRepositoryScan: "bridge:read-repository-scan",
    readManifestProposals: "bridge:read-manifest-proposals",
    editManifestProposal: "bridge:edit-manifest-proposal",
    writeManifestProposal: "bridge:write-manifest-proposal",
    listRepositoryAllowedCommands: "bridge:list-repository-allowed-commands",
    removeRepositoryAllowedCommand: "bridge:remove-repository-allowed-command",
    readKitInventory: "bridge:read-kit-inventory",
    removeKitAllowedCommand: "bridge:remove-kit-allowed-command",
    openKitFile: "bridge:open-kit-file",
    listKitServers: "bridge:list-kit-servers",
    addKitServer: "bridge:add-kit-server",
    forgetKitServer: "bridge:forget-kit-server",
    setKitServerReach: "bridge:set-kit-server-reach",
    setManifestServerReach: "bridge:set-manifest-server-reach",
    readWorkflows: "bridge:read-workflows",
    readWorkflowDefinition: "bridge:read-workflow-definition",
    saveWorkflow: "bridge:save-workflow",
    readTriggers: "bridge:read-triggers",
    readTrigger: "bridge:read-trigger",
    saveTrigger: "bridge:save-trigger",
    removeTrigger: "bridge:remove-trigger",
    pickRepository: "bridge:pick-repository",
    chooseFolder: "bridge:choose-folder",
    resolveFolder: "bridge:resolve-folder",
    addRepository: "bridge:add-repository",
    cloneRepository: "bridge:clone-repository",
    startServer: "bridge:start-server",
    stopServer: "bridge:stop-server",
    openServerLink: "bridge:open-server-link",
    openLink: "bridge:open-link",
    restartFleet: "bridge:restart-fleet",
    changeFleetBuild: "bridge:change-fleet-build",
    readSessionCheckOutput: "bridge:read-session-check-output",
    stopSessionCheck: "bridge:stop-session-check",
    readRetro: "bridge:read-retro",
    readLessons: "bridge:read-lessons",
    agreeLesson: "bridge:agree-lesson",
    disagreeLesson: "bridge:disagree-lesson",
    startLessonSession: "bridge:start-lesson-session",
    reviewLessons: "bridge:review-lessons",
    askLesson: "bridge:ask-lesson",
    readHeld: "bridge:read-held",
    summoned: "bridge:summoned",
    history: "bridge:history",
    askHelm: "bridge:ask-helm",
    helmDebugInfo: "bridge:helm-debug-info",
    readThought: "bridge:read-thought",
    recordThoughtSent: "bridge:record-thought-sent",
    startHelmFresh: "bridge:start-helm-fresh",
    pointHelm: "bridge:point-helm",
    watchStudios: "bridge:watch-studios",
    watchStudio: "bridge:watch-studio",
    createStudio: "bridge:create-studio",
    renameStudio: "bridge:rename-studio",
    addStudioNode: "bridge:add-studio-node",
    addStudioPicture: "bridge:add-studio-picture",
    addStudioSketch: "bridge:add-studio-sketch",
    saveStudioSketch: "bridge:save-studio-sketch",
    moveStudioNode: "bridge:move-studio-node",
    removeStudioNodes: "bridge:remove-studio-nodes",
    decideStudioEdge: "bridge:decide-studio-edge",
    captureStudioNote: "bridge:capture-studio-note",
    readStudioFrame: "bridge:read-studio-frame",
    promoteOnStudio: "bridge:promote-on-studio",
    startStudioRun: "bridge:start-studio-run",
    startStudioServer: "bridge:start-studio-server",
    openStudioNode: "bridge:open-studio-node",
    openCaptureWindow: "bridge:open-capture-window",
    captureWindowChanged: "bridge:capture-window-changed",
    captureWindowRead: "bridge:capture-window-read",
    captureWindowArm: "bridge:capture-window-arm",
    captureWindowAim: "bridge:capture-window-aim",
    captureWindowHold: "bridge:capture-window-hold",
    captureWindowRelease: "bridge:capture-window-release",
    captureWindowSave: "bridge:capture-window-save",
    captureWindowReload: "bridge:capture-window-reload",
    captureWindowFollowRefused: "bridge:capture-window-follow-refused",
    captureWindowApprove: "bridge:capture-window-approve",
    captureWindowScroll: "bridge:capture-window-scroll",
    captureWindowShowAsk: "bridge:capture-window-show-ask",
    captureWindowAnswerAsk: "bridge:capture-window-answer-ask",
    captureWindowPick: "bridge:capture-window-pick",
    captureWindowPanel: "bridge:capture-window-panel",
    captureWindowOpenOwner: "bridge:capture-window-open-owner",
    captureWindowWalkNext: "bridge:capture-window-walk-next",
    captureWindowWalkBack: "bridge:capture-window-walk-back",
    captureWindowWalkAutoplay: "bridge:capture-window-walk-autoplay",
    captureWindowWalkReplay: "bridge:capture-window-walk-replay",
    captureWindowWalkGuide: "bridge:capture-window-walk-guide",
    captureWindowWalkTo: "bridge:capture-window-walk-to",
    captureWindowKey: "bridge:capture-window-key",
    captureWindowOverhang: "bridge:capture-window-overhang",
    walkPanelOpenInBridge: "bridge:walk-panel-open-in-bridge",
    walkPanelSubject: "bridge:walk-panel-subject",
    walkPanelSubjectChanged: "bridge:walk-panel-subject-changed",
    walkPanelGoInBridge: "bridge:walk-panel-go-in-bridge",
    walkPanelHeld: "bridge:walk-panel-held",
    walkPanelHeldChanged: "bridge:walk-panel-held-changed",
    walkPanelEditNote: "bridge:walk-panel-edit-note",
    walkPanelRemoveNote: "bridge:walk-panel-remove-note",
    walkPanelSendNotes: "bridge:walk-panel-send-notes",
    captureWindowSendNotes: "bridge:capture-window-send-notes",
    tap: "bridge:tap",
    startSession: "bridge:start-session",
    forkSession: "bridge:fork-session",
    forkSessionAt: "bridge:fork-session-at",
    sendSessionMessage: "bridge:send-session-message",
    sendSessionNote: "bridge:send-session-note",
    answerSessionAsk: "bridge:answer-session-ask",
    answerWaiting: "bridge:answer-waiting",
    claimPullRequest: "bridge:claim-pull-request",
    readSessionWait: "bridge:read-session-wait",
    changeWaiting: "bridge:change-waiting",
    startWaitingNow: "bridge:start-waiting-now",
    dismissWaiting: "bridge:dismiss-waiting",
    approvePlan: "bridge:approve-plan",
    sendPlanChanges: "bridge:send-plan-changes",
    judgeSession: "bridge:judge-session",
    sendConcern: "bridge:send-concern",
    dismissConcern: "bridge:dismiss-concern",
    getSleep: "bridge:get-sleep",
    getPlanUsage: "bridge:get-plan-usage",
    getWriteClashes: "bridge:get-write-clashes",
    readClashFile: "bridge:read-clash-file",
    getRepositoryAreas: "bridge:get-repository-areas",
    getSlotTouches: "bridge:get-slot-touches",
    getFleetTimeline: "bridge:get-fleet-timeline",
    timelineAppended: "bridge:timeline-appended",
    getSlotHistory: "bridge:get-slot-history",
    getUsageSummary: "bridge:get-usage-summary",
    getUsageRules: "bridge:get-usage-rules",
    planUsageChanged: "bridge:plan-usage-changed",
    getModelDownshift: "bridge:get-model-downshift",
    acceptDownshift: "bridge:accept-downshift",
    restoreDownshift: "bridge:restore-downshift",
    keepDownshift: "bridge:keep-downshift",
    settleDownshift: "bridge:settle-downshift",
    modelDownshiftChanged: "bridge:model-downshift-changed",
    listIssues: "bridge:list-issues",
    getIssue: "bridge:get-issue",
    getPullRequest: "bridge:get-pull-request",
    setSleep: "bridge:set-sleep",
    overrideSleep: "bridge:override-sleep",
    reviewSleep: "bridge:review-sleep",
    getAccounts: "bridge:get-accounts",
    startSignIn: "bridge:start-sign-in",
    signOut: "bridge:sign-out",
    setRepositoryProfile: "bridge:set-repository-profile",
    sleepChanged: "bridge:sleep-changed",
    tuneSession: "bridge:tune-session",
    runSessionCommand: "bridge:run-session-command",
    renameSession: "bridge:rename-session",
    retroSession: "bridge:retro-session",
    closeSession: "bridge:close-session",
    stopSession: "bridge:stop-session",
    retryHandoff: "bridge:retry-handoff",
    pauseSession: "bridge:pause-session",
    resumeSession: "bridge:resume-session",
    setSessionAutoLand: "bridge:set-session-auto-land",
    setSessionWorkflow: "bridge:set-session-workflow",
    releaseSessionHolds: "bridge:release-session-holds",
    handToSession: "bridge:hand-to-session",
    moveSession: "bridge:move-session",
    watchSession: "bridge:watch-session",
    readSessionFile: "bridge:read-session-file",
    readSessionSubagent: "bridge:read-session-subagent",
    readSessionLoadout: "bridge:read-session-loadout",
    readLoadoutItem: "bridge:read-loadout-item",
    openSessionFile: "bridge:open-session-file",
    openSessionWindow: "bridge:open-session-window",
    getSessionWindows: "bridge:get-session-windows",
    sessionWindowsChanged: "bridge:session-windows-changed",
    readSessionArtifact: "bridge:read-session-artifact",
    showSessionPage: "bridge:show-session-page",
    moveSessionPage: "bridge:move-session-page",
    hideSessionPage: "bridge:hide-session-page",
    sessionPageEscape: "bridge:session-page-escape",
    pressPullRequest: "bridge:press-pull-request",
    pressRepositoryPullRequest: "bridge:press-repository-pull-request",
    readPullRequestPage: "bridge:read-pull-request-page",
    readPullRequestFile: "bridge:read-pull-request-file",
    writePullRequest: "bridge:write-pull-request",
    startSessionRun: "bridge:start-session-run",
    catchUpCheckout: "bridge:catch-up-checkout",
    sendCheckoutAsPullRequest: "bridge:send-checkout-as-pull-request",
    readCheckoutDiff: "bridge:read-checkout-diff",
    stopSessionRun: "bridge:stop-session-run",
    writeSessionRun: "bridge:write-session-run",
    resizeSessionRun: "bridge:resize-session-run",
    sessionRunMessage: "bridge:session-run-message",
    ...AUTOPILOT_CHANNELS,
    ...SKETCH_SHARE_CHANNELS,
    ...REVIEWS_CHANNELS,
    ...FLIGHT_CONTROLS_CHANNELS,
    ...ONBOARDING_CHANNELS,
} as const;

type Same<A, B> = [A] extends [B] ? ([B] extends [A] ? true : false) : false;
const same = <T extends true>(_: T) => _;

same<Same<BridgeApi, OldBridgeApi>>(true);
same<Same<BridgeState, OldBridgeState>>(true);
same<Same<typeof CHANNELS, typeof OLD_CHANNELS>>(true);
same<Same<typeof NOTHING_YET, OldBridgeState>>(true);

describe("the API split", () => {
  it("composes the same channels", () => {
    expect(CHANNELS).toEqual(OLD_CHANNELS);
    expect(Object.keys(CHANNELS).sort()).toEqual(Object.keys(OLD_CHANNELS).sort());
  });

  it("composes the same empty state", () => {
    expect(NOTHING_YET).toEqual(OLD_NOTHING_YET);
    expect(Object.keys(NOTHING_YET).sort()).toEqual(Object.keys(OLD_NOTHING_YET).sort());
  });
});
