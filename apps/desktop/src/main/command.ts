// The acts Bridge sends that belong to no one surface: preferences, settings, a file search, the
// slot pool, an answer to a Helm call and an answer to a retro item.
//
// Beside `connection.ts` rather than inside it: the connection is a socket, a runtime file and a
// state machine, and a POST is none of those.
//
// Nothing here holds a connection. It is handed a `Board`, which is the whole of what an act needs:
// where to send, and where to publish.
//
// **Bridge never talks to a Drone.** Every act below names Fleet.

import type { RequestRepositoryRead } from "../shared/api/dispatch";
import { readRequestRepository as readRequest } from "./request-repository";
import type { RescueOutcome } from "@armada/cleanup/api";
import type { BridgeState } from "../shared/bridge";
import type { FilesFound, Outcome, SavePreference, SaveSettings } from "@armada/protocol";
import type { ChangeSlotPool, RescueSlot, SlotPoolChanged, SlotRescued } from "@armada/protocol";
import type { AnswerHelmCall, HelmCallAnswer } from "@armada/protocol";
import type { LessonAnswer } from "@armada/protocol";
import type { LessonSessionRead } from "@armada/screens/src/lessons/session-wire";
import { ask } from "./request";
import type { Picked } from "./picked";
import { Preferring } from "./preferences";
import { Settling } from "./settings";
import { Lessoning } from "./lessoning";

/**
 * What an act needs of the connection, and nothing more.
 *
 * The port is asked for rather than held, because a command sent over a connection that has since
 * dropped is a command with nowhere to go — and it is asked for **once**, at the top of the act.
 */
export type Board = {
  /** Where Fleet answers, or `null` when nothing is connected. */
  port: () => number | null;
  /** The repository a search is about — `picked.ts`. */
  picked: Picked;
  publish: (change: Partial<BridgeState>) => void;
};

export class Commands {
  private readonly board: Board;
  /** A person's Bridge preferences — `preferences.ts`. */
  private readonly preferring: Preferring;
  /** settings.json — `settings.ts`. */
  private readonly settling: Settling;
  /** Retro items being answered — `lessoning.ts`. */
  private readonly lessoning: Lessoning;

  constructor(board: Board) {
    this.board = board;
    this.preferring = new Preferring(board);
    this.settling = new Settling(board);
    this.lessoning = new Lessoning(board);
  }

  // --------------------------------------------------------- Bridge preferences
  /** Save one preference by name. `preferences.ts` holds why this is not the
   *  shape `act` or `setting` above are. */
  savePreference(save: SavePreference): Promise<Outcome> {
    return this.preferring.save(save);
  }

  // ---------------------------------------------------------------- settings.json
  /** Change settings by key; `null` removes one. `settings.ts`. */
  saveSettings(changes: SaveSettings["changes"]): Promise<Outcome> {
    return this.settling.save(changes);
  }

  /**
   * Paths under the checkout narrowed against typed text, for the `@` mention
   * popup — a person opens it while writing a request or a brief.
   *
   * **Empty rather than an outcome, on nothing connected or on a refusal.**
   * This fires on every keystroke; sending a search failure through the same
   * pipeline every other command failure uses would raise a toast for a
   * popup nobody has to have open. `search_files` itself never refuses — see
   * `Queries::search_files` — so what is left to answer empty for is Bridge
   * having nowhere to ask at all.
   *
   * `picked` is this window's own — `main/index.ts` names it; `this.board.picked` only where a
   * caller (`picked.test.ts`) supplies none. `manifestId` names the repository outright: a Session's message box
   * searches the Session's own, whatever the window has picked.
   */
  async searchFiles(query: string, picked: Picked = this.board.picked, manifestId?: string): Promise<string[]> {
    const port = this.board.port();
    const q = encodeURIComponent(query);
    const path = manifestId === undefined ? picked.manifest(`/manifest/files?q=${q}`) : picked.manifestNamed(`/manifest/files?q=${q}`, manifestId);
    if (port === null || path === null) return [];
    const answer = await ask(port, "GET", path);
    return answer.ok === true ? (answer.body as FilesFound).paths : [];
  }

  /** Which repository the words typed so far are about — `request-repository.ts`. */
  async readRequestRepository(words: string): Promise<RequestRepositoryRead> {
    return readRequest(this.board, words);
  }

  /**
   * Add a slot to one repository's pool, remove one, or close or reopen one,
   * on this machine only. A refusal comes back coded — `fleet.slot_held` and
   * its siblings — so Cleanup says on the slot why it would not go.
   */
  async changeSlotPool(manifestId: string, change: ChangeSlotPool): Promise<Outcome> {
    const port = this.board.port();
    if (port === null) return { ok: false, why: "not_connected" };
    const answer = await ask(port, "POST", this.board.picked.manifestNamed("/worktrees/slots", manifestId), change);
    return answer.ok === true ? { ok: true, slotChanged: answer.body as SlotPoolChanged } : answer.outcome;
  }

  /**
   * Start or stop the rescue Scout on a stranded slot, or Scrap or Stash what
   * it holds. A refusal comes back coded, `fleet.rescue_reading` and its
   * siblings, so the bay says why. The receipt rides on the answer.
   */
  async rescueSlot(manifestId: string, rescue: RescueSlot): Promise<RescueOutcome> {
    const port = this.board.port();
    if (port === null) return { ok: false, why: "not_connected" };
    const answer = await ask(port, "POST", this.board.picked.manifestNamed("/worktrees/slots/rescue", manifestId), rescue);
    return answer.ok === true ? { ok: true, rescued: answer.body as SlotRescued } : (answer.outcome as RescueOutcome);
  }

  /**
   * Answer one held Helm call. #1389.
   *
   * **Outside `act` and outside every in-flight set here**, because a Helm call
   * has no board row: nothing is re-read, and the answer
   * lands inside a session's own tool call rather than on a row. Fleet answers
   * with the calls still waiting, which the dock does not need — the event says
   * the same thing — so only the outcome comes back.
   */
  async answerHelmCall(call: string, answer: HelmCallAnswer, note?: string): Promise<Outcome> {
    const port = this.board.port();
    if (port === null) return { ok: false, why: "not_connected" };
    // Nothing rather than an empty string, `answerCommand`'s rule.
    const said = note?.trim() ?? "";
    const body: AnswerHelmCall = { call, answer, ...(said === "" ? {} : { note: said }) };
    const answered = await ask(port, "POST", "/helm/calls/answer", body);
    return answered.ok === true ? { ok: true } : answered.outcome;
  }

  /** Agree with one retro item. `lessoning.ts`. */
  agreeLesson(lessonId: string): Promise<LessonAnswer> {
    return this.lessoning.answer(lessonId, "agree");
  }

  /** Disagree with one retro item: Fleet discards it. */
  disagreeLesson(lessonId: string): Promise<LessonAnswer> {
    return this.lessoning.answer(lessonId, "disagree");
  }

  /** Agree with one retro item by starting a Session on its repository. `lessoning.ts`. */
  startLessonSession(lessonId: string): Promise<LessonSessionRead> {
    return this.lessoning.startSession(lessonId);
  }
}
