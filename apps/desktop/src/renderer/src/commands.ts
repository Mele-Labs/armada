// What this app asks the host to do, and what it asks the host to hold open.
//
// **Its own file for the reason `palette.ts` is its own**: this is a subject rather than a line of
// wiring. The subject is the preload boundary — nothing here knows which surface is open or what is
// drawn, and nothing that draws knows how to reach the host.
//
// **The pieces of state here are the ones a command in flight has**: what the last command answered,
// and which Helm call an answer is out on. A command that ends somewhere on screen says so through a
// callback rather than by reaching for state it does not own: a re-read's answer is published by
// `onRead`.

import type { EditManifestProposal, StudioPosition, StudioPromotion, WriteManifestProposal } from "@armada/protocol";
import type { BuildSource, ChangeSlotPool, LandCheckAt, RescueSlot, RetroSubject, SketchToKeep } from "@armada/protocol";
import { useState } from "react";

import type { BridgeState } from "../../shared/bridge";
import type { ActingOnWorkflowCopy, AskingForWorkflowChange, SavingWorkflow, SuggestingWorkflowChange } from "../../shared/workflows";
import type { EditManifest, ProposeToArmada, SaveManifestFile, SendBackWorkflowChange, WorkflowChangeRef } from "@armada/protocol";
import type { AddKitServer, InstallKitPiece, LookAtKitSource, ManifestReach, ReachesDrones, RemoveKitPiece, RestoreKitPiece } from "@armada/protocol";
import type { Outcome } from "@armada/protocol";
import type { HelmCallAnswer, StartCheckoutRun } from "@armada/protocol";
import type { HelmContext } from "@armada/protocol";
import type { StudioCapture, StudioNodeByHand } from "@armada/protocol";
import type { SendSessionMessage } from "@armada/protocol";
import type { DropRunFact, SetRunMemoryPlace } from "@armada/protocol";
import type { AskTurn } from "@armada/screens/src/lessons/review-wire";

/* The host calls the screens make, bound once at module scope.
 *
 * **Stable on purpose.** Three of these are depended on by effects, and a
 * lambda rebuilt every render would open and close a read on a loop that feeds
 * itself — the reads publish state. Module scope is the cheapest guarantee
 * there is, and `window.armada` is itself fixed for the life of the window.
 *
 * They exist at all because a screen may not reach for the preload. A screen
 * that did could not be rendered outside the app, which is the whole of why the
 * screens are a layer. */
export const readHeld = (want: boolean): void => void window.armada.readHeld(want);
// The Studios surface — #1287. The two reads are held by effects, so they are stable here too.
export const watchStudios = (manifestId: string | readonly string[] | null): void => void window.armada.watchStudios(manifestId);
export const watchStudio = (studioId: string | null): void => void window.armada.watchStudio(studioId);
export const createStudio = (manifestId: string) => window.armada.createStudio(manifestId);
// Named, and added to, by a person — #1364. Neither goes through Helm.
export const renameStudio = (studioId: string, name: string) => window.armada.renameStudio(studioId, name);
export const addStudioNode = (
  studioId: string,
  node: StudioNodeByHand,
  position: { x: number; y: number },
  within: string | null,
) => window.armada.addStudioNode(studioId, node, position, within);
export const pathOfFile = (file: File) => window.armada.pathOfFile(file);
export const addStudioPicture = (studioId: string, bytes: Uint8Array, position: { x: number; y: number }) =>
  window.armada.addStudioPicture(studioId, bytes, position);
export const addStudioSketch = (
  studioId: string,
  drawing: SketchToKeep,
  position: { x: number; y: number },
  within: string | null,
) => window.armada.addStudioSketch(studioId, drawing, position, within);
export const saveStudioSketch = (studioId: string, nodeId: string, drawing: SketchToKeep) =>
  window.armada.saveStudioSketch(studioId, nodeId, drawing);
export const moveStudioNode = (
  studioId: string,
  nodeId: string,
  position: { x: number; y: number },
  within: string | null,
) => window.armada.moveStudioNode(studioId, nodeId, position, within);
export const removeStudioNodes = (studioId: string, nodeIds: readonly string[]) =>
  window.armada.removeStudioNodes(studioId, nodeIds);
export const decideStudioEdge = (studioId: string, edgeId: string, accepted: boolean) =>
  window.armada.decideStudioEdge(studioId, edgeId, accepted);
// Studio capture — #1290. What a person pointed at; the frame is main's to take.
export const captureStudioNote = (studioId: string, said: string, capture: StudioCapture) =>
  window.armada.captureStudioNote(studioId, said, capture);
/** The picture one Note kept — #1352. The bytes become a `blob:` this window owns and revokes. */
export const readStudioFrame = (studioId: string, nodeId: string, picture?: string) =>
  window.armada.readStudioFrame(studioId, nodeId, picture);
export const promoteOnStudio = (studioId: string, promotion: StudioPromotion) =>
  window.armada.promoteOnStudio(studioId, promotion);
export const startStudioRun = (studioId: string, name: string, position: StudioPosition) =>
  window.armada.startStudioRun(studioId, name, position);
export const startStudioServer = (studioId: string, name: string, position: StudioPosition) =>
  window.armada.startStudioServer(studioId, name, position);
/** Open what a Studio node points at. Main reads the address; the renderer sends none. #1406. */
export const openStudioNode = (studioId: string, nodeId: string) =>
  window.armada.openStudioNode(studioId, nodeId);
export const changeSlotPool = (manifestId: string, change: ChangeSlotPool) =>
  window.armada.changeSlotPool(manifestId, change);
export const rescueSlot = (manifestId: string, rescue: RescueSlot) =>
  window.armada.rescueSlot(manifestId, rescue);
/**
 * What a tile of Cleanup's grid acts with: the pool reshaped or a slot rescued. Each receipt is
 * answered to the press that asked for it, because a published notice would outlive the screen it
 * was made on.
 */
export const slotActs = {
  onChangeSlotPool: changeSlotPool,
  onRescueSlot: rescueSlot,
};
/**
 * What a bay's pane on the Cockpit acts with: the tile panel's acts, under the names `BayManage` reads. A constant, so the deck
 * is not drawn again for a new object each time the window is.
 */
export const bayActs = {
  onChange: slotActs.onChangeSlotPool,
  onRescue: slotActs.onRescueSlot,
};
export const readSessionCheckOutput = (run: number) => window.armada.readSessionCheckOutput(run);
export const stopSessionCheck = (run: number, force: boolean) => window.armada.stopSessionCheck(run, force);
/** A Session's retro and the Lessons listing, read on open and on focus — `docs/concepts/retro.md`. */
export const readRetro = (subject: RetroSubject) => window.armada.readRetro(subject);
export const readLessons = (state: "open" | "accepted") => window.armada.readLessons(state);
/** The owner's answer to one retro item. */
export const agreeLesson = (lessonId: string) => window.armada.agreeLesson(lessonId);
export const disagreeLesson = (lessonId: string) => window.armada.disagreeLesson(lessonId);
export const startLessonSession = (lessonId: string) => window.armada.startLessonSession(lessonId);
/** Setup's Set up with a session: the two Session calls it makes, on the repository being set up. */
export const setupSessionFleet = { startSession: (title?: string, root?: string) => window.armada.startSession(title, root), sendSessionMessage: (send: SendSessionMessage) => window.armada.sendSessionMessage(send) };
export const reviewLessons = () => window.armada.reviewLessons();
export const askLesson = (lessonId: string, question: string, history: AskTurn[]) =>
  window.armada.askLesson(lessonId, question, history);
/** The Logbook, read on open, and the owner's three acts on an entry — `docs/concepts/logbook.md`. */
export const readLogbook = () => window.armada.readLogbook();
export const editLogbookEntry = (id: string, text: string) => window.armada.editLogbookEntry(id, text);
export const lockLogbookEntry = (id: string, locked: boolean) => window.armada.lockLogbookEntry(id, locked);
export const forgetLogbookEntry = (id: string) => window.armada.forgetLogbookEntry(id);
export const tidyLogbook = () => window.armada.tidyLogbook();
export const followCheckOutput = (jobId: string | null, kept: string | null): void =>
  void window.armada.followCheckOutput(jobId, kept);
export const followLandCheck = (at: LandCheckAt | null): void => void window.armada.followLandCheck(at);


// The same rehearsal in the main checkout — the Manifest surface. Held open
// by the app rather than by the screen, because the palette lists off the same
// reading and a read the screen owned would leave those rows missing
// everywhere but on that surface.
export const watchCheckoutRunSheet = (want: boolean): void =>
  void window.armada.watchCheckoutRunSheet(want);
export const observeCheckoutRun = (runId: string | null): void =>
  void window.armada.observeCheckoutRun(runId);
export const startCheckoutRun = (body: StartCheckoutRun) => window.armada.startCheckoutRun(body);
export const stopCheckoutRun = (runId: string) => window.armada.stopCheckoutRun(runId);
export const undoCheckoutRun = (runId: string) => window.armada.undoCheckoutRun(runId);
export const listCheckoutRuns = () => window.armada.listCheckoutRuns();
export const getCheckoutRunOutput = (runId: string) => window.armada.getCheckoutRunOutput(runId);
export const getCheckoutRunDiff = (runId: string) => window.armada.getCheckoutRunDiff(runId);
// Journey 9's *Verify*. Drift is held open by the surface; Verify is only ever
// pressed, so nothing here starts one on a read.
export const watchManifestDrift = (want: boolean): void =>
  void window.armada.watchManifestDrift(want);
// Overview's health and per-repository drift. Held open by the surface, `holding.ts`'s shape.
export const watchOverview = (want: boolean): void => void window.armada.watchOverview(want);
export const startCheckoutVerify = (workspace?: string) => window.armada.startCheckoutVerify(workspace);
export const pickRepository = (root: string | null): void => void window.armada.pickRepository(root);
// Helm's conversation — #944. The reply, and the thread it joins, arrive on
// `BridgeState.helm`. `context` names where the person is — #1075.
export const askHelm = (text: string, context?: HelmContext) => window.armada.askHelm(text, context);
/** The session as one quotable record, read once when a person opens it — #1367. */
export const helmDebugInfo = () => window.armada.helmDebugInfo();
export const startHelmFresh = () => window.armada.startHelmFresh();
export const pointHelm = (manifestId: string): void => void window.armada.pointHelm(manifestId);
// Locate: a folder from the OS dialog, and a repository added or cloned. The window picks what it located.
export const chooseFolder = () => window.armada.chooseFolder();
export const resolveFolder = (path: string) => window.armada.resolveFolder(path);
export const addRepository = (path: string) => window.armada.addRepository(path);
export const cloneRepository = (url: string, parent: string) => window.armada.cloneRepository(url, parent);
export const createProject = (parent: string, name: string) => window.armada.createProject(parent, name);
// Starting out: the folders already worked in with another agent CLI, how a repository runs and where that is kept, the mod's install.
export const listImportFolders = () => window.armada.listImportFolders();
export const getImportRuns = (path: string) => window.armada.getImportRuns(path);
/** Fleet installs only for a press: call this from one. */
export const installAgentMod = () => window.armada.installAgentMod();
export const getRunMemory = (repository: string) => window.armada.getRunMemory(repository);
export const getRunMemoryPlace = (repository: string) => window.armada.getRunMemoryPlace(repository);
export const setRunMemoryPlace = (body: SetRunMemoryPlace) => window.armada.setRunMemoryPlace(body);
export const dropRunFact = (body: DropRunFact) => window.armada.dropRunFact(body);
// The Manifest file. Held by the app, like the sheet above, so an unsaved edit
// outlives the surface it was made on.
export const readManifestFile = () => window.armada.readManifestFile();
export const saveManifestFile = (body: SaveManifestFile) => window.armada.saveManifestFile(body);
export const editManifest = (body: EditManifest) => window.armada.editManifest(body);
// Setup: Scan, the proposals, one edit, Write.
export const readRepositoryScan = () => window.armada.readRepositoryScan();
export const readManifestProposals = () => window.armada.readManifestProposals();
export const editManifestProposal = (body: EditManifestProposal) => window.armada.editManifestProposal(body);
export const writeManifestProposal = (body: WriteManifestProposal) => window.armada.writeManifestProposal(body);
// A repository-wide always-allow — Fleet's own table since protocol 13.5.
export const listRepositoryAllowedCommands = () => window.armada.listRepositoryAllowedCommands();
export const removeRepositoryAllowedCommand = (run: string) =>
  window.armada.removeRepositoryAllowedCommand(run);
/** Kit's MCP servers, and the two tiers of reach over each — #1275. */
export const readKitInventory = () => window.armada.readKitInventory();
export const removeKitAllowedCommand = (run: string) => window.armada.removeKitAllowedCommand(run);
/** Add and Remove for a plugin or a skill, and Undo for a skill. */
export const lookAtKitSource = (body: LookAtKitSource) => window.armada.lookAtKitSource(body);
export const installKitPiece = (body: InstallKitPiece) => window.armada.installKitPiece(body);
export const removeKitPiece = (body: RemoveKitPiece) => window.armada.removeKitPiece(body);
export const restoreKitPiece = (body: RestoreKitPiece) => window.armada.restoreKitPiece(body);
export const listKitServers = () => window.armada.listKitServers();
/** The Workflow creator: the list, one definition, and a save. */
export const readWorkflows = () => window.armada.readWorkflows();
export const readWorkflowDefinition = (workflowId: string, source: string, manifestId: string | null) =>
  window.armada.readWorkflowDefinition(workflowId, source, manifestId);
export const saveWorkflow = (saving: SavingWorkflow) => window.armada.saveWorkflow(saving);
/** The Workflows page beside a definition's text: its steps, a copy deleted or reset to Armada's, a nudge marked not a drift. */
export const readWorkflowSteps = (workflowId: string, source: string, manifestId: string | null) =>
  window.armada.readWorkflowSteps(workflowId, source, manifestId);
export const deleteWorkflowCopy = (acting: ActingOnWorkflowCopy) => window.armada.deleteWorkflowCopy(acting);
export const resetWorkflowCopy = (acting: ActingOnWorkflowCopy) => window.armada.resetWorkflowCopy(acting);
export const markNotADrift = (nudge: number) => window.armada.markNotADrift(nudge);
/** The changes an agent drafts for him to approve: typed, suggested, polled while drafting, then approved, dismissed or sent back. */
export const askWorkflowChange = (asking: AskingForWorkflowChange) => window.armada.askWorkflowChange(asking);
export const suggestWorkflowChange = (suggesting: SuggestingWorkflowChange) => window.armada.suggestWorkflowChange(suggesting);
export const readWorkflowChanges = (workflowId: string, source: string, manifestId: string | null) =>
  window.armada.readWorkflowChanges(workflowId, source, manifestId);
export const approveWorkflowChange = (change: number) => window.armada.approveWorkflowChange(change);
export const dismissWorkflowChange = (change: number) => window.armada.dismissWorkflowChange(change);
export const sendBackWorkflowChange = (back: SendBackWorkflowChange) => window.armada.sendBackWorkflowChange(back);
/** A job that builds what a change Armada can't do yet lacks. */
export const buildWorkflowSupport = (building: WorkflowChangeRef) => window.armada.buildWorkflowSupport(building);
/** A Job on Armada's own repository that lands a definition, so it ships to every install. */
export const proposeWorkflowToArmada = (proposing: ProposeToArmada) => window.armada.proposeWorkflowToArmada(proposing);
export const addKitServer = (adding: AddKitServer) => window.armada.addKitServer(adding);
export const forgetKitServer = (name: string) => window.armada.forgetKitServer(name);
export const setKitServerReach = (name: string, drones: ReachesDrones) =>
  window.armada.setKitServerReach(name, drones);
export const setManifestServerReach = (name: string, reach: ManifestReach | null) =>
  window.armada.setManifestServerReach(name, reach);
export const startServer = (name: string) => window.armada.startServer(name);
export const stopServer = (serverId: string) => window.armada.stopServer(serverId);
export const openServerLink = (serverId: string, url: string) =>
  window.armada.openServerLink(serverId, url);
/** A link in a model's text. Main opens `http(s):` only — `main/links.ts`. */
export const openLink = (address: string) => window.armada.openLink(address);
export const restartFleet = () => window.armada.restartFleet();
export const changeFleetBuild = (build: BuildSource, adopt: boolean) => window.armada.changeFleetBuild(build, adopt);

// The capture window on a server Run — #1294. One entry, and everything the
// window then does is its own bar's.
export const openCaptureWindow = (serverId: string, url: string) =>
  window.armada.openCaptureWindow(serverId, url);
export const stageAttachment = (bytes: ArrayBuffer, filename: string, mimeType: string) =>
  window.armada.stageAttachment(bytes, filename, mimeType);
/** Paths under the checkout narrowed against typed text, for the `@` mention popup. */
export const searchFiles = (query: string, manifestId?: string) => window.armada.searchFiles(query, manifestId);

/** What a command needs from the render. */
export type Sending = {
  /** Where a re-read's answer goes: the one place published state is held. */
  onRead: (state: BridgeState) => void;
};

/**
 * Every command the window can send, and the state one in flight has.
 *
 * **Nothing memoises.** These are rebuilt every render, as they were when they
 * lived in `App`, and no effect depends on one — the bound host calls above are
 * the ones effects read, and those are module scope for exactly that reason.
 */
export function useCommands(sending: Sending) {
  const [outcome, setOutcome] = useState<Outcome | null>(null);
  // Which Helm call has an answer in flight, by the call id Fleet minted, so its card's controls wait. #1389.
  const [answeringHelm, setAnsweringHelm] = useState<string | null>(null);

  /**
   * Send main's red to a Session as a message, through the call the Cockpit sends with. **A
   * refusal goes to the pipeline every command failure uses**; an accepted one says nothing, since the
   * Session's ledger shows the message.
   */
  async function sendToSession(sessionId: string, text: string): Promise<void> {
    const answer = await window.armada.sendSessionMessage({ session_id: sessionId, text });
    if (!answer.ok) setOutcome(answer.outcome);
  }

  /** Answer one held Helm call. What draws the card's own controls off while this is out is the call id held here. #1389. */
  async function answerHelmCall(call: string, chose: HelmCallAnswer, note?: string): Promise<Outcome> {
    setAnsweringHelm(call);
    try {
      const answered = await window.armada.answerHelmCall(call, chose, note);
      setOutcome(answered);
      return answered;
    } finally {
      setAnsweringHelm((held) => (held === call ? null : held));
    }
  }

  /**
   * Ask Fleet for current state over the connection Bridge already holds.
   *
   * **It re-reads; it does not reconnect.** The stream keeps the window current
   * on its own, so this is for the case where somebody wants to be sure rather
   * than for a connection that is broken — and dropping a working socket would
   * not fix one that is.
   */
  async function refresh(): Promise<void> {
    sending.onRead(await window.armada.state());
  }

  return {
    outcome,
    setOutcome,
    sendToSession,
    answerHelmCall,
    answeringHelm,
    refresh,
  };
}
