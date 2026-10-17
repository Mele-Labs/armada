// Dispatch session on a failure toast: a Session started in Armada's own repository whose first message is the
// failure's debug info and the window as it stood (`failure-message.ts`), with a picture of it where the annotation
// layer can take one. It starts the way a Session's Report does (`draft.start` in `reportRoot`, then `draft.send`), and
// opens it the way every hand-off does.
// A start Fleet refused is said as a toast, since the toast that offered it may be over a page that shows no refusal.
// The owner's ask of 15 Oct 2026.

import { useRef } from "react";
import type { DebugPayload } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";

import type { WindowFacts } from "../../shared/annotations";
import { captureWindow, windowBox } from "./annotate/capture";
import { contextOf, type ContextState } from "./annotate/context";
import { withoutLayer } from "./annotate/shoot";
import { plain, reportRoot } from "./cockpit/deck/session-tiles";
import { failureMessage, type PressedState } from "./failure-message";
import { useSessionsDraft } from "./sessions-draft";
import { askToTell } from "./tell";

/** The window as the whole-window note records it, read now. */
function windowNow(state: ContextState): WindowFacts {
  const { screen, layer, location, scenario, window: size } = captureWindow(new Date());
  let context: WindowFacts["context"];
  try {
    context = contextOf(state);
  } catch {
    // A state read half-published is no reason to lose the rest.
    context = undefined;
  }
  return { screen, layer, location, scenario, window: size, ...(context === undefined ? {} : { context }) };
}

/** A picture of the window with the layer out of it, or none: only a build with the annotation layer can capture, and a capture that fails sends without. */
async function windowShot(): Promise<File | undefined> {
  const dev = window.armadaDev;
  if (dev === undefined) return undefined;
  const png = await withoutLayer(document.querySelector(".armada-annotate"), () => dev.capture(windowBox())).catch(() => null);
  return png === null ? undefined : new File([png], "bridge-window.png", { type: "image/png" });
}

/**
 * What a failure toast calls to hand its debug info to a new Session, or `undefined` where it cannot: nothing serves
 * Sessions, or Fleet serves no Armada repository to start one in. It resolves to whether the Session was started and told.
 */
export function useDispatchFailure(
  repositories: readonly RepositorySummary[],
  open: (id: string) => void,
  state: ContextState & PressedState,
): ((payload: DebugPayload) => Promise<boolean>) | undefined {
  const draft = useSessionsDraft();
  /** The Session a press started, kept while its message is refused so a second press tells the same one. */
  const started = useRef<string | undefined>(undefined);
  const armada = reportRoot(repositories, undefined);
  if (draft === undefined || armada === undefined) return undefined;
  return async (payload: DebugPayload): Promise<boolean> => {
    // Read at the press, before the Session starts and the window moves on to it.
    const view = windowNow(state);
    const shot = await windowShot();
    const id = started.current ?? (await draft.start(undefined, armada));
    if (id === undefined) {
      const why = draft.said?.();
      askToTell(why === undefined ? "Job not started" : `Job not started: ${why}`);
      return false;
    }
    started.current = id;
    const files = shot === undefined ? [] : [{ id: "failure-window", name: shot.name, file: shot, src: URL.createObjectURL(shot) }];
    if ((await draft.send(id, { ...plain(failureMessage(payload, view, state)), files })) === false) return false;
    started.current = undefined;
    open(id);
    return true;
  };
}
