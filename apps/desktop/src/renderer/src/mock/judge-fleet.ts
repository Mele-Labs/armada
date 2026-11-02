// Fleet's Judge as the mock answers it: the four routes (`ask_judge`, `send_judge_note`, `dismiss_judge_note`, `mark_judge_read`) over the Job the scenario
// holds, publishing the readings on the Job as `judge_readings` does. A press reads for two seconds, then the reading arrives with its notes. A pull request, the plan
// a document and the whole Job are answered with notes, a commit with none, and a walk page, a sketch and a command with one note on the whole of them. When the press carried a
// prompt, the first note answers it. The mock does not read: it answers.

import type {
  AskJudge,
  DismissJudgeNote,
  JudgeNote,
  JudgeReading,
  MarkJudgeRead,
  Outcome,
  SendJudgeNote,
} from "@armada/protocol";
import type { SessionJudgeActs } from "@armada/screens/src/draft/sessions";

import type { SessionActed } from "../../../shared/api/sessions";
import { plain } from "../cockpit/deck/session-tiles";
import { keyOf } from "../judge/store";
import type { SessionsStore } from "./sessions/script";

/** How long a judge reads before it answers. */
const READS_MS = 2000;
/** A prompt that makes the reading fail, so a walk can show one. */
export const FAILS = "the deploy is safe";
const FAILED = "The judge stopped before it answered. Fleet restarted while it was reading.";

export const MIDDLEWARE = "crates/api/src/auth/middleware.rs";
export const TOKENS = "crates/api/src/auth/tokens.rs";

const note = (id: string, claim: string, more: Partial<JudgeNote> = {}): JudgeNote => ({
  id,
  claim,
  ...more,
});

/** The note that answers a prompt. A prompt about sign-ins gets the legacy tokens, any other is said not to be shown. */
function answerTo(prompt: string, where: string, about: string | undefined): JudgeNote {
  const signIns = /sign.?in|session|survive|legacy|token/i.test(prompt);
  return note(
    "asked",
    signIns
      ? "Existing sign-ins do not survive: a token issued before the switch is rejected, and nothing migrates it."
      : `Nothing in the change shows that ${prompt.replace(/\.$/, "")}.`,
    {
      where,
      ...(about === undefined
        ? {}
        : {
            about,
          }),
    },
  );
}

/** What a reading of an artifact finds. `plan` is told which groups and tasks it can pin a note to by the scenario's ids. */
export function notesFor({
  artifact,
  prompt,
}: Pick<AskJudge, "artifact"> & {
  prompt: string;
}): JudgeNote[] {
  const asked = prompt === "" ? [] : null;
  switch (artifact.kind) {
    case "pull_request":
      return [
        ...(asked ?? [
          answerTo(prompt, `${MIDDLEWARE}:88`, `${MIDDLEWARE}:88`),
        ]),
        note(
          "n2",
          "A session token never expires when its lifetime is zero, so a misconfigured deploy signs people in for good.",
          {
            where: `${TOKENS}:41`,
            about: `${TOKENS}:41`,
          },
        ),
      ];
    case "plan":
      return [
        ...(asked ?? [
          answerTo(prompt, "Plan, group Move sign-in to session tokens", "g1"),
        ]),
        note(
          "n2",
          "The token is checked on every request, and no task says what happens to a request that still carries the old cookie.",
          {
            where: "Plan, task Check the token on every request",
            about: "T2",
          },
        ),
        note("n3", "No task adds or changes a test for the token lifetime."),
      ];
    case "doc":
      return [
        ...(asked ?? [
          answerTo(
            prompt,
            "A token issued before the switch",
            "p:A token issued before the switch is read once and exchanged for a session token.",
          ),
        ]),
        note("n2", `The write-up says a token lasts a day. The code sets thirty minutes.`, {
          where: `${TOKENS}:41`,
          about: "p:A session token lasts a day, then the client signs in again.",
        }),
        note(
          "n3",
          "The heading says how a token lives, and nothing under it says how one is revoked.",
          {
            where: "How a token lives",
            about: "p:How a token lives",
          },
        ),
        note("n4", "The write-up names no owner for the switch.", {
          where: artifact.title,
        }),
      ];
    case "job":
      return [
        ...(asked ?? [
          answerTo(prompt, "Plan, group Move sign-in to session tokens", "g1"),
        ]),
        note("n2", "No task says what happens to a request that still carries the old cookie.", {
          where: "Plan, task Check the token on every request",
          about: "T2",
        }),
        note(
          "n3",
          "A session token never expires when its lifetime is zero, so a misconfigured deploy signs people in for good.",
          {
            where: `${TOKENS}:41`,
            about: `${TOKENS}:41`,
          },
        ),
      ];
    case "page":
      return [
        note(
          "n1",
          "The walk never shows a signed-out start, so the first sign-in after the switch is not seen.",
          {
            where: artifact.title,
          },
        ),
      ];
    case "sketch":
      return [
        note(
          "n1",
          "The sketch has the token refreshed by the client. The change refreshes it in the API.",
          {
            where: artifact.title,
          },
        ),
      ];
    case "command":
      return [
        note("n1", "The command exited 0 and printed two warnings about the old cookie name.", {
          where: artifact.title,
        }),
      ];
    default:
      return asked === null
        ? [
            answerTo(prompt, artifact.title, undefined),
          ]
        : [];
  }
}

const refused = (code: string, message: string): SessionActed<never> => ({
  ok: false,
  outcome: {
    ok: false,
    why: "refused",
    error: {
      code,
      message,
    },
  } as unknown as Outcome,
});

export type JudgeRoutes = {
  askJudge: (ask: AskJudge) => Promise<SessionActed<JudgeReading>>;
  sendJudgeNote: (send: SendJudgeNote) => Promise<SessionActed<JudgeReading>>;
  dismissJudgeNote: (dismiss: DismissJudgeNote) => Promise<SessionActed<JudgeReading>>;
  markJudgeRead: (read: MarkJudgeRead) => Promise<SessionActed<JudgeReading>>;
};

/** The routes the window's api answers, and the acts the draft's Sessions hand to them. Torn down with the scenario. */
export function mockJudge(store: Pick<SessionsStore, "get" | "edit" | "send">): {
  routes: JudgeRoutes;
  acts: SessionJudgeActs;
  dispose: () => void;
} {
  const timers = new Set<number>();
  let made = 0;
  const held = (job: string) => store.get().find((one) => one.id === job)?.judgeReadings ?? [];
  /** The reading replaces the one before it for the same artifact, or itself. */
  const put = (job: string, reading: JudgeReading) =>
    store.edit(job, (one) => ({
      ...one,
      judgeReadings: [
        ...(one.judgeReadings ?? []).filter(
          (it) => it.id !== reading.id && keyOf(it.artifact) !== keyOf(reading.artifact),
        ),
        reading,
      ],
    }));
  const find = (job: string, id: string) => held(job).find((one) => one.id === id);
  const missing = () =>
    refused("fleet.no_such_judge_reading", "No reading of this Job has that id.");
  const settle = (
    job: string,
    reading: JudgeReading,
    id: string,
    how: "sent" | "dismissed",
  ): JudgeReading => {
    const next = {
      ...reading,
      notes: reading.notes.map((one) =>
        one.id === id && one.settled === undefined
          ? {
              ...one,
              settled: {
                how,
                at: new Date().toISOString(),
              },
            }
          : one,
      ),
    };
    put(job, next);
    return next;
  };
  const routes: JudgeRoutes = {
    askJudge: async ({ session_id: job, artifact, prompt = "" }) => {
      const going = held(job).find(
        (one) => one.state === "reading" && keyOf(one.artifact) === keyOf(artifact),
      );
      if (going !== undefined)
        return {
          ok: true,
          value: going,
        };
      made += 1;
      const reading: JudgeReading = {
        id: `judge-${made}`,
        artifact,
        prompt,
        state: "reading",
        asked_at: new Date().toISOString(),
        read: false,
        notes: [],
      };
      put(job, reading);
      const handle = window.setTimeout(() => {
        timers.delete(handle);
        const finished_at = new Date().toISOString();
        put(
          job,
          prompt === FAILS
            ? {
                ...reading,
                state: "failed",
                finished_at,
                failed: FAILED,
              }
            : {
                ...reading,
                state: "done",
                finished_at,
                notes: notesFor({
                  artifact,
                  prompt,
                }),
              },
        );
      }, READS_MS);
      timers.add(handle);
      return {
        ok: true,
        value: reading,
      };
    },
    sendJudgeNote: async ({ session_id: job, reading: id, note: noteId }) => {
      const reading = find(job, id);
      if (reading === undefined) return missing();
      const found = reading.notes.find((one) => one.id === noteId);
      if (found === undefined)
        return refused("fleet.no_such_judge_note", "No note of this reading has that id.");
      if (found.settled !== undefined)
        return {
          ok: true,
          value: reading,
        };
      const next = settle(job, reading, noteId, "sent");
      await store.send(
        job,
        plain(
          `${reading.artifact.title}${found.where === undefined ? "" : `, ${found.where}`}: ${found.claim}`,
        ),
      );
      return {
        ok: true,
        value: next,
      };
    },
    dismissJudgeNote: async ({ session_id: job, reading: id, note: noteId }) => {
      const reading = find(job, id);
      if (reading === undefined) return missing();
      if (!reading.notes.some((one) => one.id === noteId))
        return refused("fleet.no_such_judge_note", "No note of this reading has that id.");
      return {
        ok: true,
        value: settle(job, reading, noteId, "dismissed"),
      };
    },
    markJudgeRead: async ({ session_id: job, reading: id }) => {
      const reading = find(job, id);
      if (reading === undefined) return missing();
      const next = {
        ...reading,
        read: true,
      };
      put(job, next);
      return {
        ok: true,
        value: next,
      };
    },
  };
  return {
    routes,
    acts: {
      ask: (job, artifact, prompt) =>
        void routes.askJudge({
          session_id: job,
          artifact,
          prompt,
        }),
      send: (job, reading, noteId) =>
        void routes.sendJudgeNote({
          session_id: job,
          reading,
          note: noteId,
        }),
      dismiss: (job, reading, noteId) =>
        void routes.dismissJudgeNote({
          session_id: job,
          reading,
          note: noteId,
        }),
      markRead: (job, reading) =>
        void routes.markJudgeRead({
          session_id: job,
          reading,
        }),
    },
    dispose: () => timers.forEach((one) => window.clearTimeout(one)),
  };
}
