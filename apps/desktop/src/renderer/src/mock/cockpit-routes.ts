// The session routes the Cockpit calls, served over a scenario's Sessions as the mock holds them
// (`sessions/script.ts`): `POST /sessions/waiting/answer`, `/sessions/waiting/dismiss` and
// `/sessions/claim_pull_request`.
// Each answers and refuses the way Fleet does, so the card is tested against the real shapes.
// **A scenario can make each slow the way Fleet is** (`SessionsStore.pace`): it answers after `answerMs`,
// and the Session's list drops the item only `settleMs` after that, because Fleet takes the POST before
// it has re-derived what the Session waits on. A card answered in the window between is the race the
// Cockpit holds a call out for. Otherwise it answers at once.
// A scenario with a Fleet of its own for Sessions (`sessions-fleet.ts`) serves them there instead.

import type { Outcome, SessionRecord } from "@armada/protocol";
import { titleOf } from "@armada/screens";

import type { BridgeApi } from "../../../shared/api";
import type { BridgeState } from "../../../shared/bridge";
import { viewsOf } from "../merge-line";
import { permissionOf, waitingOf } from "../cockpit/waiting";
import type { SessionsStore } from "./sessions/script";

/** Items the mock Fleet refuses, by id, with the code and words it refuses in: a walk's way to show a refusal. */
export const REFUSES = new Map<string, { code: string; message: string }>();

const INSTANT = { answerMs: 0, settleMs: 0 };

/** Fleet as slow as it is, for a scenario that shows the wait: it answers after `answerMs`, and stops carrying the item `settleMs` after that. A test may shorten it. */
export const SLOW_FLEET = { answerMs: 1200, settleMs: 1500 };

const after = (ms: number) => (ms === 0 ? Promise.resolve() : new Promise<void>((done) => window.setTimeout(done, ms)));

const refused = (code: string, message: string) => ({
  ok: false as const,
  outcome: { ok: false, why: "refused", error: { code, message, run_id: "", fields: {}, chain: [] } } satisfies Outcome,
});

/** The record a served act answers with. The Cockpit reads that it was taken and nothing more. */
const taken = (id: string) => ({ ok: true as const, value: { id } as SessionRecord });

export function cockpitRoutes(sessions: SessionsStore, state: () => BridgeState, publish: (change: Partial<BridgeState>) => void): Pick<BridgeApi, "answerWaiting" | "claimPullRequest" | "dismissWaiting"> {
  return {
    answerWaiting: async ({ session_id, item_id, choice, text }) => {
      const { answerMs, settleMs } = sessions.pace ?? INSTANT;
      await after(answerMs);
      const refusal = REFUSES.get(item_id);
      if (refusal !== undefined) return refused(refusal.code, refusal.message);
      const session = sessions.get().find((one) => one.id === session_id);
      const item = session === undefined ? undefined : waitingOf(session).find((one) => one.id === item_id);
      if (session === undefined || item === undefined) {
        return refused("fleet.session_waiting_unheld", "nothing is waiting under that item. It was settled already, or the session stopped waiting");
      }
      // A mode hands the decision over, which the mock cannot weigh: it takes the first thing offered.
      const chosen = choice === undefined ? (text ?? item.options?.[0]?.label) : item.options?.[choice]?.label;
      const served = session.waitingFor?.some((one) => one.id === item_id) === true;
      const settle = () => {
        if (served) sessions.answerWaiting?.(session_id, item_id, { ...(choice === undefined ? {} : { choice }), ...(text === undefined ? {} : { text }) });
        else if (item.source === "permission") sessions.answer(session_id, permissionOf(chosen ?? "") ?? "refuse");
        else sessions.answer(session_id, undefined, [{ question: item.text, chosen: chosen === undefined ? [] : [chosen] }]);
      };
      if (settleMs === 0) settle();
      else window.setTimeout(settle, settleMs);
      return taken(session_id);
    },
    dismissWaiting: async ({ session_id, item_id }) => {
      const { answerMs, settleMs } = sessions.pace ?? INSTANT;
      await after(answerMs);
      const refusal = REFUSES.get(item_id);
      if (refusal !== undefined) return refused(refusal.code, refusal.message);
      const session = sessions.get().find((one) => one.id === session_id);
      if (session === undefined || !waitingOf(session).some((one) => one.id === item_id)) {
        return refused("fleet.session_waiting_unheld", "nothing is waiting under that item. It was settled already, or the session stopped waiting");
      }
      if (settleMs === 0) sessions.drop(session_id, item_id);
      else window.setTimeout(() => sessions.drop(session_id, item_id), settleMs);
      return taken(session_id);
    },
    claimPullRequest: async ({ number, session_id, job_id }) => {
      await after((sessions.pace ?? INSTANT).answerMs);
      if ((session_id === undefined) === (job_id === undefined)) return refused("fleet.pull_request_not_claimable", "a pull request is claimed by a session or a Job, not one of each or neither");
      const job = job_id === undefined ? undefined : state().jobs.find((one) => one.id === job_id);
      if (job_id !== undefined && job === undefined) return refused("fleet.no_such_job", `no job ${job_id}`);
      const mine = sessions.get().find((one) => one.id === session_id);
      if (job === undefined && mine === undefined) return refused("fleet.no_such_session", `no session ${String(session_id)}`);
      const pull = viewsOf(state())
        .flatMap((view) => view.hub?.pulls ?? [])
        .find((one) => one.number === number);
      if (pull === undefined) return refused("fleet.pull_request_not_claimable", `pull request #${number} is not open`);
      const holder = sessions.get().find((one) => one.dead === undefined && one.attachments.some((a) => a.kind === "pull_request" && a.number === number));
      if (holder !== undefined) return refused("fleet.pull_request_not_claimable", `pull request #${number} is held by session ${holder.title ?? holder.id}`);
      if (pull.job !== undefined) return refused("fleet.pull_request_not_claimable", `pull request #${number} is held by job ${pull.job.title}`);
      // A Job holds it where its row on the merge line names it, which is where the card reads an owner from.
      if (job !== undefined) {
        const lines = (state().mergeLines?.lines ?? []).map((line) => ({
          ...line,
          hub: line.hub === undefined ? line.hub : { ...line.hub, pull_requests: line.hub.pull_requests?.map((one) => (one.number === number ? { ...one, job: { id: job.id, title: titleOf(job) } } : one)) },
        }));
        publish({ mergeLines: { lines } });
        return { ok: true as const, value: { number, branch: pull.branch, url: pull.url, holder_kind: "job", holder_id: job.id } };
      }
      if (mine === undefined) return refused("fleet.no_such_session", `no session ${String(session_id)}`);
      sessions.attach(mine.id, { kind: "pull_request", number, title: pull.branch, branch: pull.branch, address: pull.url, checks: { state: "pending" }, state: "open", auto: false });
      return { ok: true as const, value: { number, branch: pull.branch, url: pull.url, holder_kind: "session", holder_id: mine.id } };
    },
  };
}
