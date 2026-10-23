// A Job's terminal window in the mock: Bridge's own window on the shell, drawn over the page because a browser has no
// second window, as the walk window is. **`TerminalWindow` itself, over a copy of the fake's api**: a window of its own has
// its own shell store in the real app, and one made here meets a shell that printed before it opened the way a real one does.
// The OS close is the Close window button; the shell keeps running and the fake reports the Job has no window.

import { useEffect, useMemo, useRef, useState } from "react";
import { Button } from "@armada/components";

import type { BridgeApi } from "../../../shared/api";
import { useSessions } from "../sessions-draft";
import { TerminalWindow, windowTitleOf } from "../terminal-window/TerminalWindow";
import { closeTerminalWindow, onTerminalWindowRaised } from "./scenarios/session-shell";
import "./mock.css";

function Frame({ api, sessionId, front, raised }: { api: BridgeApi; sessionId: string; front: boolean; raised: number }) {
  const title = windowTitleOf(useSessions().find((one) => one.id === sessionId)?.title);
  // A store of its own, as a window's document has.
  const own = useMemo(() => ({ ...api }), [api]);
  const frame = useRef<HTMLElement>(null);
  // The page's title comes back with the window, which sets the document's.
  const [was] = useState(() => document.title);
  useEffect(() => () => void (document.title = was), [was]);
  useEffect(() => {
    if (raised > 0) frame.current?.querySelector("textarea")?.focus({ preventScroll: true });
  }, [raised]);
  return (
    <section ref={frame} role="dialog" aria-label={title} className="armada-mock-terminal-window" data-front={front ? "" : undefined}>
      <div className="armada-mock-terminal-window__strip">
        <span>{title}</span>
        <Button variant="secondary" size="sm" onClick={() => closeTerminalWindow(sessionId)}>
          Close window
        </Button>
      </div>
      <div className="armada-mock-terminal-window__document">
        <TerminalWindow api={own} sessionId={sessionId} />
      </div>
    </section>
  );
}

/** The Jobs whose shell has a window of its own, each drawn as one, the last opened or raised in front. */
export function TerminalWindows({ api }: { api: BridgeApi }) {
  const [open, setOpen] = useState<readonly string[]>([]);
  const [front, setFront] = useState<string>();
  const [raised, setRaised] = useState<Readonly<Record<string, number>>>({});
  useEffect(() => {
    // A fake that left Sessions out serves no windows.
    if (typeof api.onTerminalWindows !== "function") return;
    let live = true;
    let known: readonly string[] = [];
    const hear = (sessionIds: readonly string[]) => {
      if (!live) return;
      const opened = sessionIds.filter((one) => !known.includes(one));
      known = sessionIds;
      if (opened.length > 0) setFront(opened[opened.length - 1]);
      setOpen(sessionIds);
    };
    let heard = false;
    const off = api.onTerminalWindows((sessionIds) => {
      heard = true;
      hear(sessionIds);
    });
    void api.getTerminalWindows().then((sessionIds) => heard || hear(sessionIds));
    const offRaised = onTerminalWindowRaised((sessionId) => {
      setFront(sessionId);
      setRaised((was) => ({ ...was, [sessionId]: (was[sessionId] ?? 0) + 1 }));
    });
    return () => {
      live = false;
      off();
      offRaised();
    };
  }, [api]);
  return (
    <>
      {open.map((sessionId) => (
        <Frame key={sessionId} api={api} sessionId={sessionId} front={front === sessionId} raised={raised[sessionId] ?? 0} />
      ))}
    </>
  );
}
