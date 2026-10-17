// The old, monolithic shapes of BridgeApi, BridgeState, NOTHING_YET and CHANNELS, frozen as they
// were before the API split (comments stripped). The composed names must stay assignable both ways.
// Kept, not deleted: a slice-by-slice equality cannot be asserted against anything but the whole.

import { describe, expect, it } from "vitest";
import type { StateChange } from "./state-change";
import type { PhoneAnswer, PhoneRequest, SettingsFileOpened } from "@armada/settings/api";
import type { AskLessonRead, AskTurn, LessonReviewRead } from "@armada/jobs/review-wire";
import type { LessonSessionRead } from "@armada/jobs/session-wire";
import type {
  ChangeWaiting,
  SessionWait,
  WaitReading,
  ChosenHold,
  SessionHeld,
  SessionHoldsReleased,
  AddTask,
  FixMain,
  EditTask,
  MovePlan,
  ApproveWave,
  EditJob,
  ToProposer,
  ApproveDispatch,
  BranchesRead,
  Artifact,
  CaptureOpened,
  CheckOutputRead,
  BriefRead,
  LessonAnswer,
  LessonsRead,
  RetroRead,
  RetroSubject,
  ChooseTriggerFix,
  HoldAct,
  ClearOutcome,
  AlwaysAllowScope,
  AnswerMode,
  Active,
  CommanderThread,
  CommandAnswer,
  HelmCallAnswer,
  CommandExplainedRead,
  DropTask,
  FileReport,
  Followed,
  FleetRestart,
  FrameRead,
  JudgeAnswer,
  Opened,
  Outcome,
  Proposed,
  ReclaimOutcome,
  RunListRead,
  RunOutputRead,
  CheckoutRunListRead,
  SavePreference,
  SaveSettings,
  SettingsList,
  StagedAttachment,
  StartCheckoutRun,
  StartRun,
  WhenBlocked,
  WhenRefused,
  ChangeSlotPool,
  LandCheckAt,
  RescueSlot,
  EditManifest,
  SaveManifestFile,
  SketchToKeep,
  HelmContext,
  HelmDebugRead,
  CheckoutRunDiffRead,
  ManifestChecksRead,
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
  Diff,
  Evidence,
  Examination,
  Footprint,
  Handed,
  Crewed,
  Holds,
  HeldWorktrees,
  ThoughtRead,
  ThoughtTarget,
  History,
  Holdings,
  FollowedLog,
  Journalled,
  Observed,
  Remarks,
  Reports,
  Watched,
  BuildSource,
  FleetBuildReport,
  FleetCapacity,
  FleetLimits,
  JobSummary,
  MergeLines,
  ModChecked,
  ModList,
  Preferences,
  ProposalInFlight,
  RepositorySummary,
  UnreadableJob,
  ManifestReading,
  FollowedLandLog,
  RunFollowed,
  RunSheetRead,
  ServerList,
  CheckoutRunFollowed,
  CheckoutRunSheetRead,
  ManifestDriftRead,
} from "@armada/protocol";
import type {
  CaptureAimed,
  CaptureAnswer,
  CaptureHeld,
  CaptureWheel,
  CaptureWindowState,
} from "./capture-window";
import type {
  Pattern,
} from "./haptics";
import type {
  PlanEditAnswer,
} from "@armada/screens/src/plan-edits";
import type {
  RescueOutcome,
} from "@armada/cleanup/api";
import type {
  ManifestEditAnswer,
  ManifestFileRead,
  ManifestSaveAnswer,
  ManifestSpendRead,
} from "@armada/screens/src/editing";
import type { ArtifactRead, PageBounds } from "@armada/screens/src/draft/sessions";
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
import type { AddingStep, AddStepAnswer, EditingStep, EditStepAnswer, ReadingRepairDiff, RemovingStep, RemoveStepAnswer, RepairDiffAnswer } from "./added-steps";
import type {
  AlertsRead,
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
  ComposingRead,
} from "@armada/screens/src/composing-reads";
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
import type { AnswerSessionAsk, AnswerWaiting, ApprovePlan, CheckoutCaughtUp, CheckoutDiff, CheckoutSentAsPullRequest, ClaimPullRequest, DismissConcern, DismissWaiting, JudgeSession, PilotOutcome, PullRequestClaimed, PullRequestFilePatch, PullRequestPage, PullRequestState, RenameSession, ReviewDispatched, SendConcern, SendPlanChanges, SendSessionMessage, SessionRow, SessionRunMessage, SessionRunUnderway, SessionSubagent, TuneSession } from "@armada/protocol";
import type { PilotExit, PullRequestPress, PullRequestWrite, SessionActed, SessionsRead } from "./api/sessions";
import type { SleepActed } from "./api/sleep";
import type { AccountsActed } from "./api/accounts";
import type { PlanUsageRead } from "./api/plan-usage";
import type { ModelDownshiftRead } from "./api/model-downshift";
import type { FleetTimelineRead } from "./api/timeline";
import type { BayHistoryRead } from "./api/bay-history";
import type { WriteClashesRead } from "./api/clashes";
import type { RepositoryAreasRead, SlotTouchesRead } from "./api/territory";
import type { IssueNumberRead, IssuesRead, PullRequestNumberRead } from "./api/issues";
import type { HelmAim, HelmCarry } from "./api/helm";
import type { RequestRepositoryRead } from "./api/jobs";
import { AUTOPILOT_CHANNELS, type AutopilotApi } from "./api/autopilot";
import { SKETCH_SHARE_CHANNELS, type SketchShareApi } from "./api/sketch-share";
import { REVIEWS_CHANNELS, type ReviewsApi } from "./api/reviews";
import type { ModelDownshiftState, PlanUsage, SignedOut, SleepState, TimelineAsk, TimelineEvent } from "@armada/protocol";
import type { BridgeApi } from "./api";
import { CHANNELS, NOTHING_YET } from "./bridge";
import type { BridgeState, HistoryStep, Summons } from "./bridge";

type OldBridgeState = {
    connection: Connection;
    bridge: BridgeIdentity;
    jobs: JobSummary[];
    unreadable: UnreadableJob[];
    capacity: FleetCapacity | null;
    fleetBuild: FleetBuildReport | null;
    limits: FleetLimits | null;
    preferences: Preferences;
    settings: SettingsList | null;
    mods: ModList | null;
    manifestReading: ManifestReading | null;
    missed: number;
    readAt: number | null;
    approving: string[];
    proposing: ProposalInFlight | null;
    holds: Holdings;
    repository: string | null;
    located: {
        repository: RepositorySummary;
        at: number;
    } | null;
    watched: Watched;
    observed: Observed;
    journalled: Journalled;
    commander: CommanderThread;
    followed: FollowedLog;
    footprint: Footprint;
    handed: Handed;
    history: History;
    evidence: Evidence;
    diff: Diff;
    remarks: Remarks;
    reports: Reports;
    resources: Holds;
    jobDrones: Crewed;
    active: Active;
    examination: Examination;
    held: HeldWorktrees;
    runSheet: RunSheetRead;
    runFollowed: RunFollowed;
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
};

type OldBridgeApi = {
    state: () => Promise<OldBridgeState>;
    subscribeChanges: (onChange: (change: StateChange<OldBridgeState>) => void) => () => void;
    syncState: () => Promise<StateChange<OldBridgeState>>;
    onWalkFocus: (onFocus: (focused: boolean) => void) => () => void;
    proposeFromRequest: (request: string, attachments: readonly StagedAttachment[], repository?: string | null) => Promise<Proposed>;
    stopProposal: () => Promise<Outcome>;
    stageAttachment: (bytes: ArrayBuffer, filename: string, mimeType: string) => Promise<{
        path: string;
    }>;
    searchFiles: (query: string) => Promise<string[]>;
    readRequestRepository: (words: string) => Promise<RequestRepositoryRead>;
    approveDispatch: (jobId: string, approval?: ApproveDispatch) => Promise<Outcome>;
    listBranches: (manifestId: string) => Promise<BranchesRead>;
    redispatchJob: (jobId: string) => Promise<Outcome>;
    killDrone: (jobId: string, droneId?: string) => Promise<Outcome>;
    killJob: (jobId: string) => Promise<Outcome>;
    parkJob: (jobId: string) => Promise<Outcome>;
    resumeJob: (jobId: string) => Promise<Outcome>;
    fixMain: (fix: FixMain) => Promise<Outcome>;
    killProcess: (jobId: string, pid: number) => Promise<Outcome>;
    killProcesses: (jobId: string) => Promise<Outcome>;
    pilotTask: (jobId: string, taskId: string) => Promise<Outcome>;
    restartTask: (jobId: string, taskId: string) => Promise<Outcome>;
    editTask: (jobId: string, taskId: string, edit: EditTask) => Promise<Outcome>;
    movePlan: (jobId: string, move: MovePlan) => Promise<Outcome>;
    approveWave: (jobId: string, wave: ApproveWave) => Promise<Outcome>;
    editJob: (jobId: string, edit: EditJob) => Promise<Outcome>;
    setLandingTarget: (jobId: string, target: string) => Promise<Outcome>;
    toProposer: (jobId: string, body: ToProposer) => Promise<Outcome>;
    clearTerminalJobs: (jobIds: readonly string[]) => Promise<ReclaimOutcome>;
    forgetTerminalJobs: (jobIds: readonly string[]) => Promise<ClearOutcome>;
    reclaimWorktree: (jobId: string) => Promise<Outcome>;
    changeSlotPool: (manifestId: string, change: ChangeSlotPool) => Promise<Outcome>;
    rescueSlot: (manifestId: string, rescue: RescueSlot) => Promise<RescueOutcome>;
    deleteBranch: (jobId: string, tip: string) => Promise<Outcome>;
    forgetJob: (jobId: string) => Promise<Outcome>;
    redirectDrone: (jobId: string, instruction: string, droneId?: string) => Promise<Outcome>;
    answerQuestion: (jobId: string, questionId: string, chose: string | undefined, mode?: AnswerMode) => Promise<Outcome>;
    answerCommand: (jobId: string, call: string, answer: CommandAnswer, note?: string, rule?: string, scope?: AlwaysAllowScope, mode?: AnswerMode) => Promise<Outcome>;
    askCommander: (jobId: string, text: string) => Promise<Outcome>;
    answerCommanderCard: (jobId: string, cardId: string, optionId: string) => Promise<Outcome>;
    answerHelmCall: (call: string, answer: HelmCallAnswer, note?: string) => Promise<Outcome>;
    explainCommand: (jobId: string, callId: string) => Promise<CommandExplainedRead>;
    setWhenBlocked: (jobId: string, whenBlocked: WhenBlocked) => Promise<Outcome>;
    answerJudge: (jobId: string, askedAt: string, answer: JudgeAnswer, note?: string, mode?: AnswerMode) => Promise<Outcome>;
    setWhenRefused: (jobId: string, whenRefused: WhenRefused) => Promise<Outcome>;
    setModel: (jobId: string, model: string | null) => Promise<Outcome>;
    setReviewModel: (jobId: string, model: string | null) => Promise<Outcome>;
    removeAllowedCommand: (jobId: string, run: string) => Promise<Outcome>;
    restartStep: (jobId: string, note?: string) => Promise<Outcome>;
    overrideVerdict: (jobId: string, reason: string) => Promise<Outcome>;
    rerunGate: (jobId: string) => Promise<Outcome>;
    rerunChecks: (jobId: string) => Promise<Outcome>;
    retryCheck: (jobId: string, check: string) => Promise<Outcome>;
    skipCheck: (jobId: string, check: string) => Promise<Outcome>;
    skipChecks: (jobId: string) => Promise<Outcome>;
    showAgain: (jobId: string, spec?: string) => Promise<Outcome>;
    raiseCostCap: (jobId: string, costCapMicros: number) => Promise<Outcome>;
    raiseTurnCap: (jobId: string, turnCap: number) => Promise<Outcome>;
    savePreference: (save: SavePreference) => Promise<Outcome>;
    saveSettings: (changes: SaveSettings["changes"]) => Promise<Outcome>;
    openSettingsFile: () => Promise<SettingsFileOpened>;
    validateMod: (name: string) => Promise<ModChecked | null>;
    setModEnabled: (name: string, enabled: boolean) => Promise<Outcome>;
    promoteMod: (name: string) => Promise<Outcome>;
    phone: (request: PhoneRequest) => Promise<PhoneAnswer>;
    fileReport: (jobId: string, filing: FileReport) => Promise<Outcome>;
    addTask: (jobId: string, add: AddTask) => Promise<PlanEditAnswer>;
    dropTask: (jobId: string, drop: DropTask) => Promise<PlanEditAnswer>;
    watchJob: (jobId: string | null) => Promise<void>;
    observeJob: (jobId: string | null) => Promise<void>;
    followCheckOutput: (jobId: string | null, kept: string | null) => Promise<void>;
    followLandCheck: (at: LandCheckAt | null) => Promise<void>;
    readHistory: (jobId: string | null) => Promise<void>;
    readResources: (jobId: string | null) => Promise<void>;
    followActive: (want: boolean) => Promise<void>;
    watchPulse: (jobId: string | null) => Promise<void>;
    watchRunSheet: (jobId: string | null) => Promise<void>;
    observeRun: (jobId: string | null, runId: string | null) => Promise<void>;
    startRun: (jobId: string, body: StartRun) => Promise<Outcome>;
    stopRun: (jobId: string, runId: string) => Promise<Outcome>;
    undoRun: (jobId: string, runId: string) => Promise<Outcome>;
    listRuns: (jobId: string) => Promise<RunListRead>;
    getRunOutput: (jobId: string, runId: string) => Promise<RunOutputRead>;
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
    readManifestSpend: () => Promise<ManifestSpendRead>;
    readManifestChecks: () => Promise<ManifestChecksRead>;
    readRepositoryScan: () => Promise<RepositoryScanRead>;
    readManifestProposals: () => Promise<ManifestProposalsRead>;
    editManifestProposal: (body: EditManifestProposal) => Promise<ProposalAnswer>;
    writeManifestProposal: (body: WriteManifestProposal) => Promise<ProposalAnswer>;
    listRepositoryAllowedCommands: () => Promise<RepositoryAllowedCommandsRead>;
    removeRepositoryAllowedCommand: (run: string) => Promise<RepositoryAllowedCommandsRead>;
    readKitInventory: () => Promise<KitInventoryRead>;
    removeKitAllowedCommand: (run: string) => Promise<KitAllowedCommandsRead>;
    listKitServers: () => Promise<KitServersRead>;
    addKitServer: (adding: AddKitServer) => Promise<KitServersRead>;
    forgetKitServer: (name: string) => Promise<KitServersRead>;
    setKitServerReach: (name: string, drones: ReachesDrones) => Promise<KitServersRead>;
    setManifestServerReach: (name: string, reach: ManifestReach | null) => Promise<KitServersRead>;
    readWorkflows: () => Promise<WorkflowsRead>;
    readWorkflowDefinition: (workflowId: string, source: string) => Promise<WorkflowDefinitionRead>;
    saveWorkflow: (saving: SavingWorkflow) => Promise<WorkflowSaveAnswer>;
    readTriggers: () => Promise<TriggersRead>;
    readAlerts: () => Promise<AlertsRead>;
    readTrigger: (reading: ReadingTrigger) => Promise<TriggerDefinitionRead>;
    saveTrigger: (saving: SavingTrigger) => Promise<TriggerSaveAnswer>;
    removeTrigger: (removing: RemovingTrigger) => Promise<TriggerRemoveAnswer>;
    chooseTriggerFix: (jobId: string, body: ChooseTriggerFix) => Promise<Outcome>;
    rerunTrigger: (jobId: string, body: HoldAct) => Promise<Outcome>;
    skipTrigger: (jobId: string, body: HoldAct) => Promise<Outcome>;
    addJobStep: (adding: AddingStep) => Promise<AddStepAnswer>;
    removeJobStep: (removing: RemovingStep) => Promise<RemoveStepAnswer>;
    editJobStep: (editing: EditingStep) => Promise<EditStepAnswer>;
    readRepairDiff: (reading: ReadingRepairDiff) => Promise<RepairDiffAnswer>;
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
    examineJob: (jobId: string) => Promise<void>;
    readEvidence: (jobId: string | null) => Promise<void>;
    readDiff: (jobId: string | null) => Promise<void>;
    readCheckOutput: (jobId: string, kept: string) => Promise<CheckOutputRead>;
    readSessionCheckOutput: (run: number) => Promise<CheckOutputRead>;
    stopSessionCheck: (run: number, force: boolean) => Promise<Outcome>;
    readBrief: (jobId: string, name: string) => Promise<BriefRead>;
    readRetro: (subject: RetroSubject) => Promise<RetroRead>;
    readLessons: (state: "open" | "accepted") => Promise<LessonsRead>;
    agreeLesson: (lessonId: string) => Promise<LessonAnswer>;
    disagreeLesson: (lessonId: string) => Promise<LessonAnswer>;
    startLessonSession: (lessonId: string) => Promise<LessonSessionRead>;
    reviewLessons: () => Promise<LessonReviewRead>;
    askLesson: (lessonId: string, question: string, history: AskTurn[]) => Promise<AskLessonRead>;
    readFrame: (jobId: string, kept: string) => Promise<FrameRead>;
    readComposing: (repository: string) => Promise<ComposingRead>;
    frameStreamUrl: (jobId: string, kept: string) => string;
    readReports: (want: boolean) => Promise<void>;
    readHeld: (want: boolean) => Promise<void>;
    watchStudios: (manifestId: string | null) => Promise<void>;
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
    };
    walkPanel: {
        openInBridge: () => Promise<void>;
    };
    approveReview: (jobId: string) => Promise<Outcome>;
    mergePullRequest: (jobId: string) => Promise<Outcome>;
    autoMergePullRequest: (jobId: string) => Promise<Outcome>;
    rerunFailedChecks: (jobId: string) => Promise<Outcome>;
    investigateFailedChecks: (jobId: string) => Promise<Outcome>;
    queueAfterFinding: (jobId: string, finding: string) => Promise<Outcome>;
    fileFindingIssue: (jobId: string, finding: string, title: string, body: string) => Promise<Outcome>;
    openFindingIssue: (jobId: string, finding: string) => Promise<Followed>;
    requestChanges: (jobId: string, note: string, withWalkNotes?: boolean) => Promise<Outcome>;
    removeWalkNote: (jobId: string, noteId: string) => Promise<Outcome>;
    rejectWork: (jobId: string) => Promise<Outcome>;
    readRemarks: (jobId: string | null) => Promise<void>;
    takeUpRemarks: (jobId: string, remarks: string[]) => Promise<Outcome>;
    dismissFinding: (jobId: string, finding: string, reason: string) => Promise<Outcome>;
    openArtifact: (jobId: string, what: Artifact) => Promise<Opened>;
    openInTerminal: (jobId: string) => Promise<Opened>;
    openPullRequest: (jobId: string) => Promise<Followed>;
    openRemarkLink: (jobId: string, remarkId: string) => Promise<Followed>;
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
    pilotJob: (jobId: string, outcome: PilotOutcome) => Promise<SessionActed>;
    exitPilot: (jobId: string, exit: PilotExit, note?: string) => Promise<Outcome>;
    sendSessionMessage: (send: SendSessionMessage) => Promise<SessionActed>;
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
    getRepositoryAreas: () => Promise<RepositoryAreasRead>;
    getSlotTouches: () => Promise<SlotTouchesRead>;
    getFleetTimeline: (ask?: TimelineAsk) => Promise<FleetTimelineRead>;
    onTimelineAppended: (onAppended: (event: TimelineEvent) => void) => () => void;
    getSlotHistory: (manifestId: string, slot: number) => Promise<BayHistoryRead>;
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
    setSessionWorkflow: (sessionId: string, workflow?: string) => Promise<SessionActed>;
    releaseSessionHolds: (sessionId: string, toSessionId?: string, rows?: ChosenHold[]) => Promise<SessionActed<SessionHoldsReleased>>;
    handToSession: (sessionId: string, group: string) => Promise<SessionActed<SessionHeld>>;
    moveSession: (sessionId: string, manifestId: string) => Promise<SessionActed>;
    watchSession: (sessionId: string) => Promise<void>;
    readSessionFile: (sessionId: string, file: string) => Promise<FrameRead>;
    readSessionSubagent: (sessionId: string, subagentId: string) => Promise<SessionActed<SessionSubagent>>;
    openSessionFile: (sessionId: string, path: string) => Promise<Followed>;
    openSessionWindow: (sessionId: string, url: string) => Promise<Outcome>;
    getSessionWindows: () => Promise<readonly string[]>;
    onSessionWindows: (onChanged: (addresses: readonly string[]) => void) => () => void;
    readSessionArtifact: (sessionId: string, path: string) => Promise<ArtifactRead>;
    showSessionPage: (sessionId: string, address: string, bounds: PageBounds) => Promise<Followed>;
    moveSessionPage: (bounds: PageBounds) => Promise<void>;
    hideSessionPage: () => Promise<void>;
    onSessionPageEscape: (on: () => void) => () => void;
    pressPullRequest: (sessionId: string, number: number, press: PullRequestPress) => Promise<SessionActed<PullRequestState | ReviewDispatched>>;
    pressRepositoryPullRequest: (manifestId: string, number: number, press: PullRequestPress) => Promise<SessionActed<PullRequestState | ReviewDispatched>>;
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
} & AutopilotApi & SketchShareApi & ReviewsApi;

const OLD_NOTHING_YET: OldBridgeState = {
    connection: { state: "reading" },
    bridge: { auditPath: null, fleetProtocol: null },
    jobs: [],
    unreadable: [],
    capacity: null,
    fleetBuild: null,
    limits: null,
    preferences: { where_things_are_open: false },
    settings: null,
    mods: null,
    manifestReading: null,
    missed: 0,
    readAt: null,
    approving: [],
    proposing: null,
    holds: { workflows: [], manifests: [], models: null, repositories: [] },
    repository: null,
    located: null,
    watched: { state: "none" },
    observed: { state: "none" },
    journalled: { state: "none" },
    commander: { state: "none" },
    followed: { state: "none" },
    footprint: { state: "none" },
    handed: { state: "none" },
    history: { state: "none" },
    evidence: { state: "none" },
    diff: { state: "none" },
    remarks: { state: "none" },
    reports: { state: "none" },
    resources: { state: "none" },
    jobDrones: { state: "none" },
    active: {},
    examination: { state: "none" },
    held: { state: "none" },
    runSheet: { state: "none" },
    runFollowed: { state: "none" },
    servers: { servers: [] },
    mergeLines: null,
    landFollowed: { state: "none" },
    checkoutRunSheet: { state: "none" },
    checkoutRunFollowed: { state: "none" },
    manifestDrift: { state: "none" },
    health: { state: "none" },
      questions: [],
    helm: { state: "none" },
    studios: { state: "none" },
    studio: { state: "none" },
    sessions: { state: "none" },
    sessionThreads: {},
};

const OLD_CHANNELS = {
    state: "bridge:state",
    changed: "bridge:changed",
    syncState: "bridge:sync-state",
    walkFocused: "bridge:walk-focused",
    proposeFromRequest: "bridge:propose-from-request",
    stopProposal: "bridge:stop-proposal",
    stageAttachment: "bridge:stage-attachment",
    searchFiles: "bridge:search-files",
    readRequestRepository: "bridge:read-request-repository",
    approveDispatch: "bridge:approve-dispatch",
    listBranches: "bridge:list-branches",
    redispatchJob: "bridge:redispatch-job",
    killDrone: "bridge:kill-drone",
    killJob: "bridge:kill-job",
    parkJob: "bridge:park-job",
    resumeJob: "bridge:resume-job",
    fixMain: "bridge:fix-main",
    killProcess: "bridge:kill-process",
    killProcesses: "bridge:kill-processes",
    pilotTask: "bridge:pilot-task",
    restartTask: "bridge:restart-task",
    editTask: "bridge:edit-task",
    movePlan: "bridge:move-plan",
    approveWave: "bridge:approve-wave",
    editJob: "bridge:edit-job",
    setLandingTarget: "bridge:set-landing-target",
    toProposer: "bridge:to-proposer",
    clearTerminalJobs: "bridge:clear-terminal-jobs",
    forgetTerminalJobs: "bridge:forget-terminal-jobs",
    reclaimWorktree: "bridge:reclaim-worktree",
    changeSlotPool: "bridge:change-slot-pool",
    rescueSlot: "bridge:rescue-slot",
    deleteBranch: "bridge:delete-branch",
    forgetJob: "bridge:forget-job",
    redirectDrone: "bridge:redirect-drone",
    answerQuestion: "bridge:answer-question",
    answerCommand: "bridge:answer-command",
    askCommander: "bridge:ask-commander",
    answerCommanderCard: "bridge:answer-commander-card",
    answerHelmCall: "bridge:answer-helm-call",
    explainCommand: "bridge:explain-command",
    setWhenBlocked: "bridge:set-when-blocked",
    answerJudge: "bridge:answer-judge",
    setWhenRefused: "bridge:set-when-refused",
    setModel: "bridge:set-model",
    setReviewModel: "bridge:set-review-model",
    removeAllowedCommand: "bridge:remove-allowed-command",
    restartStep: "bridge:restart-step",
    overrideVerdict: "bridge:override-verdict",
    rerunGate: "bridge:rerun-gate",
    rerunChecks: "bridge:rerun-checks",
    retryCheck: "bridge:retry-check",
    skipCheck: "bridge:skip-check",
    skipChecks: "bridge:skip-checks",
    showAgain: "bridge:show-again",
    raiseCostCap: "bridge:raise-cost-cap",
    raiseTurnCap: "bridge:raise-turn-cap",
    savePreference: "bridge:save-preference",
    saveSettings: "bridge:save-settings",
    openSettingsFile: "bridge:open-settings-file",
    validateMod: "bridge:validate-mod",
    setModEnabled: "bridge:set-mod-enabled",
    promoteMod: "bridge:promote-mod",
    phone: "bridge:phone",
    fileReport: "bridge:file-report",
    addTask: "bridge:add-task",
    dropTask: "bridge:drop-task",
    watchJob: "bridge:watch-job",
    observeJob: "bridge:observe-job",
    followCheckOutput: "bridge:follow-check-output",
    followLandCheck: "bridge:follow-land-check",
    readHistory: "bridge:read-history",
    readEvidence: "bridge:read-evidence",
    readResources: "bridge:read-resources",
    followActive: "bridge:follow-active",
    watchPulse: "bridge:watch-pulse",
    watchRunSheet: "bridge:watch-run-sheet",
    observeRun: "bridge:observe-run",
    startRun: "bridge:start-run",
    stopRun: "bridge:stop-run",
    undoRun: "bridge:undo-run",
    listRuns: "bridge:list-runs",
    getRunOutput: "bridge:get-run-output",
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
    readManifestSpend: "bridge:read-manifest-spend",
    readManifestChecks: "bridge:read-manifest-checks",
    readRepositoryScan: "bridge:read-repository-scan",
    readManifestProposals: "bridge:read-manifest-proposals",
    editManifestProposal: "bridge:edit-manifest-proposal",
    writeManifestProposal: "bridge:write-manifest-proposal",
    listRepositoryAllowedCommands: "bridge:list-repository-allowed-commands",
    removeRepositoryAllowedCommand: "bridge:remove-repository-allowed-command",
    readKitInventory: "bridge:read-kit-inventory",
    removeKitAllowedCommand: "bridge:remove-kit-allowed-command",
    listKitServers: "bridge:list-kit-servers",
    addKitServer: "bridge:add-kit-server",
    forgetKitServer: "bridge:forget-kit-server",
    setKitServerReach: "bridge:set-kit-server-reach",
    setManifestServerReach: "bridge:set-manifest-server-reach",
    readWorkflows: "bridge:read-workflows",
    readWorkflowDefinition: "bridge:read-workflow-definition",
    saveWorkflow: "bridge:save-workflow",
    readTriggers: "bridge:read-triggers",
    readAlerts: "bridge:read-alerts",
    readTrigger: "bridge:read-trigger",
    saveTrigger: "bridge:save-trigger",
    removeTrigger: "bridge:remove-trigger",
    chooseTriggerFix: "bridge:choose-trigger-fix",
    rerunTrigger: "bridge:rerun-trigger",
    skipTrigger: "bridge:skip-trigger",
    addJobStep: "bridge:add-job-step",
    removeJobStep: "bridge:remove-job-step",
    editJobStep: "bridge:edit-job-step",
    readRepairDiff: "bridge:read-repair-diff",
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
    examineJob: "bridge:examine-job",
    readDiff: "bridge:read-diff",
    readRemarks: "bridge:read-remarks",
    readCheckOutput: "bridge:read-check-output",
    readSessionCheckOutput: "bridge:read-session-check-output",
    stopSessionCheck: "bridge:stop-session-check",
    readBrief: "bridge:read-brief",
    readRetro: "bridge:read-retro",
    readLessons: "bridge:read-lessons",
    agreeLesson: "bridge:agree-lesson",
    disagreeLesson: "bridge:disagree-lesson",
    startLessonSession: "bridge:start-lesson-session",
    reviewLessons: "bridge:review-lessons",
    askLesson: "bridge:ask-lesson",
    readFrame: "bridge:read-frame",
    readReports: "bridge:read-reports",
    readHeld: "bridge:read-held",
    approveReview: "bridge:approve-review",
    mergePullRequest: "bridge:merge-pull-request",
    autoMergePullRequest: "bridge:auto-merge-pull-request",
    rerunFailedChecks: "bridge:rerun-failed-checks",
    investigateFailedChecks: "bridge:investigate-failed-checks",
    queueAfterFinding: "bridge:queue-after-finding",
    fileFindingIssue: "bridge:file-finding-issue",
    openFindingIssue: "bridge:open-finding-issue",
    requestChanges: "bridge:request-changes",
    removeWalkNote: "bridge:remove-walk-note",
    rejectWork: "bridge:reject-work",
    takeUpRemarks: "bridge:take-up-remarks",
    dismissFinding: "bridge:dismiss-finding",
    openArtifact: "bridge:open-artifact",
    openInTerminal: "bridge:open-in-terminal",
    openPullRequest: "bridge:open-pull-request",
    openRemarkLink: "bridge:open-remark-link",
    summoned: "bridge:summoned",
    history: "bridge:history",
    readComposing: "bridge:read-composing",
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
    walkPanelOpenInBridge: "bridge:walk-panel-open-in-bridge",
    tap: "bridge:tap",
    startSession: "bridge:start-session",
    pilotJob: "bridge:pilot-job",
    forkSession: "bridge:fork-session",
    forkSessionAt: "bridge:fork-session-at",
    exitPilot: "bridge:exit-pilot",
    sendSessionMessage: "bridge:send-session-message",
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
    getRepositoryAreas: "bridge:get-repository-areas",
    getSlotTouches: "bridge:get-slot-touches",
    getFleetTimeline: "bridge:get-fleet-timeline",
    timelineAppended: "bridge:timeline-appended",
    getSlotHistory: "bridge:get-slot-history",
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
    setSessionWorkflow: "bridge:set-session-workflow",
    releaseSessionHolds: "bridge:release-session-holds",
    handToSession: "bridge:hand-to-session",
    moveSession: "bridge:move-session",
    watchSession: "bridge:watch-session",
    readSessionFile: "bridge:read-session-file",
    readSessionSubagent: "bridge:read-session-subagent",
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
