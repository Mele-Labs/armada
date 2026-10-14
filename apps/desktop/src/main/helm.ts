// Which repository Helm answers for, and the requests that send into its Session. Helm is a hosted
// Session (`docs/concepts/helm.md`): its thread, its turn and its model are the Session's, held by
// `sessions.ts` with every other Session and read by the dock from there. What is Helm's own, and
// lives here, is only the aim and the four requests the dock makes: ask, start fresh, read a thought
// and record where one went. #939, #944.
//
// A Session a request answers with is folded into the Sessions host at once, so the dock draws the
// Helm Session the moment its first message is sent and does not wait on the stream to say so.

import type {
  AskHelm,
  HelmContext,
  HelmDebugInfo,
  HelmDebugRead,
  Outcome,
  ReadThought,
  RepositorySummary,
  SessionRecord,
  ThoughtRead,
  ThoughtReading,
  ThoughtSent,
  ThoughtTarget,
} from "@armada/protocol";
import type { HelmAim, HelmCarry } from "../shared/api/helm";
import { ask, MODEL_CALL_MS } from "./request";

/**
 * Which repository Helm answers for, and the requests into its Session.
 *
 * The most recent explicit act wins — a pick to a specific repository, or a
 * point ("Discuss with Helm", the dock's own switch) — and failing either,
 * Helm stays on the last repository it actually heard from this run, or the
 * first one Fleet lists.
 */
export class HelmConnection {
  private readonly publish: (change: { helm: HelmAim }) => void;
  private readonly port: () => number | null;
  /** A Session a request answered with, handed to the Sessions host. */
  private readonly folded: (record: SessionRecord) => void;
  private repositories: readonly RepositorySummary[] = [];
  private aimed: HelmAim = { state: "none" };
  /**
   * The most recent explicit act — a pick to a specific repository, or a
   * point ("Discuss with Helm", the dock's own switch) — whichever came
   * last. `none` until either has ever happened.
   *
   * **Moving to All is not an act with a target of its own.** A pick to a
   * repository sets this; a pick to All carries none to set, so it leaves
   * whichever act was most recent standing — which is what lets a person
   * pick a repository, look at All, and find Helm still on the one they
   * picked, and what lets Discuss survive the rail moving under it.
   */
  private explicit: { kind: "picked"; root: string } | { kind: "pointed"; manifestId: string } | { kind: "none" } = {
    kind: "none",
  };
  /** The last repository a message was actually sent to. In memory for this run of Bridge —
   * surviving a quit needs a `crates/config` field or a new local store, neither of which this reaches. */
  private lastTalked: string | null = null;

  constructor(wiring: {
    publish: (change: { helm: HelmAim }) => void;
    port: () => number | null;
    folded: (record: SessionRecord) => void;
  }) {
    this.publish = wiring.publish;
    this.port = wiring.port;
    this.folded = wiring.folded;
  }

  /** Nothing is held open: Helm's thread rides the Sessions stream. */
  close(): void {}

  /** The repositories Fleet serves changed, or were read for the first time. */
  onRepositoriesChanged(repositories: readonly RepositorySummary[]): void {
    this.repositories = repositories;
    this.retarget();
  }

  /** The rail's own pick moved. `null` is All repositories, which names nothing to point at. */
  onPicked(root: string | null): void {
    if (root !== null) this.explicit = { kind: "picked", root };
    this.retarget();
  }

  /** "Discuss with Helm" on a card, or the dock's own switch. The rail's pick does not move. */
  point(manifestId: string): void {
    this.explicit = { kind: "pointed", manifestId };
    this.retarget();
  }

  /** The connection came back: say again which repository Helm answers for. */
  reconnected(_port: number): void {
    this.publish({ helm: this.aimed });
  }

  /**
   * `POST /helm/ask`. Answers the Helm Session's record, which the first message of a repository makes;
   * the reply is that Session's thread, on the stream.
   */
  async askHelm(text: string, context?: HelmContext, carry?: HelmCarry): Promise<Outcome> {
    const manifestId = this.targetManifestId();
    const port = this.port();
    if (manifestId === null || port === null) return { ok: false, why: "not_connected" };
    const attachments = carry?.attachments ?? [];
    const mentions = carry?.mentions ?? [];
    // Words may be blank when the message carries something; with neither it is not sent.
    if (text.trim() === "" && attachments.length === 0 && mentions.length === 0) return { ok: false, why: "empty_brief" };
    const body: AskHelm = {
      text,
      ...(context === undefined ? {} : { context }),
      ...(attachments.length === 0 ? {} : { attachments }),
      ...(mentions.length === 0 ? {} : { mentions }),
      ...(carry?.model === undefined ? {} : { model: carry.model }),
      ...(carry?.effort === undefined ? {} : { effort: carry.effort }),
    };
    const answer = await ask(port, "POST", `/helm/ask?manifest_id=${encodeURIComponent(manifestId)}`, body);
    if (answer.ok !== true) return answer.outcome;
    this.lastTalked = manifestId;
    this.folded(answer.body as SessionRecord);
    return { ok: true };
  }

  /** `POST /helm/start_fresh`. The Session ends, whatever it was doing, and the next message starts a new one. */
  async startFresh(): Promise<Outcome> {
    const manifestId = this.targetManifestId();
    const port = this.port();
    if (manifestId === null || port === null) return { ok: false, why: "not_connected" };
    const answer = await ask(port, "POST", `/helm/start_fresh?manifest_id=${encodeURIComponent(manifestId)}`);
    return answer.ok === true ? { ok: true } : answer.outcome;
  }

  /**
   * `POST /helm/read_thought`. **Answered to the caller and never published**: it is the dock's gesture
   * on one message. No repository in the path, since a thought is read against everything Fleet serves.
   */
  async readThought(text: string): Promise<ThoughtRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const body: ReadThought = { text };
    // A model call is inside the request, and Fleet answers `question` at its own 20 s budget.
    const answer = await ask(port, "POST", "/helm/read_thought", body, MODEL_CALL_MS);
    return answer.ok === true
      ? { ok: true, reading: answer.body as ThoughtReading }
      : { ok: false, outcome: answer.outcome };
  }

  /** `POST /helm/thought_sent`. Said after the words were delivered; the `sent` row arrives as a row of the Helm Session. */
  async recordThoughtSent(manifestId: string, text: string, target: ThoughtTarget, carry?: HelmCarry): Promise<Outcome> {
    const port = this.port();
    if (port === null) return { ok: false, why: "not_connected" };
    const attachments = carry?.attachments ?? [];
    const mentions = carry?.mentions ?? [];
    const body: ThoughtSent = {
      text,
      target,
      ...(attachments.length === 0 ? {} : { attachments }),
      ...(mentions.length === 0 ? {} : { mentions }),
    };
    const answer = await ask(port, "POST", `/helm/thought_sent?manifest_id=${encodeURIComponent(manifestId)}`, body);
    if (answer.ok !== true) return answer.outcome;
    this.folded(answer.body as SessionRecord);
    return { ok: true };
  }

  /**
   * `GET /helm/debug` — the Helm Session of whichever repository Helm answers for, as one record taken now.
   *
   * **Answered to the caller and never published.** It is one person's gesture; a record in
   * `BridgeState` would redraw every surface each time a reply arrived.
   */
  async debugInfo(): Promise<HelmDebugRead> {
    const manifestId = this.targetManifestId();
    const port = this.port();
    if (manifestId === null || port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const answer = await ask(port, "GET", `/helm/debug?manifest_id=${encodeURIComponent(manifestId)}`);
    return answer.ok === true
      ? { ok: true, record: answer.body as HelmDebugInfo }
      : { ok: false, outcome: answer.outcome };
  }

  /** The repository Helm answers for right now, for the dock to name. `null` where nothing is servable yet. */
  target(): string | null {
    return this.targetManifestId();
  }

  private retarget(): void {
    const manifestId = this.targetManifestId();
    const same = manifestId === null ? this.aimed.state === "none" : this.aimed.state === "pointed" && this.aimed.manifestId === manifestId;
    if (same) return;
    this.aimed = manifestId === null ? { state: "none" } : { state: "pointed", manifestId };
    this.publish({ helm: this.aimed });
  }

  private targetManifestId(): string | null {
    const explicit = this.explicit;
    if (explicit.kind === "picked") {
      // Looked up live, not snapshotted: a repository picked before its
      // Manifest finished reading is followed to it the moment it arrives.
      return this.repositories.find((one) => one.root === explicit.root)?.manifest?.id ?? null;
    }
    if (explicit.kind === "pointed") return explicit.manifestId;
    if (this.lastTalked !== null) return this.lastTalked;
    return this.repositories.find((one) => one.manifest !== undefined)?.manifest?.id ?? null;
  }
}
