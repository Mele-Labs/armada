// What the Operator did with a dispatch he sent while it was on (`operator.dispatched`): he stayed where he was, so a toast names the Job
// it started or handed his words to, and a press opens it. Where it asked him instead, the toast names his words and the card is in Needs you.

import { useEffect, useSyncExternalStore } from "react";
import { Toast } from "@armada/components";
import type { OperatorDispatch } from "@armada/screens/src/draft/sessions";

import { useSessionsDraft } from "./sessions-draft";

/** How long one stays up unpressed, from when the pointer last left it. */
const LINGER_MS = 12000;

// Held outside the component, so a redraw of the window that remounts the toast column does not drop one he is about to press.
let shown: readonly OperatorDispatch[] = [];
const listeners = new Set<() => void>();
const timers = new Map<string, number>();
const told = () => listeners.forEach((hear) => hear());
const subscribe = (listener: () => void) => {
  listeners.add(listener);
  return () => void listeners.delete(listener);
};
const drop = (one: OperatorDispatch) => {
  window.clearTimeout(timers.get(one.id));
  shown = shown.filter((it) => it !== one);
  told();
};
// Held while the pointer is on it, so a toast he is reaching for does not go from under him.
const hold = (one: OperatorDispatch) => window.clearTimeout(timers.get(one.id));
const linger = (one: OperatorDispatch) => {
  hold(one);
  timers.set(
    one.id,
    window.setTimeout(() => drop(one), LINGER_MS),
  );
};
const add = (one: OperatorDispatch) => {
  shown = [
    ...shown.filter((it) => it.id !== one.id),
    one,
  ];
  linger(one);
  told();
};

function saying(one: OperatorDispatch): string {
  if (one.way === "started") return `Operator started ${one.session?.title ?? one.subject}`;
  if (one.way === "handed") return `Operator handed it to ${one.session?.title ?? "a live Job"}`;
  return `Operator asks about ${one.subject}`;
}

export function OperatorDispatchToasts({ open }: { open: (sessionId: string) => void }) {
  const draft = useSessionsDraft();
  useEffect(
    () => draft?.onDispatched?.(add),
    [
      draft,
    ],
  );
  const now = useSyncExternalStore(subscribe, () => shown);
  return (
    <>
      {now.map((one) => {
        const session = one.session;
        return (
          <div key={one.id} onPointerEnter={() => hold(one)} onPointerLeave={() => linger(one)}>
            <Toast
              status={one.way === "asked" ? "awaiting-review" : "running"}
              {...(session === undefined
                ? {}
                : {
                    onPress: () => {
                      drop(one);
                      open(session.id);
                    },
                  })}
            >
              {saying(one)}
            </Toast>
          </div>
        );
      })}
    </>
  );
}
