// What the Operator did with a dispatch he sent while it was on (`operator.dispatched`): he stayed where he was, so a toast names the Job
// it started or handed his words to, and a press opens it. Where it asked him instead, the toast names his words and the card is in Needs you.

import { useEffect, useState } from "react";
import { Toast } from "@armada/components";
import type { OperatorDispatch } from "@armada/screens/src/draft/sessions";

import { useSessionsDraft } from "./sessions-draft";

/** How long one stays up unpressed. */
const LINGER_MS = 8000;

function saying(one: OperatorDispatch): string {
  if (one.way === "started") return `Operator started ${one.session?.title ?? one.subject}`;
  if (one.way === "handed") return `Operator handed it to ${one.session?.title ?? "a live Job"}`;
  return `Operator asks about ${one.subject}`;
}

export function OperatorDispatchToasts({ open }: { open: (sessionId: string) => void }) {
  const draft = useSessionsDraft();
  const [shown, setShown] = useState<readonly OperatorDispatch[]>([]);
  useEffect(
    () =>
      draft?.onDispatched?.((one) => {
        setShown((was) => [
          ...was.filter((it) => it.id !== one.id),
          one,
        ]);
        window.setTimeout(() => setShown((was) => was.filter((it) => it !== one)), LINGER_MS);
      }),
    [
      draft,
    ],
  );
  return (
    <>
      {shown.map((one) => {
        const session = one.session;
        return (
          <Toast
            key={one.id}
            status={one.way === "asked" ? "awaiting-review" : "running"}
            {...(session === undefined
              ? {}
              : {
                  onPress: () => {
                    setShown((was) => was.filter((it) => it !== one));
                    open(session.id);
                  },
                })}
          >
            {saying(one)}
          </Toast>
        );
      })}
    </>
  );
}
