// ⌥⌘A in Bridge's document starts Capture on the Job browser on screen, ahead of the app annotation layer's own ⌥⌘A. The listener is
// added when this module loads, before any component mounts, so it runs first on `window`; where the layer is already on, or no Job
// browser is shown, it does nothing and the layer keeps the chord (it must still be able to close itself).

const toggles = new Set<() => void>();

/** A Job browser pane is on screen and answers the chord by `toggle`. */
export function takesAnnotateChord(toggle: () => void): () => void {
  toggles.add(toggle);
  return () => void toggles.delete(toggle);
}

if (typeof window !== "undefined") {
  window.addEventListener(
    "keydown",
    (event) => {
      const chord = event.code === "KeyA" && event.metaKey && event.altKey && !event.ctrlKey && !event.shiftKey;
      if (!chord || toggles.size === 0 || document.querySelector("[data-armada-annotate]") !== null) return;
      event.preventDefault();
      event.stopImmediatePropagation();
      [...toggles].at(-1)?.();
    },
    true,
  );
}
