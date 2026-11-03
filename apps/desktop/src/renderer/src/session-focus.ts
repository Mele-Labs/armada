// A Job opened with nothing in it is asked to put the caret in its message box when its page shows. The ask is held for a moment, because the page
// that shows it may draw twice while the Cockpit takes the stage over.

const HELD_MS = 3000;
let asked:
  | {
      id: string;
      at: number;
    }
  | undefined;

const heard = new Set<(id: string) => void>();

/** Hears each ask, for a message box already on screen, which a mount-time read would not reach. */
export function onComposerFocusAsked(listener: (id: string) => void): () => void {
  heard.add(listener);
  return () => void heard.delete(listener);
}

export function askComposerFocus(id: string): void {
  asked = {
    id,
    at: Date.now(),
  };
  heard.forEach((one) => one(id));
}

export function composerFocusAsked(id: string): boolean {
  return asked !== undefined && asked.id === id && Date.now() - asked.at < HELD_MS;
}
