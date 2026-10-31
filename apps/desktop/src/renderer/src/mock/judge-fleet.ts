// Fleet's judge on demand, as the mock answers it: a press reads for two seconds, then the record arrives with its notes. **Provisional**: the request and
// the record are `judge/store.ts`'s, which the wire will replace. A pull request, the plan and a document are answered with notes, a commit with none, and
// a walk page, a sketch and a command with one note on the whole of them. When the press carried a prompt, the first note answers it.

import type { SessionsStore } from "./sessions/script";
import { plain } from "../cockpit/deck/session-tiles";
import { provideJudge, putRecord, type JudgeApi, type JudgeNote, type JudgeRecord, type JudgeRequest } from "../judge/store";

/** How long a judge reads before it answers. */
const READS_MS = 2000;

export const MIDDLEWARE = "crates/api/src/auth/middleware.rs";
export const TOKENS = "crates/api/src/auth/tokens.rs";

const note = (id: string, claim: string, more: Partial<JudgeNote> = {}): JudgeNote => ({ id, claim, ...more });

/** The note that answers a prompt. The mock does not read: a prompt about sign-ins gets the legacy tokens, any other is said not to be shown. */
function answerTo(prompt: string, where: string, about: string | undefined): JudgeNote {
  const signIns = /sign.?in|session|survive|legacy|token/i.test(prompt);
  return note(
    "asked",
    signIns ? "Existing sign-ins do not survive: a token issued before the switch is rejected, and nothing migrates it." : `Nothing in the change shows that ${prompt.replace(/\.$/, "")}.`,
    { where, ...(about === undefined ? {} : { about }) },
  );
}

/** What a reading of an artifact finds. `plan` is told which groups and tasks it can pin a note to by the scenario's ids. */
export function notesFor(request: JudgeRequest): JudgeNote[] {
  const { kind } = request.artifact;
  const asked = request.prompt === "" ? [] : null;
  switch (kind) {
    case "pull_request":
      return [
        ...(asked ?? [answerTo(request.prompt, `${MIDDLEWARE}:88`, `${MIDDLEWARE}:88`)]),
        note("n2", "A session token never expires when its lifetime is zero, so a misconfigured deploy signs people in for good.", { where: `${TOKENS}:41`, about: `${TOKENS}:41` }),
      ];
    case "plan":
      return [
        ...(asked ?? [answerTo(request.prompt, "Plan, group Move sign-in", "g1")]),
        note("n2", "No task moves the sign-ins that exist, so everyone is signed out when the switch ships.", { where: "Plan, task Issue session tokens", about: "T2" }),
        note("n3", "No task adds or changes a test for the token lifetime.", { where: "Plan" }),
      ];
    case "doc":
      return [
        ...(asked ?? [answerTo(request.prompt, "Write-up, second paragraph", "p:1")]),
        note("n2", `The write-up says a token lasts a day. The code sets thirty minutes.`, { where: `${TOKENS}:41`, about: "p:2" }),
      ];
    case "page":
      return [note("n1", "The walk never shows a signed-out start, so the first sign-in after the switch is not seen.", { where: request.artifact.title })];
    case "sketch":
      return [note("n1", "The sketch has the token refreshed by the client. The change refreshes it in the API.", { where: request.artifact.title })];
    case "command":
      return [note("n1", "The command exited 0 and printed two warnings about the old cookie name.", { where: request.artifact.title })];
    default:
      return asked === null ? [answerTo(request.prompt, request.artifact.title, undefined)] : [];
  }
}

/** The api the renderer asks through, over the store that holds the Job. Torn down with the scenario. */
export function mockJudge(store: Pick<SessionsStore, "get" | "send">): { api: JudgeApi; dispose: () => void } {
  const timers = new Set<number>();
  const held = new Map<string, JudgeRecord>();
  let made = 0;
  const put = (record: JudgeRecord) => {
    held.set(record.id, record);
    putRecord(record);
  };
  const edit = (id: string, change: (record: JudgeRecord) => JudgeRecord) => {
    const was = held.get(id);
    if (was !== undefined) put(change(was));
  };
  const settle = (record: string, id: string, how: "sent" | "dismissed") => edit(record, (one) => ({ ...one, notes: one.notes.map((it) => (it.id === id && it.settled === undefined ? { ...it, settled: how } : it)) }));
  const api: JudgeApi = {
    ask: async (request) => {
      made += 1;
      const id = `judge-${made}`;
      const reading: JudgeRecord = { id, job: request.job, artifact: request.artifact, prompt: request.prompt, state: "reading", asked_at: new Date().toISOString(), notes: [], read: false };
      put(reading);
      const handle = window.setTimeout(() => {
        timers.delete(handle);
        const rows = store.get().find((one) => one.id === request.job)?.rows ?? [];
        const last = rows.at(-1)?.id;
        put({ ...reading, state: "done", finished_at: new Date().toISOString(), notes: notesFor(request), ...(last === undefined ? {} : { after: last }) });
      }, READS_MS);
      timers.add(handle);
    },
    send: async (record, id) => {
      const one = held.get(record);
      const found = one?.notes.find((it) => it.id === id);
      if (one === undefined || found === undefined) return;
      settle(record, id, "sent");
      await store.send(one.job, plain(`${one.artifact.title}${found.where === undefined ? "" : `, ${found.where}`}: ${found.claim}`));
    },
    dismiss: async (record, id) => settle(record, id, "dismissed"),
    markRead: async (record) => edit(record, (one) => ({ ...one, read: true })),
  };
  provideJudge(api);
  return {
    api,
    dispose: () => {
      timers.forEach((one) => window.clearTimeout(one));
      provideJudge(undefined);
    },
  };
}
