// The thread's look, the quick bar and Talk for a Job: what the Aa panel and System › Thread set, read where a Job is drawn.
// **Mock only for now.** A scenario hands a seed (`threadLook` on its draft) and the window holds it; a window with none holds no
// look, and a Job draws as it always has: no Aa, no quick bar, no Talk. Fleet keeps none of it yet, and Talk hears a script.

import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { ThreadLookButton, ThreadLookContext, useThreadLook } from "@armada/components";
import type { SessionComposerProps, SessionThreadProps, ThreadLook, ThreadLookSource, ThreadSpacing, WhisperModel } from "@armada/components";
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

/** Holds a scenario's look for the window. Without a seed nothing is held. */
export function ThreadLookFrom({ seed, children }: { seed: ThreadLookSeed | undefined; children: ReactNode }) {
  return seed === undefined ? <>{children}</> : <Held seed={seed}>{children}</Held>;
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
      listen: (onWord) => {
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

/** The agent's last message ends in a question, and nothing has been said since. */
function asking(session: Session): boolean {
  for (let at = session.rows.length - 1; at >= 0; at -= 1) {
    const row = session.rows[at]!;
    if (row.kind !== "message") continue;
    return row.from.kind === "agent" && row.text.trim().endsWith("?");
  }
  return false;
}

/**
 * What a Job's view hands its thread, its box and its head, where the window holds a look. Undefined where it holds none, and
 * the view passes nothing new.
 */
export function useThreadLookHere(session: Session): { thread: Pick<SessionThreadProps, "look" | "agentName">; composer: Pick<SessionComposerProps, "quick" | "talk">; head: ReactNode } | undefined {
  const source = useThreadLook();
  const density = useSetting("spacing.density");
  if (source === null) return undefined;
  const { look } = source;
  const spacing: ThreadSpacing = density === "compact" || density === "roomy" ? density : "default";
  const name = look.agentName.trim();
  return {
    thread: {
      look: { showTimes: look.showTimes, showAgentName: look.showAgentName, textSize: look.textSize },
      ...(name === "" ? {} : { agentName: name }),
    },
    composer: {
      quick: {
        asked: asking(session) ? source.askedFor(session.id) : [],
        // Learnt lines go while he has learning off; the saved ones are his.
        phrases: look.learn ? look.quick : look.quick.filter((one) => one.kind === "saved"),
        onSave: (text) => source.setLook((was) => (text === "" || was.quick.some((one) => one.text === text) ? was : { ...was, quick: [...was.quick, { id: `saved:${Date.now()}`, text, kind: "saved" }] })),
      },
      talk: { listen: source.listen },
    },
    head: <ThreadLookButton spacing={spacing} onSpacing={(next) => void saveSetting("spacing.density", next)} />,
  };
}
