// A terminal's window in the mock: Bridge's own window on the shell, drawn over the page because a browser has no
// second window, as the walk window is. **`TerminalWindow` itself, over a copy of the fake's api**: a window of its own has
// its own shell store in the real app, and one made here meets a shell that printed before it opened the way a real one does.
// The OS close is the Close window button; the shell keeps running and the fake reports the Job has no window.

import { useEffect, useMemo, useRef, useState } from "react";
import { Button } from "@armada/components";

import type { BridgeApi } from "../../../shared/api";
import type { TerminalWindowId } from "../../../shared/api/sessions";
import { TerminalWindow, useTerminalName, windowTitleOf } from "../terminal-window/TerminalWindow";
import { useSessions } from "../sessions-draft";
import { closeTerminalWindow, onTerminalWindowRaised } from "./scenarios/session-shell";
import "./mock.css";

/** A window under one key: the Job's id, and the terminal's ID where it has one. */
const keyOf = ({ sessionId, terminal }: TerminalWindowId): string =>
  terminal === undefined ? sessionId : `${sessionId}#${terminal}`;

function Frame({
  api,
  window: { sessionId, terminal },
  front,
  raised,
}: {
  api: BridgeApi;
  window: TerminalWindowId;
  front: boolean;
  raised: number;
}) {
  // A store of its own, as a window's document has.
  const own = useMemo(
    () => ({
      ...api,
    }),
    [
      api,
    ],
  );
  const title = windowTitleOf(
    useTerminalName(own, sessionId, terminal),
    useSessions().find((one) => one.id === sessionId)?.title,
  );
  const frame = useRef<HTMLElement>(null);
  // The page's title comes back with the window, which sets the document's.
  const [was] = useState(() => document.title);
  useEffect(
    () => () => void (document.title = was),
    [
      was,
    ],
  );
  useEffect(() => {
    if (raised > 0)
      frame.current?.querySelector("textarea")?.focus({
        preventScroll: true,
      });
  }, [
    raised,
  ]);
  return (
    <section
      ref={frame}
      role="dialog"
      aria-label={title}
      className="armada-mock-terminal-window"
      data-front={front ? "" : undefined}
    >
      <div className="armada-mock-terminal-window__strip">
        <span>{title}</span>
        <Button
          variant="secondary"
          size="sm"
          onClick={() =>
            closeTerminalWindow({
              sessionId,
              ...(terminal === undefined
                ? {}
                : {
                    terminal,
                  }),
            })
          }
        >
          Close window
        </Button>
      </div>
      <div className="armada-mock-terminal-window__document">
        <TerminalWindow api={own} sessionId={sessionId} terminal={terminal} />
      </div>
    </section>
  );
}

/** The terminals that have a window of their own, each drawn as one, the last opened or raised in front. */
export function TerminalWindows({ api }: { api: BridgeApi }) {
  const [open, setOpen] = useState<readonly TerminalWindowId[]>([]);
  const [front, setFront] = useState<string>();
  const [raised, setRaised] = useState<Readonly<Record<string, number>>>({});
  useEffect(() => {
    // A fake that left Sessions out serves no windows.
    if (typeof api.onTerminalWindows !== "function") return;
    let live = true;
    let known: readonly string[] = [];
    const hear = (windows: readonly TerminalWindowId[]) => {
      if (!live) return;
      const keys = windows.map(keyOf);
      const opened = keys.filter((one) => !known.includes(one));
      known = keys;
      if (opened.length > 0) setFront(opened[opened.length - 1]);
      setOpen(windows);
    };
    let heard = false;
    const off = api.onTerminalWindows((windows) => {
      heard = true;
      hear(windows);
    });
    // A fake that left Sessions out answers a refusal, not a list.
    void api.getTerminalWindows().then((windows) => {
      if (!heard && Array.isArray(windows)) hear(windows);
    });
    const offRaised = onTerminalWindowRaised((id) => {
      setFront(keyOf(id));
      setRaised((was) => ({
        ...was,
        [keyOf(id)]: (was[keyOf(id)] ?? 0) + 1,
      }));
    });
    return () => {
      live = false;
      off();
      offRaised();
    };
  }, [
    api,
  ]);
  return (
    <>
      {open.map((one) => (
        <Frame
          key={keyOf(one)}
          api={api}
          window={one}
          front={front === keyOf(one)}
          raised={raised[keyOf(one)] ?? 0}
        />
      ))}
    </>
  );
}
