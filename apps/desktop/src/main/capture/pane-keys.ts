// The keys of Bridge's own window that a walk's page or bar hears first, and how they are handed back. A walk drawn as a tab is a pair of views
// over Bridge's window, and a key goes to the view that has the keyboard: Bridge's renderer hears nothing while they do, so jumping to another Job,
// ⌘K and Esc would stop working. Pure, so it is read without a window.

/** A key press as Electron reads it in a view's `before-input-event`. */
export type PaneInput = {
  type: string;
  key: string;
  code: string;
  meta: boolean;
  alt: boolean;
  control: boolean;
  shift: boolean;
};

/** What a page needs for itself under ⌘: editing its text. Its field would otherwise lose copy, paste, cut, select all and undo. */
const EDITING = new Set([
  "KeyC",
  "KeyV",
  "KeyX",
  "KeyA",
  "KeyZ",
]);

/** What the system owns under ⌘, and the app's menu: quitting, closing, minimising, hiding and settings. They are not handed on, so the menu still gets them. */
const SYSTEM = new Set([
  "KeyQ",
  "KeyW",
  "KeyM",
  "KeyH",
  "Comma",
  "Backquote",
  "Tab",
  "Space",
]);

/**
 * Whether a press is Bridge's own window's: Esc alone, or any press with ⌘ held that is neither editing the page's text nor the system's. **⌘↵, ⌘R and the
 * capture binding are the walk's own and are read before this**, so they never get here. A press with ⌃ and no ⌘ is a text field's on macOS.
 */
export function isMainKey(input: PaneInput): boolean {
  if (input.type !== "keyDown") return false;
  if (input.key === "Escape") return !input.meta && !input.alt && !input.control && !input.shift;
  if (!input.meta || input.control) return false;
  return !EDITING.has(input.code) && !SYSTEM.has(input.code);
}

const NAMED: Record<string, string> = {
  ArrowUp: "Up",
  ArrowDown: "Down",
  ArrowLeft: "Left",
  ArrowRight: "Right",
  " ": "Space",
  Escape: "Escape",
  Enter: "Enter",
  Backspace: "Backspace",
  Delete: "Delete",
  Tab: "Tab",
  Home: "Home",
  End: "End",
  PageUp: "PageUp",
  PageDown: "PageDown",
};

/** The `keyCode` Electron's `sendInputEvent` reads for a press: its letter or digit by its place on the keyboard, which ⌥ and ⇧ rewrite in `key`. */
export function keyCodeOf(input: PaneInput): string {
  const letter = /^Key([A-Z])$/.exec(input.code)?.[1];
  if (letter !== undefined) return letter;
  const digit = /^Digit([0-9])$/.exec(input.code)?.[1];
  if (digit !== undefined) return digit;
  return NAMED[input.key] ?? input.key;
}

/** The modifiers a press holds, as `sendInputEvent` names them. */
export function modifiersOf(input: PaneInput): ("meta" | "alt" | "shift" | "control")[] {
  return [
    ...(input.meta
      ? ([
          "meta",
        ] as const)
      : []),
    ...(input.alt
      ? ([
          "alt",
        ] as const)
      : []),
    ...(input.shift
      ? ([
          "shift",
        ] as const)
      : []),
    ...(input.control
      ? ([
          "control",
        ] as const)
      : []),
  ];
}

/** Hands a press to Bridge's own window: down and up, as the keyboard made them. `false` where the press is not Bridge's. */
export function forwardTo(
  contents: {
    isDestroyed: () => boolean;
    sendInputEvent: (event: {
      type: "keyDown" | "keyUp";
      keyCode: string;
      modifiers: ("meta" | "alt" | "shift" | "control")[];
    }) => void;
  } | null,
  input: PaneInput,
): boolean {
  if (contents === null || contents.isDestroyed() || !isMainKey(input)) return false;
  const event = {
    keyCode: keyCodeOf(input),
    modifiers: modifiersOf(input),
  };
  contents.sendInputEvent({
    type: "keyDown",
    ...event,
  });
  contents.sendInputEvent({
    type: "keyUp",
    ...event,
  });
  return true;
}
