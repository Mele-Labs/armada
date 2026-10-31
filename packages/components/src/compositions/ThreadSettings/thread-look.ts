import { createContext, useContext } from "react";

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
  voice: VoiceEngine;
};

/** A line the quick bar offers: one he saved, or one Bridge learnt from what he sends. */
export type QuickPhrase = { id: string; text: string; kind: "saved" | "learnt" };

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

export const DEFAULT_LOOK: ThreadLook = { agentName: "", showTimes: true, showAgentName: true, textSize: "base", quick: [], learn: true, voice: "apple" };

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
  /** Hears words, one at a time, until the returned stop is called. */
  listen: (onWord: (word: string) => void) => () => void;
  whisper: WhisperModel;
  installWhisper: () => void;
};

export const ThreadLookContext = createContext<ThreadLookSource | null>(null);

export const useThreadLook = (): ThreadLookSource | null => useContext(ThreadLookContext);
