// One stream message, folded into `FleetConnection`'s state.
//
// Pulled out of `connection.ts` because the switch on `message.event.kind` was
// the largest thing left in that file and grows by one arm per new kind of
// event Fleet publishes — the same shape of growth #891 named. **Kept whole,
// not split by kind**: `connection.ts`'s own history already tried that
// question and answered it — every arm reaches the same state and the same
// `publish`, so cutting the switch into several files would put one machine's
// transitions in several places rather than one. What moved instead is the
// switch's *address*: it now takes its state through `ArrivalHost`, a narrow
// view of `FleetConnection` built once in its constructor, so the switch
// itself carries no private field of the class it serves.
//
// `readLimits`, `readPreferences` and `readModels` travel with it: each is a stream
// arm's own read-and-publish, used nowhere else the switch is not.

import { connectedTo, PROTOCOL_ID, speaksOurProtocol } from "@armada/protocol";
import type { Connection, ModelChoices, ServerState, StreamMessage } from "@armada/protocol";
import type { BridgeState } from "../shared/bridge";
import type { FleetBuilds } from "./fleet-build";
import type { HeldReader } from "./holding";
import type { IssuesHost } from "./issues";
import type { Modding } from "./mods";
import type { Questions } from "./questions";
import type { RehearsalConnection } from "./rehearsal";
import { ask, limitsOf, mergeLinesOf, preferencesOf } from "./request";
import type { RepositoryReads } from "./repositories";
import type { BridgeStateFleet } from "./socket";
import type { SessionsHost } from "./sessions";
import { publishSettings, readSettings } from "./settings";
import type { SleepHost } from "./sleep";
import type { PlanUsageHost } from "./plan-usage";
import type { ModelDownshiftHost } from "./model-downshift";
import type { ProjectsHost } from "./projects";
import type { BugReportsHost } from "./bug-reports";
import type { LifecyclesHost } from "./lifecycles";
import type { AgentSignInHost } from "./agent-sign-in";
import type { ReviewsHost } from "./reviews";
import type { TimelineHost } from "./timeline";
import type { ActFailures } from "./act-failed";
import type { StudioReads } from "./studios";

/** What the arrival switch reaches on `FleetConnection`, and nothing more. */
export interface ArrivalHost {
  current(): BridgeState;
  now(): number;
  readonly repositories: RepositoryReads;
  readonly rehearsal: RehearsalConnection;
  /** Every open window's own Overview read again — `connection.ts`'s `windowFacades`. */
  overviewAgain(port: number): Promise<void>;
  /** Helm calls waiting on a person — `questions.ts`. */
  readonly questions: Pick<Questions, "helmAsking" | "helmAnswered" | "readHelmCalls">;
  readonly helm: { reconnected(port: number): void };
  /** The Studios surface's two reads — `studios.ts`. */
  readonly studios: Pick<StudioReads, "again" | "changed" | "deleted">;
  /** The mods on this machine — `mods.ts`. */
  readonly mods: Pick<Modding, "read" | "listed">;
  /** Every session and the threads a window opened — `sessions.ts`. */
  readonly sessions: Pick<SessionsHost, "again" | "changed" | "row" | "terminal" | "watchesChanged" | "browser">;
  /** What Fleet holds disk for, read once for nobody so the first surface to open has a list, and again on a slot lease or release — `holding.ts`. */
  readonly held: Pick<HeldReader, "warm" | "again">;
  /** Sleep mode, told when Fleet changes the night — `sleep.ts`. */
  readonly sleep: Pick<SleepHost, "changed">;
  /** The plan's windows, told when a session reports a newer reading — `plan-usage.ts`. */
  readonly planUsage: Pick<PlanUsageHost, "changed">;
  /** The default model's step down, told when Fleet moves it and read once per connection — `model-downshift.ts`. */
  readonly modelDownshift: Pick<ModelDownshiftHost, "changed" | "seed">;
  /** His projects, told whole after any write to one — `projects.ts`. */
  readonly projects: Pick<ProjectsHost, "changed">;
  /** The bugs agents reported, told whole after one is made, kept or dropped — `bug-reports.ts`. */
  readonly bugReports: Pick<BugReportsHost, "changed">;
  /** One Job's lifecycles, told whole after an instance is taken on or moves — `lifecycles.ts`. */
  readonly lifecycles: Pick<LifecyclesHost, "changed">;
  /** The agents' sign-in, told when Fleet moves it and read once per connection — `agent-sign-in.ts`. */
  readonly agentSignIn: Pick<AgentSignInHost, "changed" | "seed">;
  /** What happened across the Fleet, told each entry as it is kept — `timeline.ts`. */
  readonly timeline: Pick<TimelineHost, "appended">;
  /** A background act that failed after its reply, told to the windows whole — `act-failed.ts`. */
  readonly acts: Pick<ActFailures, "failed">;
  /** A review run or a held walk that moved, told to the windows whole — `reviews.ts`. */
  readonly reviews: Pick<ReviewsHost, "changed" | "walkChanged">;
  /** Every repository's Autopilot, told to the windows — `issues.ts`. #2092. */
  readonly issues: Pick<IssuesHost, "autopilotChanged" | "autopilotsAgain">;
  /** The build Fleet runs on, read now and every minute — `fleet-build.ts`. */
  readonly fleetBuild: Pick<FleetBuilds, "watch">;
  readonly socket: { close(): void; resetUnreachable(): void };
  publish(change: Partial<BridgeState>): void;
  settle(connection: Connection): void;
}

/**
 * Fleet's three admission limits. **Once per connection**, `readManifest`'s
 * terms, and again on every `settings.changed`, since settings.json holds them.
 */
export async function readLimits(port: number, publish: (change: Partial<BridgeState>) => void): Promise<void> {
  publish({ limits: await limitsOf(port) });
}

/**
 * A person's Bridge preferences. **Once per connection**, `readLimits`'
 * terms: nothing but a save changes them, and `Preferring.save` publishes the
 * new reading itself.
 *
 * **A failed read publishes nothing**, unlike `readLimits` — `preferencesOf`'s
 * reason: `BridgeState.preferences` is never `null`, so there is no "not read
 * yet" to draw, and overwriting it with the shipped default on a glitch would
 * draw a person's own choice as though it had never been saved.
 */
export async function readPreferences(port: number, publish: (change: Partial<BridgeState>) => void): Promise<void> {
  const read = await preferencesOf(port);
  if (read !== null) publish({ preferences: read });
}

/**
 * The models Fleet holds, read into `holds.models` alone. **A failed read keeps what was held**,
 * `holdingsOf`'s rule. The held `holds` is taken after the answer, so a listing published while
 * this was out is not written over. `fresh` has Fleet ask the harness again rather than answer
 * from what it kept.
 */
export async function readModels(port: number, host: Pick<ArrivalHost, "current" | "publish">, fresh = false): Promise<void> {
  const answer = await ask(port, "GET", fresh ? "/models?fresh=true" : "/models");
  if (answer.ok !== true) return;
  host.publish({ holds: { ...host.current().holds, models: answer.body as ModelChoices } });
}

export function applyArrival(host: ArrivalHost, text: string, fleet: BridgeStateFleet): void {
  let message: StreamMessage;
  try {
    message = JSON.parse(text) as StreamMessage;
  } catch {
    // The stream carries no error message, so an unparseable one is a
    // connection to drop rather than a state to fold.
    host.socket.close();
    return;
  }

  if (message.message === "resync") {
    // Again, because a Fleet restarted under a live socket is not the one
    // the runtime file described.
    if (!speaksOurProtocol(message.protocol_id)) {
      host.socket.close();
      host.settle({
        state: "protocol_mismatch",
        fleet,
        speaks: message.protocol_id,
        expected: PROTOCOL_ID,
      });
      return;
    }
    host.socket.resetUnreachable();
    host.publish({
      connection: connectedTo(fleet, message.cursor),
      readAt: host.now(),
    });
    // What a dispatch may name: read once per connection, because it changes
    // when Fleet restarts.
    void host.repositories.readHoldings(fleet.port);
    // And the build it runs on, once now and then every minute: a fetch moves its position and no
    // event says so.
    host.fleetBuild.watch(fleet.port);
    // And what Fleet's last read of `armada.yml` came to. **Once per
    // connection and never again**: it changes when somebody
    // saves a file, and `manifest.reread` is what says so. This read is for
    // the window that opened after the save — which is most windows, since a
    // refusal stands until the file is corrected.
    void host.repositories.readManifestForEveryWindow(fleet.port);
    // And Fleet's three admission limits, once per connection: nothing but a
    // save changes them, and that act publishes its own new reading.
    void readLimits(fleet.port, host.publish);
    // And a person's Bridge preferences, once per connection, `readLimits`'
    // reason.
    void readPreferences(fleet.port, host.publish);
    // And settings.json, once per connection — `settings.changed` carries the list whole.
    void readSettings(fleet.port, host.publish);
    // And the mods on this machine, once per connection — `mods.changed` carries the list whole.
    void host.mods.read(fleet.port);
    // And every server Fleet holds, once per connection — `server.*` on
    // `/events` carries each row whole from here on.
    void host.rehearsal.readServers(fleet.port);
    // And each repository's merge line, once per connection — `merge_lines.changed` carries it whole.
    void mergeLinesOf(fleet.port).then((mergeLines) => host.publish({ mergeLines }));
    // And every Helm call waiting on a person: events carry only what is asked next.
    void host.questions.readHelmCalls(fleet.port);
    // And Helm's own socket, on a repository already targeted — a second
    // socket to the same peer, `takeAgain`'s reason repeated one seam over.
    host.helm.reconnected(fleet.port);
    // And the Studios surface's two reads, where held: a Studio written while the stream was down
    // or lagging published a `studio.changed` this window never folded.
    void host.studios.again();
    // And every session, with the threads a window holds: a row published while the stream was down was never folded.
    void host.sessions.again(fleet.port);
    // And the pool's held worktrees, once, for nobody: the first open after a restart then has a list to draw.
    void host.held.warm(fleet.port);
    // And every Autopilot, `sessions.again`'s reason.
    void host.issues.autopilotsAgain();
    // And the default model's step down, once, for nobody: the notification is told where it stands, not that it moved.
    void host.modelDownshift.seed();
    // And the agents' sign-in, for the same reason.
    void host.agentSignIn.seed();
    return;
  }

  if (message.message === "missed") {
    // The count alone cannot repair what Bridge holds. A resync always
    // follows; until it lands the screen says how many were lost.
    host.publish({ missed: host.current().missed + message.dropped });
    return;
  }

  const event = message.event;
  const connection: Connection = connectedTo(fleet, message.cursor);

  // A Helm call waits inside one tool call for the length of one reply, so the card goes up on the
  // ask and comes down on the answer, whoever answered — a person, or fleet's bound running out. #1389.
  if (event.kind === "helm.asking_to_run") {
    host.questions.helmAsking(event.waiting);
    return;
  }
  if (event.kind === "helm.call_answered") {
    host.questions.helmAnswered(event.call);
    return;
  }
  if (event.kind === "manifest.reread") {
    // The whole reading replaces what is held rather than merging into it.
    // A read that took after one that was refused leaves nothing of the
    // refusal standing, and a merge would keep the old fault on screen
    // beside the news that the file is now fine.
    // Overview's drift spans every repository on All, so it reads whichever file this was.
    void host.overviewAgain(fleet.port);
    host.publish({ connection });
    // Every window whose own pick reads this file draws it, the Manifest reading and drift both.
    host.repositories.manifestReread(event);
    host.rehearsal.onManifestReread(event.path, fleet.port);
    return;
  }
  if (event.kind === "repositories.changed") {
    // Carried whole, so an add made in another window or at the CLI lands without a round trip.
    host.publish({ connection });
    return void host.repositories.listed(event, fleet.port);
  }
  if (event.kind === "mods.changed") {
    // Carried whole: the list replaces the one held.
    host.publish({ connection });
    host.mods.listed({ mods: event.mods });
    return;
  }
  if (event.kind === "settings.changed") {
    // Carried whole: a save, or a hand edit to the file.
    host.publish({ connection });
    const { kind: _kind, ...list } = event;
    publishSettings(list, host.publish);
    // The limits and preferences are read off the same file.
    for (const read of [readLimits, readPreferences]) void read(fleet.port, host.publish);
    // And the models: `models.default` and `models.session` live in the same file, and the Session
    // composer's empty Model option reads the second off `holds`.
    void readModels(fleet.port, host);
    return;
  }
  if (event.kind === "merge_lines.changed") {
    // Replaced whole, never folded.
    const { kind: _kind, ...mergeLines } = event;
    host.publish({ connection, mergeLines });
    return;
  }
  if (event.kind === "studio.changed") {
    // The Studio travels whole,
    // so the open one is replaced and the list's row with it, without a round trip.
    host.publish({ connection });
    const { kind: _kind, ...studio } = event;
    host.studios.changed(studio);
    return;
  }
  if (event.kind === "studio.deleted") {
    host.publish({ connection });
    host.studios.deleted({ id: event.id, manifest_id: event.manifest_id });
    return;
  }
  if (event.kind === "session.changed") {
    // The session travels whole.
    host.publish({ connection });
    const { kind: _kind, ...record } = event;
    host.sessions.changed(record);
    return;
  }
  if (event.kind === "watches.changed") {
    // One Job's watches, whole.
    host.publish({ connection });
    const { kind: _kind, ...change } = event;
    host.sessions.watchesChanged(change);
    return;
  }
  if (event.kind === "sleep.changed") {
    // The night travels whole.
    host.publish({ connection });
    const { kind: _kind, ...state } = event;
    host.sleep.changed(state);
    return;
  }
  if (event.kind === "plan_usage.changed") {
    // Above the tail for `sleep.changed`'s reason: no Job to find, and the reading travels whole.
    host.publish({ connection });
    const { kind: _kind, ...usage } = event;
    host.planUsage.changed(usage);
    return;
  }
  if (event.kind === "model_downshift.changed") {
    // Above the tail for `plan_usage.changed`'s reason: no Job to find, and the state travels whole.
    host.publish({ connection });
    const { kind: _kind, ...state } = event;
    host.modelDownshift.changed(state);
    // An accept or a restore moves `models.session`, which `holds.models` read once and would keep.
    void readModels(fleet.port, host);
    return;
  }
  if (event.kind === "projects.changed") {
    // Above the tail for `model_downshift.changed`'s reason: no Job to find, and the projects travel whole.
    host.publish({ connection });
    const { kind: _kind, ...projects } = event;
    host.projects.changed(projects);
    return;
  }
  if (event.kind === "bug_reports.changed") {
    // Above the tail for `model_downshift.changed`'s reason: no Job to find, and the reports travel whole.
    host.publish({ connection });
    const { kind: _kind, ...reports } = event;
    host.bugReports.changed(reports);
    return;
  }
  if (event.kind === "job_lifecycles.changed") {
    // Above the tail for `model_downshift.changed`'s reason: it carries a `session_id` the tail would take for a Job's row, and it travels whole.
    host.publish({ connection });
    const { kind: _kind, ...held } = event;
    host.lifecycles.changed(held);
    return;
  }
  if (event.kind === "agent_sign_in.changed") {
    // Above the tail for `model_downshift.changed`'s reason: no Job to find, and the state travels whole.
    host.publish({ connection });
    const { kind: _kind, ...state } = event;
    host.agentSignIn.changed(state);
    return;
  }
  if (event.kind === "act.failed") {
    // The refusal travels whole under `error`; main keeps none of it.
    host.publish({ connection });
    const { kind: _kind, ...failed } = event;
    host.acts.failed(failed);
    return;
  }
  if (event.kind === "timeline.appended") {
    // The entry travels whole.
    host.publish({ connection });
    host.timeline.appended(event.event);
    // A lease or a release moves the pool, and the pool is read only when a surface asks or an act
    // re-reads it, so a Session taking a slot on its first write left the bays as drawn. Fleet
    // enters the event after the pool changes, so the read finds the new holder. Nothing open is no read.
    if (event.event.kind === "slot.leased" || event.event.kind === "slot.released") void host.held.again(fleet.port);
    return;
  }
  if (event.kind === "review_walk.changed") {
    // Above the tail as the run's is: no Job to find, and the walk travels whole.
    host.publish({ connection });
    host.reviews.walkChanged(event.walk);
    return;
  }
  if (event.kind === "review_run.changed") {
    // The run travels whole in `run`, since a run has a `kind` of its own. Main keeps none of it.
    host.publish({ connection });
    host.reviews.changed(event.run);
    return;
  }
  if (event.kind === "autopilot.changed") {
    // The Autopilot travels whole.
    // Main keeps none of it: each window folds what it is told by repository, and a resync tells
    // every one again.
    host.publish({ connection });
    const { kind: _kind, ...autopilot } = event;
    host.issues.autopilotChanged(autopilot);
    return;
  }
  if (event.kind === "session.row") {
    host.publish({ connection });
    const { kind: _kind, ...change } = event;
    host.sessions.row(change);
    return;
  }
  if (event.kind === "terminal.opened" || event.kind === "terminal.exited") {
    // Nothing is folded: a shell is not part of the Session's record. The window is told, and what
    // the shell prints arrives as a run's does.
    host.publish({ connection });
    host.sessions.terminal(event.kind === "terminal.opened" ? "opened" : "exited", { session_id: event.session_id, run_id: event.run_id });
    return;
  }
  if (event.kind === "browser.request") {
    // Nothing is folded: the act is the Job browser's to do and answer, and Fleet holds the agent's call until it does.
    host.publish({ connection });
    host.sessions.browser({ session_id: event.session_id, request_id: event.request_id, act: event.act });
    return;
  }
  if (event.kind === "checkout_run.finished") {
    host.publish({ connection });
    host.rehearsal.onCheckoutRunFinished(fleet.port);
    return;
  }
  if (
    event.kind === "server.starting" ||
    event.kind === "server.serving" ||
    event.kind === "server.exited"
  ) {
    // Replaced, never patched: the event carries the whole `ServerState`. The list alone moved no
    // sheet, so Run and Stop on a server drew nothing until a reload.
    const row: ServerState = event;
    const servers = host.rehearsal.onServerEvent(host.current().servers.servers, row);
    host.publish({ connection, servers: { servers } });
    host.rehearsal.onServerMoved(fleet.port);
    return;
  }

  // A newer Fleet's kind, or one that moves nothing Bridge holds.
  host.publish({ connection });
}
