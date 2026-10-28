// The four keys on a Job's waiting cards: `⌥↵` yes, `⌥E` change, `⌥R` send back, `⌥X` no, one meaning on every card.
// `docs/contracts/design-system/keyboard-and-command-palette.md`, *Keys on a Job's thread cards*.
//
// **Each waiting card registers here, and the host draws one listener.** The cards sit in the thread and on the composer, which are
// siblings in the Job's view, so the provider wraps both. A card with nothing waiting registers nothing, and outside a provider a card
// draws as it always did.

import { createContext, useCallback, useContext, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { createPortal } from "react-dom";

import { Dialog } from "../../primitives/Dialog/Dialog";
import { Kbd } from "../../primitives/Kbd/Kbd";
import { Radio, RadioGroup } from "../../primitives/Radio/Radio";
import { keyFor } from "../../actions";
import { isPressed, useKeyBindings } from "../../keymap";

/** The four acts, by what they mean on any card. */
export type ThreadAct = "yes" | "change" | "sendBack" | "no";

/** The registry act each answers to. */
export const THREAD_ACT_ID: Readonly<Record<ThreadAct, string>> = { yes: "thread_yes", change: "thread_change", sendBack: "thread_send_back", no: "thread_no" };

/** What a card does for an act, and the word its button reads, which the confirm dialog names it by. */
export type ThreadActRun = { verb: string; run: () => void };

/** One waiting card: what kind it is and what it is, as the confirm dialog lists it, and the acts it has. */
export type ThreadCardKeys = { id: string; kind: string; title: string; acts: Partial<Record<ThreadAct, ThreadActRun>> };

type Held = { kind: string; title: string; acts: ThreadCardKeys["acts"] };

type Keys = {
  register: (id: string, held: Held) => void;
  drop: (id: string) => void;
  place: (id: string, element: HTMLElement | null) => void;
  target: string | undefined;
  /** Some waiting card has a yes, so `⌥↵` is the thread's. */
  yes: boolean;
};

const ThreadKeysContext = createContext<Keys | null>(null);

/** The cards in the order they are drawn, top to bottom, which is oldest to newest. */
function drawnOrder(ids: readonly string[], places: ReadonlyMap<string, HTMLElement>): string[] {
  return ids
    .filter((id) => places.get(id)?.isConnected === true)
    .sort((a, b) => (places.get(a)!.compareDocumentPosition(places.get(b)!) & Node.DOCUMENT_POSITION_FOLLOWING ? -1 : 1));
}

const LAYER = '[role="dialog"], [role="alertdialog"]';

type Confirming = { act: "yes" | "no"; ids: readonly string[]; picked: number; back: Element | null };

/**
 * The Job's cards and the listener for their keys. Wraps the thread and the composer of one Job. The target is the waiting card that
 * holds focus, otherwise the newest; `⌥↵` and `⌥X` always confirm in a dialog, `⌥E` and `⌥R` open the target's own box.
 */
export function ThreadKeys({ children }: { children: ReactNode }) {
  const cards = useRef(new Map<string, Held>());
  const places = useRef(new Map<string, HTMLElement>());
  // Moves whenever a card comes or goes, so the order below is read again.
  const [, setVersion] = useState(0);
  const [focused, setFocused] = useState<string>();
  const [confirming, setConfirming] = useState<Confirming>();
  const bump = useCallback(() => setVersion((was) => was + 1), []);

  const register = useCallback(
    (id: string, held: Held) => {
      const was = cards.current.get(id);
      cards.current.set(id, held);
      // The handlers change every render; only a change of what the dialog lists or which acts exist draws again.
      if (was === undefined || was.kind !== held.kind || was.title !== held.title || Object.keys(was.acts).join() !== Object.keys(held.acts).join()) bump();
    },
    [bump],
  );
  const drop = useCallback(
    (id: string) => {
      if (cards.current.delete(id)) bump();
    },
    [bump],
  );
  const place = useCallback((id: string, element: HTMLElement | null) => {
    if (element === null) places.current.delete(id);
    else places.current.set(id, element);
  }, []);

  const order = drawnOrder([...cards.current.keys()], places.current);
  const target = focused !== undefined && order.includes(focused) ? focused : order[order.length - 1];

  // Focus inside a card makes it the target; focus anywhere else gives the target back to the newest. A layer over the Job keeps it.
  useEffect(() => {
    const on = (event: FocusEvent) => {
      const at = event.target;
      if (!(at instanceof Element) || at.closest(LAYER) !== null) return;
      for (const [id, element] of places.current) if (element.contains(at) && cards.current.has(id)) return setFocused(id);
      setFocused(undefined);
    };
    document.addEventListener("focusin", on);
    return () => document.removeEventListener("focusin", on);
  }, []);

  const now = useRef({ order, target });
  now.current = { order, target };
  useEffect(() => {
    if (confirming !== undefined) return;
    const listen = (event: KeyboardEvent) => {
      if (event.defaultPrevented || document.querySelector(LAYER) !== null) return;
      const act: ThreadAct | undefined = (Object.keys(THREAD_ACT_ID) as ThreadAct[]).find((one) => isPressed(THREAD_ACT_ID[one], event));
      if (act === undefined) return;
      const { order: ids, target: on } = now.current;
      if (act === "yes" || act === "no") {
        const having = ids.filter((id) => cards.current.get(id)?.acts[act] !== undefined);
        if (having.length === 0) return;
        event.preventDefault();
        const at = on === undefined ? -1 : having.indexOf(on);
        setConfirming({ act, ids: having, picked: at < 0 ? having.length - 1 : at, back: document.activeElement });
        return;
      }
      const run = on === undefined ? undefined : cards.current.get(on)?.acts[act];
      if (run === undefined) return;
      event.preventDefault();
      run.run();
    };
    window.addEventListener("keydown", listen);
    return () => window.removeEventListener("keydown", listen);
  }, [confirming]);

  const yes = order.some((id) => cards.current.get(id)?.acts.yes !== undefined);
  const keys = useMemo(() => ({ register, drop, place, target, yes }), [register, drop, place, target, yes]);
  const close = (run?: ThreadActRun) => {
    const back = confirming?.back;
    setConfirming(undefined);
    run?.run();
    // Focus goes back where the key was pressed, so the target stays the card it was.
    if (back instanceof HTMLElement && back.isConnected) back.focus();
  };
  return (
    <ThreadKeysContext.Provider value={keys}>
      {children}
      {confirming === undefined ? null : createPortal(<ThreadConfirm confirming={confirming} cards={cards.current} onPick={(picked) => setConfirming({ ...confirming, picked })} onClose={close} />, document.body)}
    </ThreadKeysContext.Provider>
  );
}

/** Which card a yes or a no goes to: every waiting card that has it, one picked. `⌥↵` or Enter confirms, `1`–`9` picks, Esc cancels. */
function ThreadConfirm({ confirming, cards, onPick, onClose }: { confirming: Confirming; cards: ReadonlyMap<string, Held>; onPick: (at: number) => void; onClose: (run?: ThreadActRun) => void }) {
  const rows = confirming.ids.flatMap((id) => {
    const card = cards.get(id);
    const run = card?.acts[confirming.act];
    return card === undefined || run === undefined ? [] : [{ id, card, run }];
  });
  const picked = rows[Math.min(confirming.picked, rows.length - 1)];
  const confirm = () => onClose(picked?.run);
  const live = useRef({ confirm, onPick, count: rows.length });
  live.current = { confirm, onPick, count: rows.length };
  useEffect(() => {
    // Before the page's own keys: a digit or `⌥↵` here is the dialog's alone.
    const listen = (event: KeyboardEvent) => {
      const plain = !event.metaKey && !event.ctrlKey && !event.altKey && !event.shiftKey;
      const digit = plain && /^[1-9]$/.test(event.key) ? Number(event.key) : 0;
      const onRadio = event.target instanceof HTMLInputElement && event.target.type === "radio";
      if (isPressed("thread_yes", event) || (plain && event.key === "Enter" && onRadio)) {
        event.preventDefault();
        event.stopPropagation();
        live.current.confirm();
      } else if (digit > 0) {
        event.preventDefault();
        event.stopPropagation();
        if (digit <= live.current.count) live.current.onPick(digit - 1);
      }
    };
    window.addEventListener("keydown", listen, true);
    return () => window.removeEventListener("keydown", listen, true);
  }, []);
  // Every card it listed was answered some other way meanwhile.
  const gone = picked === undefined;
  useEffect(() => {
    if (gone) onClose();
  }, [gone, onClose]);
  if (picked === undefined) return null;
  return (
    <Dialog open title={`${picked.run.verb} which?`} tone="neutral" confirmLabel={picked.run.verb} initialFocus="confirm" onConfirm={confirm} onCancel={() => onClose()}>
      <RadioGroup label="Waiting cards">
        {rows.map((row, at) => (
          <Radio key={row.id} name="thread-confirm" checked={row === picked} onChange={() => onPick(at)}>
            <span className="armada-thread-confirm__row">
              {at < 9 ? <Kbd aria-hidden>{at + 1}</Kbd> : <span aria-hidden />}
              <span className="armada-thread-confirm__title">
                {row.card.kind} · {row.card.title}
              </span>
            </span>
          </Radio>
        ))}
      </RadioGroup>
    </Dialog>
  );
}

/**
 * Registers a waiting card while `card` is given, and says whether it is the target. `bind` goes on the card's own element: it marks
 * the target's ring and lets a press anywhere on the card take focus, which makes it the target. Outside `ThreadKeys` it does nothing.
 */
export function useThreadCard(card: ThreadCardKeys | undefined): { target: boolean; bind: { ref: (element: HTMLElement | null) => void; tabIndex?: number; "data-thread-target"?: "" } } {
  const keys = useContext(ThreadKeysContext);
  useKeyBindings();
  const id = card?.id;
  const register = keys?.register;
  const drop = keys?.drop;
  useLayoutEffect(() => {
    if (card === undefined || register === undefined) return;
    register(card.id, { kind: card.kind, title: card.title, acts: card.acts });
  });
  useLayoutEffect(() => {
    if (id === undefined || drop === undefined) return;
    return () => drop(id);
  }, [id, drop]);
  const place = keys?.place;
  const ref = useCallback((element: HTMLElement | null) => {
    if (id !== undefined) place?.(id, element);
  }, [id, place]);
  const target = keys !== null && id !== undefined && keys.target === id;
  if (keys === null || id === undefined) return { target: false, bind: { ref } };
  return { target, bind: { ref, tabIndex: -1, ...(target ? { "data-thread-target": "" as const } : {}) } };
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
