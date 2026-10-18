// A way for the annotation layer to raise a failure toast with a sentence, as `open-session.ts` does
// for opening a Session: a window event, because the layer must not import the app.

import { useEffect } from "react";

const TELL = "armada:tell";

/** The act a told refusal was for: its name, the request it sent, and what the surface showed when it was pressed. A failure sent to a Session names all three. */
export type RefusedAct = { act: string; request: Record<string, unknown>; card?: Record<string, unknown> };

/** What a send to a terminal Session that is not listening says: the act is the terminal's own. */
export const NOT_REACHABLE = "Not reachable: run /reload-plugins in that job";

export function askToTell(sentence: string, act?: RefusedAct): void {
  window.dispatchEvent(new CustomEvent<{ sentence: string; act?: RefusedAct }>(TELL, { detail: { sentence, ...(act === undefined ? {} : { act }) } }));
}

export function useTellAsked(tell: (sentence: string, act?: RefusedAct) => void): void {
  useEffect(() => {
    const heard = (event: Event): void => {
      const { sentence, act } = (event as CustomEvent<{ sentence: string; act?: RefusedAct }>).detail;
      tell(sentence, act);
    };
    window.addEventListener(TELL, heard);
    return () => window.removeEventListener(TELL, heard);
  }, [tell]);
}
