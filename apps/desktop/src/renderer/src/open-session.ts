// A way for the annotation layer, which has a root of its own beside the app's, to ask the app to
// open a Session. A window event, because the layer must not import the app.

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
