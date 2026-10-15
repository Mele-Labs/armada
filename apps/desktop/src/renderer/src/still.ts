// A window nobody is looking at stops drawing motion. The map's loops (rotors, pings, glows, bobbing) hold
// the compositor at the display's rate for as long as one runs, and the page does nothing the owner can see
// while he is in another app. `styles/index.css` pauses every animation under `data-still`.

/** Marks the root `data-still` while the window is hidden or does not have focus. Returns the way to stop. */
export function watchStill(): () => void {
  const root = document.documentElement;
  const set = (): void => {
    root.toggleAttribute("data-still", document.visibilityState === "hidden" || !document.hasFocus());
  };
  set();
  window.addEventListener("focus", set);
  window.addEventListener("blur", set);
  document.addEventListener("visibilitychange", set);
  return () => {
    window.removeEventListener("focus", set);
    window.removeEventListener("blur", set);
    document.removeEventListener("visibilitychange", set);
    root.removeAttribute("data-still");
  };
}
