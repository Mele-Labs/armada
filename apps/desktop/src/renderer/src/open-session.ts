// A way for the annotation layer, which has a root of its own beside the app's, to ask the app to
// open a Session. A window event, because the layer must not import the app.

import { useEffect } from "react";

import { seedSessionDraft } from "@armada/components";

const OPEN_SESSION = "armada:open-session";

export function askToOpenSession(id: string): void {
  window.dispatchEvent(new CustomEvent<string>(OPEN_SESSION, { detail: id }));
}

/**
 * Open a Session, with a first message already in its message box and not sent. **Start Session on
 * a retro item**: Fleet started the Session and drafted the message, and the person reads it, edits
 * it and sends it. The draft is written before the page is asked to open, so the box finds it.
 */
export function openSessionWithDraft(id: string, draft?: string): void {
  if (draft !== undefined && draft !== "") seedSessionDraft(id, draft);
  askToOpenSession(id);
}

export function useOpenSessionAsked(open: (id: string) => void): void {
  useEffect(() => {
    const heard = (event: Event): void => open((event as CustomEvent<string>).detail);
    window.addEventListener(OPEN_SESSION, heard);
    return () => window.removeEventListener(OPEN_SESSION, heard);
  }, [open]);
}
