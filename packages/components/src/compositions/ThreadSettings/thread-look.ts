import { createContext, useContext } from "react";

import { DEFAULT_RESPONSES } from "../../primitives/Prose/responses";
import type { QuickResponse } from "../../primitives/Prose/responses";

/** How a Job's thread reads: the agent's name, the stamps, the text size, and the replies a press sends. */
export type ThreadLook = {
  /** What the agent's rows are labelled. Empty reads `Agent`. */
  agentName: string;
  showTimes: boolean;
  showAgentName: boolean;
  textSize: ThreadTextSize;
  /** The quick bar's own lines, in the order it draws them. */
  quick: readonly QuickPhrase[];
  /** Lines he sends often join the quick bar as learnt. */
  learn: boolean;
  /** Learnt lines he deleted or reworded, so Bridge does not learn them again. */
  forgotten: readonly string[];
  voice: VoiceEngine;
  /** The quick bar is not drawn in any Job. */
  quickOff: boolean;
  /** The quick bar is not drawn in these Jobs, by id. */
  quickOffFor: readonly string[];
  /** The quick responses on a line of an agent's message, in the order the bar draws them. */
  responses: readonly QuickResponse[];
};

/** A line the quick bar offers: one he saved, or one Bridge learnt from what he sends. */
export type QuickPhrase = { id: string; text: string; kind: "saved" | "learnt"; /** Sits first in the bar, after the answers to the agent's question. Only a saved line is pinned. */ pinned?: boolean };

/** What he does to a quick reply, from its menu or from the settings list. */
export type QuickAct =
  | { act: "edit"; id: string; text: string }
  | { act: "delete"; id: string }
  | { act: "pin"; id: string }
  | { act: "unpin"; id: string }
  /** An answer to the agent's question, kept as a saved line. */
  | { act: "save"; text: string };

/** Whether the bar is drawn in a Job: not while it is off for all Jobs, nor for this one. */
export function quickShownIn(look: ThreadLook, jobId: string | undefined): boolean {
  return !look.quickOff && (jobId === undefined || !look.quickOffFor.includes(jobId));
}

/** The lines as the bar draws them: pinned first, each group in the order it is kept. */
export function pinnedFirst<T extends { pinned?: boolean }>(lines: readonly T[]): T[] {
  return [...lines.filter((one) => one.pinned === true), ...lines.filter((one) => one.pinned !== true)];
}

/**
 * The look after one act on a quick reply. **A learnt line that is edited, pinned or deleted leaves the learnt set**: edited
 * and pinned ones become saved, and the old words of an edited or deleted one are forgotten, so they are not learnt again.
 * Pinning puts the line last among the pinned, which is the order they were pinned in.
 */
export function actOnQuick(look: ThreadLook, act: QuickAct): ThreadLook {
  if (act.act === "save") {
    const text = act.text.trim();
    if (text === "" || look.quick.some((one) => one.text === text)) return look;
    return { ...look, quick: [...look.quick, { id: `saved:${Date.now()}`, text, kind: "saved" }] };
  }
  const was = look.quick.find((one) => one.id === act.id);
  if (was === undefined) return look;
  const forget = was.kind === "learnt" && !look.forgotten.includes(was.text) ? [...look.forgotten, was.text] : look.forgotten;
  switch (act.act) {
    case "edit": {
      const text = act.text.trim();
      if (text === "" || text === was.text) return look;
      return { ...look, forgotten: forget, quick: look.quick.map((one) => (one.id === was.id ? { ...one, text, kind: "saved" } : one)) };
    }
    case "delete":
      return { ...look, forgotten: forget, quick: look.quick.filter((one) => one.id !== was.id) };
    case "pin": {
      if (was.pinned === true) return look;
      const rest = look.quick.filter((one) => one.id !== was.id);
      const at = rest.filter((one) => one.pinned === true).length;
      return { ...look, quick: [...rest.slice(0, at), { ...was, kind: "saved", pinned: true }, ...rest.slice(at)] };
    }
    case "unpin": {
      const { pinned: _, ...unpinned } = was;
      return { ...look, quick: look.quick.map((one) => (one.id === was.id ? unpinned : one)) };
    }
  }
}

/** The body sizes a thread can be read at, each a type token's (`typography.css`). `base` is what a message is drawn at today. */
export const THREAD_TEXT_SIZES = [
  { size: "xs", said: "13 px" },
  { size: "sm", said: "14 px" },
  { size: "base", said: "15 px" },
  { size: "lg", said: "18 px" },
] as const;

export type ThreadTextSize = (typeof THREAD_TEXT_SIZES)[number]["size"];

/** Where Talk's words come from. */
export type VoiceEngine = "apple" | "whisper" | "dictation";

export const VOICE_ENGINES: readonly { id: VoiceEngine; said: string }[] = [
  { id: "apple", said: "Apple on-device" },
  { id: "whisper", said: "Whisper" },
  { id: "dictation", said: "macOS Dictation" },
];

export const DEFAULT_LOOK: ThreadLook = { agentName: "", showTimes: true, showAgentName: true, textSize: "base", quick: [], learn: true, forgotten: [], voice: "apple", quickOff: false, quickOffFor: [], responses: DEFAULT_RESPONSES };

/** `one-bar`: a question's options are drawn in the band as quick bar chips and the bar over the box draws nothing. `replace`: the bar is not drawn while a band is up. */
export type WithBand = "one-bar" | "replace";

/** Whisper's model on this Mac: not here, coming down (`progress` 0 to 1), or here. */
export type WhisperModel = { state: "absent" } | { state: "installing"; progress: number } | { state: "installed" };

/**
 * What the window holds for the thread's look, and what serves the quick bar and Talk. **Absent, nothing new is drawn**: no Aa,
 * no quick bar, no Talk, and the thread reads as it always has.
 */
export type ThreadLookSource = {
  look: ThreadLook;
  setLook: (change: (was: ThreadLook) => ThreadLook) => void;
  /** Answers to the agent's question, for a Job whose last message asks one. */
  askedFor: (sessionId: string) => readonly string[];
  /** Mock only, a comparison: how the Needs you band and the quick bar share a question. Absent, both are drawn as they were. */
  withBand?: WithBand;
  /** Hears words, one at a time, until the returned stop is called. */
  listen: (onWord: (word: string) => void) => () => void;
  whisper: WhisperModel;
  installWhisper: () => void;
};

export const ThreadLookContext = createContext<ThreadLookSource | null>(null);

export const useThreadLook = (): ThreadLookSource | null => useContext(ThreadLookContext);
