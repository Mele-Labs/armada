// A way for the annotation layer to put a sentence in the app's toast, as `open-session.ts` does
// for opening a Session: a window event, because the layer must not import the app.

import { useEffect } from "react";

const TELL = "armada:tell";

export function askToTell(sentence: string): void {
  window.dispatchEvent(new CustomEvent<string>(TELL, { detail: sentence }));
}

export function useTellAsked(tell: (sentence: string) => void): void {
  useEffect(() => {
    const heard = (event: Event): void => tell((event as CustomEvent<string>).detail);
    window.addEventListener(TELL, heard);
    return () => window.removeEventListener(TELL, heard);
  }, [tell]);
}
