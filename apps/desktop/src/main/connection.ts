// Bridge's one connection to Armada API: WebSocket for events, HTTP for
// queries and commands, held in the one process allowed to hold it.
//
// The renderer never opens a socket and never fetches. It reads what is
// published from here and calls back through the preload.
//
// **Bridge never talks to a Drone.** Everything below names Fleet.
//
// What is here is the state — `current` and `publish` — and the wiring that
// builds everything reaching it. `socket.ts` is the socket's own lifecycle;
// `arrivals.ts` folds one stream message into the state, handed `ArrivalHost`,
// a narrow view of this file rather than a copy of it.

import { identifying, NOTHING_YET } from "../shared/bridge";
import type { StartAfter } from "@armada/protocol";
import type { HelmCarry } from "../shared/api/helm";
import type { SessionActed } from "../shared/api/sessions";
import type { BridgeState, PickedView } from "../shared/bridge";
import type { Connection, HelmContext, HelmDebugRead, Outcome, ThoughtRead, ThoughtTarget } from "@armada/protocol";
import type { CheckOutputRead, LandCheckAt, LessonsRead, RetroRead, RetroSubject } from "@armada/protocol";
import type { AskLessonRead, AskTurn, LessonReviewRead } from "@armada/screens/src/lessons/review-wire";
import { applyArrival, readModels } from "./arrivals";
import type { ArrivalHost } from "./arrivals";
import { Commands } from "./command";
import { CheckReads } from "./check-reads";
import { LessonReads } from "./lesson-reads";
import { FleetBuilds } from "./fleet-build";
import { FollowSocket } from "./following";
import { LandFollowSocket } from "./land-following";
import { HelmConnection } from "./helm";
import { HeldReader } from "./holding";
import { OverviewReads } from "./overview";
import { Questions } from "./questions";
import { RehearsalConnection } from "./rehearsal";
import { ManifestFileCommands } from "./editing";
import { KitCommands } from "./kit";
import { TriggerCommands } from "./triggers";
import { WorkflowCommands } from "./workflows";
import { RepositoryAllowsCommands } from "./repository-allows";
import { RepositoryReads } from "./repositories";
import { Picked, PickedByWindow, repositoryToStart, startOn } from "./picked";
import { startingIdentity } from "./runtime-file";
import { FleetSocket, type BridgeStateFleet } from "./socket";
import { Modding } from "./mods";
import { SessionsHost } from "./sessions";
import { SleepHost } from "./sleep";
import { AccountsHost } from "./accounts";
import { PlanUsageHost } from "./plan-usage";
import { ModelDownshiftHost } from "./model-downshift";
import { ProjectsHost } from "./projects";
import { AgentSignInHost } from "./agent-sign-in";
import { ClashesHost } from "./clashes";
import { TeachingHost } from "./teaching";
import { ReviewsHost } from "./reviews";
import { TerritoryHost } from "./territory";
import { BayHistoryHost } from "./bay-history";
import { LogbookHost } from "./logbook";
import { UsageSummaryHost } from "./usage-summary";
import { FlightControlsHost } from "./flight-controls";
import { OnboardingHost } from "./onboarding";
import { SharingHost } from "./sharing";
import { TimelineHost } from "./timeline";
import { ActFailures } from "./act-failed";
import { IssuesHost } from "./issues";
import { RequestsHost } from "./requests";
import { StudioReads } from "./studios";
import { askPhone } from "./phone-gateway";
import type { PhoneAnswer, PhoneRequest } from "@armada/settings/api";

/** Time is injected, never read: a connection that calls the clock cannot be replayed. */
export type Clock = () => number;

export type Wiring = {
  home: string | undefined;
  publish: (state: BridgeState) => void;
  /** One window's own `repository` and `manifestReading` — `main/index.ts` overlays it onto what
   * that window alone receives, `repositories.ts`'s `PickedView`. */
  publishToWindow: (windowId: number, change: Partial<PickedView>) => void;
  /** Every window open right now — `main/index.ts`'s own `BrowserWindow.getAllWindows()`. */
  windowIds: () => readonly number[];
  now: Clock;
  /** Where `sessions.ts` keeps the threads it holds, so a reopened Job draws at once. None where a test has no disk to use. */
  threadsDir?: string;
  /** `socket.ts`'s two waits, shortened where a test cannot spend minutes. */
  timing?: { graceMs: number; bootMs: number };
};

export class FleetConnection {
  private readonly wiring: Wiring;
  private current: BridgeState = NOTHING_YET;
  /** The socket lifecycle — reading the runtime file, connecting, retrying. */
  private readonly socket: FleetSocket;
  /** Journey 9's run sheet and the servers it starts — see `rehearsal.ts`.
   * One field for both, `commands`' reason: they are one feature. */
  readonly rehearsal: RehearsalConnection;
  /**
   * The Manifest file, repository-wide always-allows and Overview, one of each per window — see
   * `editing.ts`, `repository-allows.ts` and `overview.ts`. **Constructed on first ask, off that
   * window's own `Picked`** — `RepositoryReads.pickedByWindow` — so two windows on two
   * repositories never share one route builder or one Overview scope. `editing` and
   * `repositoryAllows` hold no mutable state of their own, which is what makes one instance per
   * window cheap; `overview` does — its own `open` and `asked` — which is exactly why a window's
   * Helm dock closing must not close another's.
   */
  private readonly windowFacades = new Map<
    number,
    {
      editing: ManifestFileCommands;
      repositoryAllows: RepositoryAllowsCommands;
      kit: KitCommands;
      workflows: WorkflowCommands;
      triggers: TriggerCommands;
      overview: OverviewReads;
    }
  >();
  /** What Fleet serves and which repository each window picked — see `repositories.ts`. */
  readonly repositories: RepositoryReads;
  /** Every question waiting on a person, from every repository — see `questions.ts`. */
  private readonly questions = new Questions({
    current: () => this.current,
    publish: (change) => this.publish(change),
  });
  /** One running Check's log, as it is written — a fourth socket. `following.ts`. */
  private readonly follow: FollowSocket;
  /** One merge line Check's log, running or ended — a socket of its own. `land-following.ts`. */
  private readonly landFollow: LandFollowSocket;
  /** Which repository Helm answers for, and the requests into its Session. `helm.ts`. */
  private readonly helm: HelmConnection;
  /**
   * What Fleet is holding disk for, where a surface asked — see `holding.ts`.
   * **Scoped to nothing**: what is being decided is which of a set to give back.
   */
  private readonly held = new HeldReader((held) => this.publish({ held }));
  /** A repository's Studios and the one open, where the Studios surface asked — `studios.ts`. */
  readonly studios: StudioReads = new StudioReads(
    (change) => this.publish(change),
    () => this.connected()?.port ?? null,
  );
  /** The build Fleet runs on and the restart onto another — `fleet-build.ts`. */
  readonly fleetBuild: FleetBuilds = new FleetBuilds({
    port: () => this.connected()?.port ?? null,
    current: () => this.current,
    publish: (change) => this.publish(change),
  });
  /** The mods on this machine and the acts on one — `mods.ts`. */
  readonly mods: Modding = new Modding(
    (change) => this.publish(change),
    () => this.connected()?.port ?? null,
  );
  /** Every session on the machine and the threads a window opened — `sessions.ts`. */
  readonly sessions: SessionsHost = new SessionsHost(
    (change) => this.publish(change),
    () => this.connected()?.port ?? null,
  );
  /** Sleep mode — `sleep.ts`. */
  readonly sleep: SleepHost = new SleepHost(() => this.connected()?.port ?? null);
  /** The account and its profiles — `accounts.ts`. */
  readonly accounts: AccountsHost = new AccountsHost(() => this.connected()?.port ?? null);
  /** The plan's windows — `plan-usage.ts`. */
  readonly planUsage: PlanUsageHost = new PlanUsageHost(() => this.connected()?.port ?? null);
  /** The default model's step down — `model-downshift.ts`. */
  readonly modelDownshift: ModelDownshiftHost = new ModelDownshiftHost(() => this.connected()?.port ?? null);
  /** A repository's projects — `projects.ts`. */
  readonly projects: ProjectsHost = new ProjectsHost(() => this.connected()?.port ?? null);
  /** The agents' sign-in — `agent-sign-in.ts`. */
  readonly agentSignIn: AgentSignInHost = new AgentSignInHost(() => this.connected()?.port ?? null);
  /** What happened across the Fleet — `timeline.ts`. */
  readonly timeline: TimelineHost = new TimelineHost(() => this.connected()?.port ?? null);
  /** A background act that failed after its reply — `act-failed.ts`. */
  readonly acts: ActFailures = new ActFailures();
  /** Where two agents' changes would conflict, per served repository — `clashes.ts`. */
  readonly clashes: ClashesHost = new ClashesHost(
    () => this.connected()?.port ?? null,
    () => {
      const listed = this.current.holds.repositories;
      return listed === undefined || listed.length === 0 ? null : listed.flatMap((one) => (one.manifest === undefined ? [] : [one.manifest.id]));
    },
  );
  /** What a person taught, read into a proposal and kept — `teaching.ts`. */
  readonly teaching: TeachingHost = new TeachingHost(() => this.connected()?.port ?? null);
  /** The pull requests waiting on a person, and the acts of a review — `reviews.ts`. */
  readonly reviews: ReviewsHost = new ReviewsHost(() => this.connected()?.port ?? null);
  /** What each repository holds and where each agent has changed it — `territory.ts`. */
  readonly territory: TerritoryHost = new TerritoryHost(() => this.connected()?.port ?? null);
  /** Who held one slot and what was checked out in it — `bay-history.ts`. */
  readonly bayHistory: BayHistoryHost = new BayHistoryHost(() => this.connected()?.port ?? null);
  /** What agents learned, and the owner's acts on an entry — `logbook.ts`. */
  readonly logbook: LogbookHost = new LogbookHost(() => this.connected()?.port ?? null);
  /** What the last seven days cost, for the Usage page — `usage-summary.ts`. */
  readonly usageSummary: UsageSummaryHost = new UsageSummaryHost(() => this.connected()?.port ?? null);
  /** The Cockpit's flight controls, and a shell line run for a Job — `flight-controls.ts`. */
  readonly flightControls: FlightControlsHost = new FlightControlsHost(() => this.connected()?.port ?? null);
  /** Past sessions to import, how a repository runs and where that is kept, and the mod's install — `onboarding.ts`. */
  readonly onboarding: OnboardingHost = new OnboardingHost(() => this.connected()?.port ?? null);
  /** A piece committed to a Job's branch, and a package made, read and added — `sharing.ts`. */
  readonly sharing: SharingHost = new SharingHost(() => this.connected()?.port ?? null);
  /** A repository's open issues — `issues.ts`. #91. */
  readonly issues: IssuesHost = new IssuesHost(() => this.connected()?.port ?? null);
  /** A repository's request collections and environments, a save of one, and a send — `requests.ts`. */
  readonly requests: RequestsHost = new RequestsHost(() => this.connected()?.port ?? null);
  /**
   * Every act no one surface owns — see `command.ts`. Reached through this rather than re-exported
   * one method at a time: a delegator carries no reasoning.
   */
  readonly commands: Commands;
  /** One Check's output, and stopping a Session's Check — `check-reads.ts`. */
  private readonly checkReads: CheckReads;
  /** A retro, the Lessons listing and the guided review — `lesson-reads.ts`. */
  private readonly lessonReads: LessonReads;
  /** `arrivals.ts`'s switch, and the exact slice of this object it may reach. */
  private readonly arrivalHost: ArrivalHost;

  constructor(wiring: Wiring) {
    this.wiring = wiring;
    // Resolved once, from the home main can see. A failure that cannot say
    // where its log is is half a failure.
    this.current = { ...NOTHING_YET, bridge: startingIdentity(wiring.home) };
    if (wiring.threadsDir !== undefined) this.sessions.keepIn(wiring.threadsDir);
    this.socket = new FleetSocket({
      home: wiring.home,
      now: wiring.now,
      ...(wiring.timing === undefined ? {} : { timing: wiring.timing }),
      settle: (connection) => this.settle(connection),
      arrived: (text, fleet) => this.arrived(text, fleet),
    });
    this.follow = new FollowSocket((followed) => this.publish({ followed }));
    this.landFollow = new LandFollowSocket((landFollowed) => this.publish({ landFollowed }));
    const port = (): number | null => this.connected()?.port ?? null;
    this.helm = new HelmConnection({ publish: (change) => this.publish(change), port, folded: (record) => this.sessions.changed(record) });
    const publish = (change: Partial<BridgeState>) => this.publish(change);
    // The one registry every per-window pick is minted into — `rehearsal` and `repositories`
    // both read it, so a window's Setup, Verify and rail all name the same `Picked`.
    const pickedByWindow = new PickedByWindow();
    this.rehearsal = new RehearsalConnection({ publish, publishToWindow: wiring.publishToWindow, port, pickedByWindow });
    const holds = () => this.current.holds;
    this.repositories = new RepositoryReads({
      pickedByWindow,
      publish,
      publishToWindow: wiring.publishToWindow,
      windowIds: wiring.windowIds,
      holds,
      rehearsal: this.rehearsal,
      overviewAgain: (at) => this.overviewAgainForEveryWindow(at),
      // Helm follows whichever window's pick most recently moved — one
      // conversation per repository, not one per window, so there is one
      // target to keep current rather than one per `windowFacades` entry.
      onPicked: (root) => this.helm.onPicked(root),
      port,
    });
    this.commands = new Commands({
      port,
      // Never held with a real listing: `searchFiles`' own default, exercised only where a caller
      // supplies no `picked` — every real one does.
      picked: new Picked(),
      publish: (change) => this.publish(change),
    });
    this.checkReads = new CheckReads(port);
    this.lessonReads = new LessonReads(port);
    // The exact slice of this object `arrivals.ts`'s switch may reach — built
    // once, after everything it names, so the switch never touches a private
    // field directly. See the module doc.
    this.arrivalHost = {
      current: () => this.current,
      now: () => this.wiring.now(),
      repositories: this.repositories,
      rehearsal: this.rehearsal,
      overviewAgain: (at) => this.overviewAgainForEveryWindow(at),
      questions: this.questions,
      helm: this.helm,
      studios: this.studios,
      mods: this.mods,
      sessions: this.sessions,
      held: this.held,
      sleep: this.sleep,
      planUsage: this.planUsage,
      modelDownshift: this.modelDownshift,
      projects: this.projects,
      agentSignIn: this.agentSignIn,
      timeline: this.timeline,
      acts: this.acts,
      reviews: this.reviews,
      // Autopilot's changes, which main/index.ts passes to every window.
      issues: this.issues,
      fleetBuild: this.fleetBuild,
      socket: this.socket,
      publish: (change) => this.publish(change),
      settle: (connection) => this.settle(connection),
    };
  }

  /** This window's own Manifest file commands — see `windowFacades`. */
  editingFor(windowId: number): ManifestFileCommands {
    return this.facadesFor(windowId).editing;
  }

  /** This window's own repository-wide always-allows — see `windowFacades`. */
  repositoryAllowsFor(windowId: number): RepositoryAllowsCommands {
    return this.facadesFor(windowId).repositoryAllows;
  }

  /** This window's own Kit acts — see `windowFacades`. #1275. */
  kitFor(windowId: number): KitCommands {
    return this.facadesFor(windowId).kit;
  }

  /** This window's own Workflow creator reads and saves — see `windowFacades`. */
  workflowsFor(windowId: number): WorkflowCommands {
    return this.facadesFor(windowId).workflows;
  }

  /** This window's own Trigger reads and saves — see `windowFacades`. */
  triggersFor(windowId: number): TriggerCommands {
    return this.facadesFor(windowId).triggers;
  }

  /** This window's own Overview reads — see `windowFacades`. */
  overviewFor(windowId: number): OverviewReads {
    return this.facadesFor(windowId).overview;
  }

  private facadesFor(
    windowId: number,
  ): {
    editing: ManifestFileCommands;
    repositoryAllows: RepositoryAllowsCommands;
    kit: KitCommands;
    workflows: WorkflowCommands;
    triggers: TriggerCommands;
    overview: OverviewReads;
  } {
    let found = this.windowFacades.get(windowId);
    if (found === undefined) {
      const picked = this.repositories.pickedByWindow.of(windowId);
      const port = (): number | null => this.connected()?.port ?? null;
      found = {
        editing: new ManifestFileCommands(port, picked, (at) => this.repositories.readHoldings(at)),
        repositoryAllows: new RepositoryAllowsCommands(port, picked),
        kit: new KitCommands(port, picked),
        workflows: new WorkflowCommands(port, picked),
        triggers: new TriggerCommands(port, picked),
        overview: new OverviewReads({
          publish: (change) => this.wiring.publishToWindow(windowId, change),
          port,
        }),
      };
      this.windowFacades.set(windowId, found);
    }
    return found;
  }

  /** Every open window's own Overview read again, off its own pick — `repositories.ts`'s `overviewAgain`. */
  private async overviewAgainForEveryWindow(port: number): Promise<void> {
    await Promise.all(this.wiring.windowIds().map((windowId) => this.facadesFor(windowId).overview.again(port)));
  }

  /** The window closed. Its own pick and every per-window facade go with it. */
  dropWindow(windowId: number): void {
    this.windowFacades.get(windowId)?.overview.close();
    this.windowFacades.delete(windowId);
    this.rehearsal.dropWindow(windowId);
    this.repositories.pickedByWindow.drop(windowId);
  }

  /**
   * What Bridge holds, brought current first. Also the bar's Refresh.
   *
   * **A window reload is not a resync**: main is the client and its connection
   * never dropped, so anything main missed stayed missing however many reloads
   * later. One round trip here makes a fresh reader current.
   */
  async state(): Promise<BridgeState> {
    const fleet = this.connected();
    if (fleet !== null) {
      await this.repositories.readHoldings(fleet.port);
      // Every Helm call waiting on a person.
      await this.questions.readHelmCalls(fleet.port);
      // What is held changes when a slot is leased or released and when a sweep runs. A refresh is
      // the cheapest way to be sure about it.
      await this.held.again(fleet.port);
      // A Studio moves when a person or Helm writes to it, and each write is on the stream — so
      // this is for a window that reloaded, not for a gap.
      await this.studios.again();
      await this.sessions.again(fleet.port);
    }
    return this.current;
  }

  /** Read the runtime file, verify the pid, connect. That order, always. */
  start(): void {
    this.socket.start();
  }

  stop(): void {
    this.socket.stop();
    this.rehearsal.close();
    this.follow.close();
    this.landFollow.close();
    this.held.close();
    this.studios.close();
    this.sessions.close();
    this.fleetBuild.close();
    for (const facades of this.windowFacades.values()) facades.overview.close();
    this.helm.close();
  }

  // ------------------------------------------------------------------- Helm
  /** Say something to Helm, about whichever repository it currently answers for. */
  askHelm(text: string, context?: HelmContext, carry?: HelmCarry): Promise<Outcome> {
    return this.helm.askHelm(text, context, carry);
  }

  /** The Helm session Bridge is pointed at, as one quotable record — #1367. */
  helmDebugInfo(): Promise<HelmDebugRead> {
    return this.helm.debugInfo();
  }

  /** What Helm makes of words typed into its box: a question for itself, or a thought for other work. */
  readThought(text: string): Promise<ThoughtRead> {
    return this.helm.readThought(text);
  }

  /** The words reached a Session: Fleet keeps a row for them in `manifestId`'s thread. */
  recordThoughtSent(manifestId: string, text: string, target: ThoughtTarget, carry?: HelmCarry): Promise<Outcome> {
    return this.helm.recordThoughtSent(manifestId, text, target, carry);
  }

  /** End the Helm Session, so the next message starts a new one. */
  startHelmFresh(): Promise<Outcome> {
    return this.helm.startFresh();
  }

  /** "Discuss with Helm" on a card, or the dock's own switch. The rail's pick does not move. */
  pointHelm(manifestId: string): void {
    this.helm.point(manifestId);
  }

  // --------------------------------------------------------------- arrivals
  /** One stream message, folded into the state. See `arrivals.ts`. */
  private arrived(text: string, fleet: BridgeStateFleet): void {
    applyArrival(this.arrivalHost, text, fleet);
  }

  /** One running Check's log, as it is written, or `null` to stop. Opened by a press. */
  followCheckOutput(jobId: string | null, kept: string | null): void {
    this.follow.open(this.connected()?.port ?? null, jobId, kept);
  }

  /** One merge line Check's log, running or ended, or `null` to stop. Opened by a press. */
  followLandCheck(at: LandCheckAt | null): void {
    this.landFollow.open(this.connected()?.port ?? null, at);
  }

  // ------------------------------------------------------- reads that answer the caller
  // `check-reads.ts` and `lesson-reads.ts`, reached through this rather than re-exported one
  // method at a time. The renderer and `index.ts` call these exact names.

  async readSessionCheckOutput(run: number): Promise<CheckOutputRead> {
    return await this.checkReads.readSessionCheckOutput(run);
  }

  async stopSessionCheck(run: number, force: boolean): Promise<Outcome> {
    return await this.checkReads.stopSessionCheck(run, force);
  }

  async readRetro(subject: RetroSubject): Promise<RetroRead> {
    return await this.lessonReads.readRetro(subject);
  }

  async readLessons(picked: Picked, state: "open" | "accepted"): Promise<LessonsRead> {
    return await this.lessonReads.readLessons(picked, state);
  }

  async reviewLessons(picked: Picked): Promise<LessonReviewRead> {
    return await this.lessonReads.reviewLessons(picked);
  }

  async askLesson(lessonId: string, question: string, history: AskTurn[]): Promise<AskLessonRead> {
    return await this.lessonReads.askLesson(lessonId, question, history);
  }

  /** Read what Fleet is holding disk for, or drop it. */
  async readHeld(want: boolean): Promise<void> {
    await this.held.want(this.connected()?.port ?? null, want);
  }

  /** Ask Fleet to ask the harness for its models again, into `holds.models`. */
  async askModels(): Promise<void> {
    const port = this.connected()?.port ?? null;
    if (port !== null) await readModels(port, this.arrivalHost, true);
  }

  /** Read it again, after something that changes what is held. A no-op where nobody has the surface open. */
  async rereadHeld(): Promise<void> {
    const port = this.connected()?.port ?? null;
    if (port !== null) await this.held.again(port);
  }

  /**
   * A blank session on the repository the calling window picked, as Dispatch names one. **On All, the
   * one repository Fleet serves** where there is exactly one, and a refusal in words where there are
   * several: a session starts in a repository, and guessing one would put it somewhere unseen.
   */
  async startSession(picked: Picked, title?: string, root?: string, request?: string, tuning?: { model?: string; effort?: string }, after?: StartAfter, held?: boolean): Promise<SessionActed> {
    const repository = repositoryToStart(this.current.holds.repositories ?? [], picked, root);
    if (repository === undefined) return { ok: false, outcome: { ok: false, why: "no_manifest" } };
    const start = startOn(repository, title, root, request, tuning, after, held);
    if (start === null) return { ok: false, outcome: { ok: false, why: "not_set_up" } };
    return await this.sessions.start(start);
  }

  /** One request about the phone: the switch to this Fleet, the rest to the Gateway — `phone-gateway.ts`. */
  phone(request: PhoneRequest): Promise<PhoneAnswer> {
    return askPhone(request, this.connected()?.port ?? null);
  }

  private connected(): BridgeStateFleet | null {
    const connection = this.current.connection;
    return connection.state === "connected" ? connection.fleet : null;
  }

  // ------------------------------------------------------------------ state
  private settle(connection: Connection): void {
    this.publish({ connection });
  }

  private publish(change: Partial<BridgeState>): void {
    // Fleet's version rides on the identity, so it is brought current in the
    // one funnel every change passes through. `shared/bridge.ts` owns the rule.
    this.current = identifying({ ...this.current, ...change });
    // Every repository Fleet serves is still shared through this funnel;
    // which one a window picked is not — `repository` moved onto `PickedView`
    // when picks went per-window, so that half of Helm's own targeting is
    // wired at `RepositoryReads`'s own `onPicked`, the one place a pick
    // actually changes.
    if (change.holds !== undefined) this.helm.onRepositoriesChanged(this.current.holds.repositories ?? []);
    this.wiring.publish(this.current);
  }
}
