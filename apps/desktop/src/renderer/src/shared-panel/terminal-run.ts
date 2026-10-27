// One command run from the panel, in a directory with no Job behind it. Small and the panel's own: a Job's shell is kept by session
// (`job-shell.ts`), and this run has none. Output arrives by run id, possibly before Fleet's answer names the id, so it is held until then.

import { useCallback, useEffect, useRef, useState } from "react";
import type { SessionRunMessage } from "@armada/protocol";
import { refusalWords } from "@armada/screens/src/refusal-words";

import type { SessionsApi } from "../../../shared/api/sessions";

export type TerminalRunApi = Pick<SessionsApi, "startTerminalRun" | "writeSessionRun" | "resizeSessionRun" | "stopSessionRun" | "onSessionRunMessage">;

type Run = { runId: string | null; out: string; screen: number; running: boolean };

export type TerminalRun = {
  run: Run | undefined;
  /** Fleet's refusal to run it. */
  said: string | undefined;
  start: (command: string) => void;
  write: (data: string) => void;
  sized: (cols: number, rows: number) => void;
  stop: () => void;
};

export function useTerminalRun(api: TerminalRunApi | undefined, dir: string | undefined): TerminalRun {
  const [run, setRun] = useState<Run | undefined>(undefined);
  const [said, setSaid] = useState<string | undefined>(undefined);
  const now = useRef<Run | undefined>(undefined);
  const early = useRef<{ runId: string; message: SessionRunMessage }[]>([]);
  const size = useRef<{ cols: number; rows: number } | undefined>(undefined);
  const screens = useRef(0);
  const put = useCallback((next: Run | undefined) => {
    now.current = next;
    setRun(next);
  }, []);

  const apply = useCallback(
    (runId: string, message: SessionRunMessage) => {
      const was = now.current;
      if (was === undefined || was.runId !== runId) return;
      if (message.kind === "output" && message.text !== "") put({ ...was, out: was.out + message.text });
      else if (message.kind === "exited") put({ ...was, running: false });
    },
    [put],
  );

  useEffect(() => {
    if (api === undefined) return;
    const off = api.onSessionRunMessage((runId, message) => {
      if (now.current?.runId === runId) apply(runId, message);
      else if (now.current?.runId === null) early.current.push({ runId, message });
    });
    return () => {
      off();
      const runId = now.current?.runId;
      // A run nobody can reach again is stopped with the panel that started it.
      if (runId !== null && runId !== undefined && now.current?.running) void api.stopSessionRun(runId);
    };
  }, [api, apply]);

  const start = (command: string) => {
    if (api === undefined || dir === undefined || now.current?.running) return;
    setSaid(undefined);
    early.current = [];
    put({ runId: null, out: "", screen: (screens.current += 1), running: true });
    const mine = now.current!.screen;
    void api.startTerminalRun(command, dir).then((answer) => {
      if (now.current?.screen !== mine) return;
      if (!answer.ok) {
        put(undefined);
        return setSaid(refusalWords(answer.outcome));
      }
      const runId = answer.value.run_id;
      put({ ...now.current, runId });
      const held = early.current.filter((one) => one.runId === runId);
      early.current = [];
      for (const one of held) apply(runId, one.message);
      if (size.current !== undefined) void api.resizeSessionRun(runId, size.current.cols, size.current.rows);
    });
  };

  const write = (data: string) => {
    const runId = now.current?.runId;
    if (api !== undefined && runId !== null && runId !== undefined && now.current?.running) void api.writeSessionRun(runId, data);
  };
  const sized = (cols: number, rows: number) => {
    size.current = { cols, rows };
    const runId = now.current?.runId;
    if (api !== undefined && runId !== null && runId !== undefined && now.current?.running) void api.resizeSessionRun(runId, cols, rows);
  };
  const stop = () => {
    const runId = now.current?.runId;
    if (api !== undefined && runId !== null && runId !== undefined) void api.stopSessionRun(runId);
  };
  return { run, said, start, write, sized, stop };
}
