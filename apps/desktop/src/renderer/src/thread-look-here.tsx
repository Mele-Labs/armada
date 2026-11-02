// The thread's look, the quick bar and Talk for a Job: what the Aa panel and System › Thread set, read where a Job is drawn.
// The window always holds a look, from one copy Fleet keeps under `thread.look` (`ThreadLookSaved`). A mock scenario may hand a
// seed (`threadLook` on its draft), which holds in place of the saved copy. Talk hears a script there and this Mac's voice in the live app.

import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { DEFAULT_LOOK, ThreadLookButton, ThreadLookContext, actOnQuick, actOnResponses, quickShownIn, responseMessage, useThreadLook } from "@armada/components";
import type { QuickPhrase, QuickResponse, SessionComposerProps, SessionThreadProps, TalkPane, TalkWhy, ThreadLook, ThreadLookSource, WhisperModel } from "@armada/components";
import type { SessionQuick, SettingValue } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import { saveSetting, useSetting } from "./use-setting";


/** What a scenario starts the look with, the answers each Job's question offers, and the words Talk hears. */
export type ThreadLookSeed = {
  look: ThreadLook;
  /** By Job id: the answers to its agent's question, offered while its last message asks one. */
  asked: Readonly<Record<string, readonly string[]>>;
  /** What Talk hears, a word at a time. */
  heard: string;
};

/** How long Talk takes between words, and how long Whisper's model takes to come down. */
const WORD_MS = 240;
const INSTALL_MS = 3200;
const INSTALL_STEP_MS = 160;

/** Holds a scenario's look for the window. Without a seed it is the saved copy, as in the live app. */
export function ThreadLookFrom({ seed, children }: { seed: ThreadLookSeed | undefined; children: ReactNode }) {
  return seed === undefined ? <ThreadLookSaved>{children}</ThreadLookSaved> : <Held seed={seed}>{children}</Held>;
}

const KEY = "thread.look";
/** How long a change waits for the next before it is saved, so typing a name is one write. */
const SAVE_MS = 400;

/** The saved copy laid over the defaults: a field it lacks, or holds as another kind, is the default's. */
export function lookFrom(saved: SettingValue | undefined): ThreadLook {
  if (typeof saved !== "object" || saved === null || Array.isArray(saved)) return DEFAULT_LOOK;
  const got = saved as Record<string, unknown>;
  const fits = (key: keyof ThreadLook) => typeof got[key] === typeof DEFAULT_LOOK[key] && Array.isArray(got[key]) === Array.isArray(DEFAULT_LOOK[key]) && got[key] !== null;
  return Object.fromEntries(Object.entries(DEFAULT_LOOK).map(([key, was]) => [key, fits(key as keyof ThreadLook) ? got[key] : was])) as ThreadLook;
}

/**
 * The look from `thread.look` in settings.json: drawn the moment it changes and saved a beat later, and a save Fleet refuses puts
 * back what Fleet has. Talk hears through this Mac's voice engines (`main/voice.ts`) where the preload offers them, and is not
 * drawn where it does not.
 */
export function ThreadLookSaved({ children }: { children: ReactNode }) {
  const saved = useSetting(KEY);
  const voice = window.armada.voice;
  const [whisper, setWhisper] = useState<WhisperModel>({ state: "absent" });
  useEffect(() => {
    if (voice === undefined) return undefined;
    void voice.whisper().then(setWhisper);
    return voice.onWhisper(setWhisper);
  }, [voice]);
  // What Fleet reads for each Job's bar (`GET /sessions/quick`), and the last message it was asked for.
  const [quick, setQuick] = useState<Readonly<Record<string, { message: string; read: SessionQuick }>>>({});
  const asked = useRef(new Map<string, string>());
  const wantQuick = useCallback((sessionId: string, message: string, settled: string) => {
    const key = `${message}|${settled}`;
    if (asked.current.get(sessionId) === key) return;
    asked.current.set(sessionId, key);
    void window.armada.readSessionQuick(sessionId).then((read) => {
      if (asked.current.get(sessionId) !== key) return;
      // A read that failed is asked again the next time the Job's last message or the look moves.
      if (!read.ok) return void asked.current.delete(sessionId);
      setQuick((was) => ({ ...was, [sessionId]: { message, read: read.value } }));
    });
  }, []);
  const kept = useMemo(() => lookFrom(saved), [saved]);
  const [look, hold] = useState(kept);
  const latest = useRef(look);
  const pending = useRef<number | undefined>(undefined);
  // Saves sent and not answered: Fleet's copy is older than the window's while any is out, so it is not taken back.
  const out = useRef(0);
  useEffect(() => {
    if (out.current > 0 || pending.current !== undefined || JSON.stringify(kept) === JSON.stringify(latest.current)) return;
    latest.current = kept;
    hold(kept);
  }, [kept]);
  useEffect(() => () => window.clearTimeout(pending.current), []);
  const source = useMemo(
    (): ThreadLookSource => ({
      look,
      setLook: (change) => {
        const next = change(latest.current);
        if (next === latest.current) return;
        latest.current = next;
        hold(next);
        window.clearTimeout(pending.current);
        pending.current = window.setTimeout(() => {
          pending.current = undefined;
          out.current += 1;
          void saveSetting(KEY, next as unknown as SettingValue).then((outcome) => {
            out.current -= 1;
            if (outcome.ok || out.current > 0 || pending.current !== undefined) return;
            latest.current = kept;
            hold(kept);
          });
        }, SAVE_MS);
      },
      // The answers belong to the message they were written for, so a newer one is not drawn over with the last one's.
      askedFor: (id, lastMessage) => (lastMessage === undefined || quick[id]?.message === lastMessage ? (quick[id]?.read.asked ?? []) : []),
      learntFor: (id) => quick[id]?.read.learnt ?? [],
      wantQuick,
      ...(voice === undefined
        ? {}
        : {
            listen: (onWord: (word: string) => void, onEnd?: (why?: TalkWhy) => void) => voice.listen(latest.current.voice, onWord, onEnd),
            openSettings: (pane: TalkPane) => voice.openSettings(pane),
            whisper,
            installWhisper: () => void voice.installWhisper(),
          }),
    }),
    [look, kept, voice, whisper, quick, wantQuick],
  );
  return <ThreadLookContext.Provider value={source}>{children}</ThreadLookContext.Provider>;
}

function Held({ seed, children }: { seed: ThreadLookSeed; children: ReactNode }) {
  const [look, setLook] = useState(seed.look);
  const [whisper, setWhisper] = useState<WhisperModel>({ state: "absent" });
  const installing = useRef<number | undefined>(undefined);
  useEffect(() => () => window.clearInterval(installing.current), []);
  const source = useMemo(
    (): ThreadLookSource => ({
      look,
      setLook,
      askedFor: (id) => seed.asked[id] ?? [],
      listen: (onWord, onEnd) => {
        // A seed with no words plays a refusal: the microphone is off, as on a Mac that has not allowed it.
        if (seed.heard === "") {
          const refused = window.setTimeout(() => onEnd?.("microphone_denied"), WORD_MS);
          return () => window.clearTimeout(refused);
        }
        const words = seed.heard.split(/\s+/).filter((one) => one !== "");
        let at = 0;
        const timer = window.setInterval(() => {
          const word = words[at];
          at += 1;
          if (word === undefined) window.clearInterval(timer);
          else onWord(word);
        }, WORD_MS);
        return () => window.clearInterval(timer);
      },
      openSettings: () => undefined,
      whisper,
      installWhisper: () => {
        if (installing.current !== undefined) return;
        let done = 0;
        setWhisper({ state: "installing", progress: 0 });
        installing.current = window.setInterval(() => {
          done += INSTALL_STEP_MS;
          if (done < INSTALL_MS) return setWhisper({ state: "installing", progress: done / INSTALL_MS });
          window.clearInterval(installing.current);
          setWhisper({ state: "installed" });
        }, INSTALL_STEP_MS);
      },
    }),
    [look, whisper, seed],
  );
  return <ThreadLookContext.Provider value={source}>{children}</ThreadLookContext.Provider>;
}

/** The message that answers one line of the agent's: the line quoted, a blank line, then the response's emoji. The thread reads its marks back from this shape. */
export function pickedLine(text: string, response: QuickResponse): string {
  return responseMessage(text, response.emoji);
}

/** What an `ask` response puts in the message box instead of sending: the line quoted and a blank line to type on. */
export const askedLine = (text: string): string => `> ${text}\n\n`;

/** The agent spoke last, so its answers are still open. Which answers fit is the reader's call, not a trailing "?". */
function asking(session: Session): boolean {
  for (let at = session.rows.length - 1; at >= 0; at -= 1) {
    const row = session.rows[at]!;
    if (row.kind !== "message") continue;
    return row.from.kind === "agent";
  }
  return false;
}

/** The Job's last message, by its row: who said it and which. Fleet's lists move when it does. */
function lastMessage(session: Session): string {
  for (let at = session.rows.length - 1; at >= 0; at -= 1) {
    const row = session.rows[at]!;
    if (row.kind === "message") return `${row.from.kind}:${row.stableKey ?? row.id}`;
  }
  return "";
}

/** Lines are alike as Fleet reads them (`alike` in `crates/fleet/src/session_quick.rs`): case, spacing and trailing punctuation set aside. */
function alike(line: string): string {
  return line.split(/\s+/).filter((one) => one !== "").join(" ").toLowerCase().replace(/[\s!-\/:-@\[-`{-~…]+$/u, "");
}

/**
 * What a Job's view hands its thread, its box and its head, where the window holds a look. Undefined where it holds none, and
 * the view passes nothing new.
 */
export function useThreadLookHere(session: Session): { thread: Pick<SessionThreadProps, "look" | "agentName" | "responses" | "onResponses">; composer: Pick<SessionComposerProps, "quick" | "talk">; head: ReactNode } | undefined {
  const source = useThreadLook();
  // Fleet is asked again when the Job's last message moves, or the look moves a thing Fleet leaves lines out by.
  const shown = source !== null && quickShownIn(source.look, session.id);
  const learning = source?.look.learn === true;
  const left = (source?.look.quick.length ?? 0) + (source?.look.forgotten.length ?? 0);
  const last = lastMessage(session);
  const want = source?.wantQuick;
  useEffect(() => {
    if (shown) want?.(session.id, last, `${learning}|${left}`);
  }, [shown, want, session.id, last, learning, left]);
  if (source === null) return undefined;
  const { look } = source;
  const name = look.agentName.trim();
  const mine = look.learn ? look.quick : look.quick.filter((one) => one.kind === "saved");
  // Fleet's learnt lines come after his own, less the ones he has since kept, deleted or reworded.
  const held = new Set([...look.quick.map((one) => one.text), ...look.forgotten].map(alike));
  const learnt: QuickPhrase[] = !look.learn
    ? []
    : (source.learntFor?.(session.id) ?? []).filter((text) => !held.has(alike(text))).map((text) => ({ id: `learnt:${text}`, text, kind: "learnt" }));
  return {
    thread: {
      look: { showTimes: look.showTimes, showAgentName: look.showAgentName, textSize: look.textSize, spacing: look.spacing },
      ...(name === "" ? {} : { agentName: name }),
      responses: look.responses,
      onResponses: (act) => source.setLook((was) => ({ ...was, responses: actOnResponses(was.responses, act) })),
    },
    composer: {
      ...(!quickShownIn(look, session.id) ? {} : { quick: {
        asked: asking(session) ? source.askedFor(session.id, last) : [],
        // Learnt lines go while he has learning off; the saved ones are his.
        phrases: [...mine, ...learnt],
        // A line Fleet learnt is not in the look until he acts on it: keeping it is how a press on it takes hold.
        onAct: (act) => source.setLook((was) => actOnQuick(act.act === "save" || was.quick.some((one) => one.id === act.id) ? was : keptLearnt(was, learnt, act.id), act)),
        onSave: (text) => source.setLook((was) => (text === "" || was.quick.some((one) => one.text === text) ? was : { ...was, quick: [...was.quick, { id: `saved:${Date.now()}`, text, kind: "saved" }] })),
        onHide: (scope: "job" | "all") => source.setLook((was) => (scope === "all" ? { ...was, quickOff: true } : was.quickOffFor.includes(session.id) ? was : { ...was, quickOffFor: [...was.quickOffFor, session.id] })),
      } }),
      ...(source.listen === undefined ? {} : { talk: { listen: source.listen } }),
    },
    head: <ThreadLookButton jobId={session.id} />,
  };
}

/** The look with the learnt line `id` among its quick lines, so an act on it finds it. Unchanged where no such line is learnt. */
function keptLearnt(look: ThreadLook, learnt: readonly QuickPhrase[], id: string): ThreadLook {
  const line = learnt.find((one) => one.id === id);
  return line === undefined ? look : { ...look, quick: [...look.quick, line] };
}
