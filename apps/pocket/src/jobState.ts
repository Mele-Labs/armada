// One Job, read from the Mac that holds it and read again when that Mac's stream says it changed;
// and every reply the phone makes to it, each sent to that Mac.

import { useEffect, useState } from "react";
import type { AnswerSessionAsk, AnswerWaiting, ApprovePlan, SendPlanChanges, SendSessionMessage, SessionThread as WireThread } from "@armada/protocol";

import { call, getJson, refusal } from "./client";
import { loadLists, onChange } from "./fleet";
import type { Machine } from "./machines";

export type JobRead = { thread?: WireThread; error?: string };

const one = (id: string) => `/api/sessions/one?session_id=${encodeURIComponent(id)}`;

export function useJob(machine: Machine | undefined, id: string): [JobRead, () => void] {
  const [read, setRead] = useState<JobRead>({});
  const [epoch, setEpoch] = useState(0);
  useEffect(() => {
    if (machine === undefined) return;
    return onChange((origin, sessionId) => {
      if (origin === machine.origin && (sessionId === undefined || sessionId === id)) setEpoch((was) => was + 1);
    });
  }, [machine?.origin, id]);
  useEffect(() => {
    if (machine === undefined) return;
    let gone = false;
    getJson<WireThread>(machine, one(id)).then(
      (thread) => !gone && setRead({ thread }),
      (why) => !gone && setRead((was) => ({ ...was, error: refusal(why, machine) })),
    );
    return () => {
      gone = true;
    };
  }, [machine?.origin, machine?.deviceId, id, epoch]);
  return [read, () => setEpoch((was) => was + 1)];
}

export type Acts = {
  answerAsk: (body: AnswerSessionAsk) => Promise<void>;
  answerWaiting: (body: AnswerWaiting) => Promise<void>;
  approvePlan: (body: ApprovePlan) => Promise<void>;
  planChanges: (body: SendPlanChanges) => Promise<void>;
  message: (body: SendSessionMessage) => Promise<void>;
};

/** The replies, each posted to `machine` and followed by a read of its lists and of the Job. */
export function actsOn(machine: Machine, reread: () => void): Acts {
  const send = (path: string) => async (body: unknown) => {
    await call(machine, path, { method: "POST", body });
    reread();
    void loadLists(machine.origin);
  };
  return {
    answerAsk: send("/api/sessions/ask/answer"),
    answerWaiting: send("/api/sessions/waiting/answer"),
    approvePlan: send("/api/sessions/plan/approve"),
    planChanges: send("/api/sessions/plan/changes"),
    message: send("/api/sessions/message"),
  };
}

/** Runs a reply. Answers null when it went, else the sentence to show. */
export async function attempt(act: () => Promise<void>, machine?: Machine): Promise<string | null> {
  try {
    await act();
    return null;
  } catch (why) {
    return refusal(why, machine);
  }
}

export type Shown = { kind: "text"; path: string; text: string } | { kind: "image"; url: string } | { kind: "refused"; says: string };

/** A file the Job showed, from `/api/sessions/shown`: text, or a picture's bytes. */
export async function readShown(machine: Machine, id: string, path: string): Promise<Shown> {
  try {
    const response = await call(machine, `/api/sessions/shown?session_id=${encodeURIComponent(id)}&path=${encodeURIComponent(path)}`);
    if ((response.headers.get("Content-Type") ?? "").startsWith("image/")) return { kind: "image", url: URL.createObjectURL(await response.blob()) };
    const body = (await response.json()) as { path: string; text: string };
    return { kind: "text", path: body.path, text: body.text };
  } catch (why) {
    return { kind: "refused", says: refusal(why, machine) };
  }
}
