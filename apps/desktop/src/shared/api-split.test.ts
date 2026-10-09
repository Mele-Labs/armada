// The old, monolithic shapes of BridgeApi, BridgeState, NOTHING_YET and CHANNELS, frozen as they
// were before the API split (comments stripped). The composed names must stay assignable both ways.
// Kept, not deleted: a slice-by-slice equality cannot be asserted against anything but the whole.

import { describe, expect, it } from "vitest";
import type { PhoneAnswer, PhoneRequest } from "@armada/settings/api";
import type {
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
  SaveLimits,
  SavePreference,
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
  HelmThread,
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
import type {
  DriftsRead,
  HealthRead,
} from "@armada/screens/src/overview-reads";
import type {
  Outstanding,
} from "@armada/screens/src/outstanding";
import type { AnswerSessionAsk, AnswerWaiting, PilotOutcome, PullRequestState, RenameSession, ReviewDispatched, SendSessionMessage, SessionRow, SessionSubagent, TuneSession } from "@armada/protocol";
import type { PilotExit, PullRequestPress, SessionActed, SessionsRead } from "./api/sessions";
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
    drifts: DriftsRead;
    questions: Outstanding[];
    helm: HelmThread;
    studios: StudiosRead;
    studio: StudioRead;
    sessions: SessionsRead;
    sessionThreads: Record<string, SessionRow[]>;
};

type OldBridgeApi = {
    state: () => Promise<OldBridgeState>;
    subscribe: (onState: (state: OldBridgeState) => void) => () => void;
    onWalkFocus: (onFocus: (focused: boolean) => void) => () => void;
    proposeFromRequest: (request: string, attachments: readonly StagedAttachment[], repository?: string | null) => Promise<Proposed>;
    stopProposal: () => Promise<Outcome>;
    stageAttachment: (bytes: ArrayBuffer, filename: string, mimeType: string) => Promise<{
        path: string;
    }>;
    searchFiles: (query: string) => Promise<string[]>;
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
    answerQuestion: (jobId: string, questionId: string, chose: string) => Promise<Outcome>;
    answerCommand: (jobId: string, call: string, answer: CommandAnswer, note?: string, rule?: string, scope?: AlwaysAllowScope) => Promise<Outcome>;
    answerHelmCall: (call: string, answer: HelmCallAnswer, note?: string) => Promise<Outcome>;
    explainCommand: (jobId: string, callId: string) => Promise<CommandExplainedRead>;
    setWhenBlocked: (jobId: string, whenBlocked: WhenBlocked) => Promise<Outcome>;
    answerJudge: (jobId: string, askedAt: string, answer: JudgeAnswer, note?: string) => Promise<Outcome>;
    setWhenRefused: (jobId: string, whenRefused: WhenRefused) => Promise<Outcome>;
    setModel: (jobId: string, model: string | null) => Promise<Outcome>;
    setReviewModel: (jobId: string, model: string | null) => Promise<Outcome>;
    removeAllowedCommand: (jobId: string, run: string) => Promise<Outcome>;
    restartStep: (jobId: string, note?: string) => Promise<Outcome>;
    overrideVerdict: (jobId: string, reason: string) => Promise<Outcome>;
    rerunGate: (jobId: string) => Promise<Outcome>;
    rerunChecks: (jobId: string) => Promise<Outcome>;
    showAgain: (jobId: string, spec?: string) => Promise<Outcome>;
    raiseCostCap: (jobId: string, costCapMicros: number) => Promise<Outcome>;
    raiseTurnCap: (jobId: string, turnCap: number) => Promise<Outcome>;
    saveLimits: (values: SaveLimits) => Promise<Outcome>;
    savePreference: (save: SavePreference) => Promise<Outcome>;
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
    readBrief: (jobId: string, name: string) => Promise<BriefRead>;
    readRetro: (subject: RetroSubject) => Promise<RetroRead>;
    readLessons: (state: "open" | "accepted") => Promise<LessonsRead>;
    agreeLesson: (lessonId: string) => Promise<LessonAnswer>;
    disagreeLesson: (lessonId: string) => Promise<LessonAnswer>;
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
    openPullRequest: (jobId: string) => Promise<Followed>;
    openRemarkLink: (jobId: string, remarkId: string) => Promise<Followed>;
    onSummoned: (onGo: (to: Summons) => void) => () => void;
    onHistory: (onStep: (step: HistoryStep) => void) => () => void;
    askHelm: (text: string, context?: HelmContext) => Promise<Outcome>;
    helmDebugInfo: () => Promise<HelmDebugRead>;
    startHelmFresh: () => Promise<Outcome>;
    pointHelm: (manifestId: string) => Promise<void>;
    tap: (pattern: Pattern) => void;
    startSession: (title?: string) => Promise<SessionActed>;
    pilotJob: (jobId: string, outcome: PilotOutcome) => Promise<SessionActed>;
    exitPilot: (jobId: string, exit: PilotExit, note?: string) => Promise<Outcome>;
    sendSessionMessage: (send: SendSessionMessage) => Promise<SessionActed>;
    answerSessionAsk: (answer: AnswerSessionAsk) => Promise<SessionActed>;
    answerWaiting: (answer: AnswerWaiting) => Promise<SessionActed>;
    tuneSession: (tune: TuneSession) => Promise<SessionActed>;
    renameSession: (rename: RenameSession) => Promise<SessionActed>;
    forkSession: (sessionId: string) => Promise<SessionActed>;
    retroSession: (sessionId: string) => Promise<Outcome>;
    closeSession: (sessionId: string) => Promise<SessionActed>;
    watchSession: (sessionId: string) => Promise<void>;
    readSessionFile: (sessionId: string, file: string) => Promise<FrameRead>;
    readSessionSubagent: (sessionId: string, subagentId: string) => Promise<SessionActed<SessionSubagent>>;
    openSessionFile: (sessionId: string, path: string) => Promise<Followed>;
    openSessionWindow: (sessionId: string, url: string) => Promise<Outcome>;
    readSessionArtifact: (sessionId: string, path: string) => Promise<ArtifactRead>;
    showSessionPage: (sessionId: string, address: string, bounds: PageBounds) => Promise<Followed>;
    moveSessionPage: (bounds: PageBounds) => Promise<void>;
    hideSessionPage: () => Promise<void>;
    onSessionPageEscape: (on: () => void) => () => void;
    pressPullRequest: (sessionId: string, number: number, press: PullRequestPress) => Promise<SessionActed<PullRequestState | ReviewDispatched>>;
};

const OLD_NOTHING_YET: OldBridgeState = {
    connection: { state: "reading" },
    bridge: { auditPath: null, fleetProtocol: null },
    jobs: [],
    unreadable: [],
    capacity: null,
    fleetBuild: null,
    limits: null,
    preferences: { where_things_are_open: false },
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
    drifts: { state: "none" },
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
    walkFocused: "bridge:walk-focused",
    proposeFromRequest: "bridge:propose-from-request",
    stopProposal: "bridge:stop-proposal",
    stageAttachment: "bridge:stage-attachment",
    searchFiles: "bridge:search-files",
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
    showAgain: "bridge:show-again",
    raiseCostCap: "bridge:raise-cost-cap",
    raiseTurnCap: "bridge:raise-turn-cap",
    saveLimits: "bridge:save-limits",
    savePreference: "bridge:save-preference",
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
    readBrief: "bridge:read-brief",
    readRetro: "bridge:read-retro",
    readLessons: "bridge:read-lessons",
    agreeLesson: "bridge:agree-lesson",
    disagreeLesson: "bridge:disagree-lesson",
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
    openPullRequest: "bridge:open-pull-request",
    openRemarkLink: "bridge:open-remark-link",
    summoned: "bridge:summoned",
    history: "bridge:history",
    readComposing: "bridge:read-composing",
    askHelm: "bridge:ask-helm",
    helmDebugInfo: "bridge:helm-debug-info",
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
    tap: "bridge:tap",
    startSession: "bridge:start-session",
    pilotJob: "bridge:pilot-job",
    forkSession: "bridge:fork-session",
    exitPilot: "bridge:exit-pilot",
    sendSessionMessage: "bridge:send-session-message",
    answerSessionAsk: "bridge:answer-session-ask",
    answerWaiting: "bridge:answer-waiting",
    tuneSession: "bridge:tune-session",
    renameSession: "bridge:rename-session",
    retroSession: "bridge:retro-session",
    closeSession: "bridge:close-session",
    watchSession: "bridge:watch-session",
    readSessionFile: "bridge:read-session-file",
    readSessionSubagent: "bridge:read-session-subagent",
    openSessionFile: "bridge:open-session-file",
    openSessionWindow: "bridge:open-session-window",
    readSessionArtifact: "bridge:read-session-artifact",
    showSessionPage: "bridge:show-session-page",
    moveSessionPage: "bridge:move-session-page",
    hideSessionPage: "bridge:hide-session-page",
    sessionPageEscape: "bridge:session-page-escape",
    pressPullRequest: "bridge:press-pull-request",
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
