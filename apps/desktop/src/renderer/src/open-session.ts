// A way for the annotation layer, which has a root of its own beside the app's, to ask the app to
// open a Session, on its page or on the Cockpit. A window event, because the layer must not import the app.

import { useEffect } from "react";

const OPEN_SESSION = "armada:open-session";

export function askToOpenSession(id: string): void {
  window.dispatchEvent(new CustomEvent<string>(OPEN_SESSION, { detail: id }));
}

export function useOpenSessionAsked(open: (id: string) => void): void {
  useEffect(() => {
    const heard = (event: Event): void => open((event as CustomEvent<string>).detail);
    window.addEventListener(OPEN_SESSION, heard);
    return () => window.removeEventListener(OPEN_SESSION, heard);
  }, [open]);
}

const OPEN_SESSION_IN_COCKPIT = "armada:open-session-in-cockpit";

/** The Session opened on the Cockpit's deck rather than the Sessions page: where an annotation sent to it goes. */
export function askToOpenSessionInCockpit(id: string): void {
  window.dispatchEvent(new CustomEvent<string>(OPEN_SESSION_IN_COCKPIT, { detail: id }));
}

export function useOpenSessionInCockpitAsked(open: (id: string) => void): void {
  useEffect(() => {
    const heard = (event: Event): void => open((event as CustomEvent<string>).detail);
    window.addEventListener(OPEN_SESSION_IN_COCKPIT, heard);
    return () => window.removeEventListener(OPEN_SESSION_IN_COCKPIT, heard);
  }, [open]);
}

/** The attribute the Sessions page carries the open Session's id on, as Job detail carries `data-armada-open-job`. */
export const OPEN_SESSION_ATTRIBUTE = "data-armada-open-session";

/** The id of the Session whose page is drawn, or `undefined` where none is. */
export function openSessionIn(doc: Document): string | undefined {
  const id = doc.querySelector(`[${OPEN_SESSION_ATTRIBUTE}]`)?.getAttribute(OPEN_SESSION_ATTRIBUTE);
  return id === null || id === undefined || id === "" ? undefined : id;
}
