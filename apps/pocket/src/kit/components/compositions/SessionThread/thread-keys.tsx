// The four keys on a Job's waiting cards: `⌥↵` yes, `⌥E` change, `⌥R` send back, `⌥X` no, one meaning on every card.
// `docs/contracts/design-system/keyboard-and-command-palette.md`, *Keys on a Job's thread cards*.
//
// **Each waiting card registers here, and the host draws one listener.** The cards sit in the thread and on the composer, which are
// siblings in the Job's view, so the provider wraps both. A card with nothing waiting registers nothing, and outside a provider a card
// draws as it always did.

import { useContext, useLayoutEffect, useCallback, createContext } from "react";
import { useKeyBindings } from "../../keymap";
import { keyFor } from "../../actions";
import { Kbd } from "../../primitives/Kbd/Kbd";

/** The four acts, by what they mean on any card. */
export type ThreadAct = "yes" | "change" | "sendBack" | "no";

/** The registry act each answers to. */
export const THREAD_ACT_ID: Readonly<Record<ThreadAct, string>> = {
  yes: "thread_yes",
  change: "thread_change",
  sendBack: "thread_send_back",
  no: "thread_no",
};

/** What a card does for an act, and the word its button reads, which the confirm dialog names it by. */
export type ThreadActRun = {
  verb: string;
  run: () => void;
};

/** One waiting card: what kind it is and what it is, as the confirm dialog lists it, and the acts it has. */
export type ThreadCardKeys = {
  id: string;
  kind: string;
  title: string;
  acts: Partial<Record<ThreadAct, ThreadActRun>>;
};

type Held = {
  kind: string;
  title: string;
  acts: ThreadCardKeys["acts"];
};

type Keys = {
  register: (id: string, held: Held) => void;
  drop: (id: string) => void;
  place: (id: string, element: HTMLElement | null) => void;
  target: string | undefined;
  /** Some waiting card has a yes, so `⌥↵` is the thread's. */
  yes: boolean;
};

const ThreadKeysContext = createContext<Keys | null>(null);

/**
 * Registers a waiting card while `card` is given, and says whether it is the target. `bind` goes on the card's own element: it marks
 * the target's ring and lets a press anywhere on the card take focus, which makes it the target. Outside `ThreadKeys` it does nothing.
 */
export function useThreadCard(card: ThreadCardKeys | undefined): {
  target: boolean;
  bind: {
    ref: (element: HTMLElement | null) => void;
    tabIndex?: number;
    "data-thread-target"?: "";
  };
} {
  const keys = useContext(ThreadKeysContext);
  useKeyBindings();
  const id = card?.id;
  const register = keys?.register;
  const drop = keys?.drop;
  useLayoutEffect(() => {
    if (card === undefined || register === undefined) return;
    register(card.id, {
      kind: card.kind,
      title: card.title,
      acts: card.acts,
    });
  });
  useLayoutEffect(() => {
    if (id === undefined || drop === undefined) return;
    return () => drop(id);
  }, [
    id,
    drop,
  ]);
  const place = keys?.place;
  const ref = useCallback(
    (element: HTMLElement | null) => {
      if (id !== undefined) place?.(id, element);
    },
    [
      id,
      place,
    ],
  );
  const target = keys !== null && id !== undefined && keys.target === id;
  if (keys === null || id === undefined)
    return {
      target: false,
      bind: {
        ref,
      },
    };
  return {
    target,
    bind: {
      ref,
      tabIndex: -1,
      ...(target
        ? {
            "data-thread-target": "" as const,
          }
        : {}),
    },
  };
}

/** The key of one act, drawn on its button on the target card only, at rest. */
export function ThreadKbd({ act, on }: { act: ThreadAct; on: boolean }) {
  if (!on) return null;
  const key = keyFor(THREAD_ACT_ID[act]);
  return key === "" ? null : <Kbd aria-hidden>{key}</Kbd>;
}

/** Whether `⌥↵` belongs to a waiting card here, so a message box around it leaves the key alone rather than sending. */
export function useThreadYesWaits(): boolean {
  return useContext(ThreadKeysContext)?.yes === true;
}
