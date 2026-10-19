// A Job opened with nothing in it is asked to put the caret in its message box when its page shows. The ask is held for a moment, because the page
// that shows it may draw twice while the Cockpit takes the stage over.

const HELD_MS = 3000;
let asked: { id: string; at: number } | undefined;

export function askComposerFocus(id: string): void {
  asked = { id, at: Date.now() };
}

export function composerFocusAsked(id: string): boolean {
  return asked !== undefined && asked.id === id && Date.now() - asked.at < HELD_MS;
}
