// The one rule every single-key shortcut shares: it is suppressed while a text input holds focus.
//
// **`docs/contracts/design-system.md`, "Keyboard and command palette", is the map.** Nothing here
// decides a binding. `holdsText` is the whole of the suppression, and every read goes through it:
// typing into a filter box must not fire the keys the map binds.

/**
 * Whether a text input holds focus, and so whether every single-key shortcut is
 * suppressed.
 *
 * `contentEditable` is in here with the three element types because a rich
 * field is a text input to the person typing in it, whatever the DOM calls it.
 * A `select` counts too: its own type-ahead is how a person picks an option by
 * name, and a key that acted instead of jumping to the option would be the same
 * defect one control over.
 */
export function holdsText(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  // A listener on the window sees a field inside a shadow root as its host, as the file editor's is; the focused one is under it.
  const inside = target.shadowRoot?.activeElement;
  if (inside !== null && inside !== undefined) return holdsText(inside);
  if (target.isContentEditable) return true;
  const tag = target.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT";
}
