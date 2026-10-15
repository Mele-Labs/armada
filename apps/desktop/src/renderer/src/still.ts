// A window nobody is looking at stops drawing motion. The map's loops (pings, glows, bobbing) hold
// the compositor at the display's rate for as long as one runs, and the page does nothing the owner can see
// while he is in another app. `styles/index.css` pauses every animation under `data-still`, and the map's
// under `data-idle`, which a minute with no input sets (the owner, 15 Oct 2026: the machine over the motion).

/** How long with no pointer, key or wheel before the map stands still. */
export const IDLE_MS = 60_000;
/** How often the idle mark is looked at: an event only stamps the time, so a moving mouse costs nothing. */
const LOOK_MS = 5_000;

/** Marks the root `data-still` while the window is hidden or does not have focus, and `data-idle` after `IDLE_MS` with no input. Returns the way to stop. */
export function watchStill(): () => void {
  const root = document.documentElement;
  let last = Date.now();
  const set = (): void => {
    root.toggleAttribute("data-still", document.visibilityState === "hidden" || !document.hasFocus());
  };
  const look = (): void => {
    root.toggleAttribute("data-idle", Date.now() - last >= IDLE_MS);
  };
  const input = (): void => {
    last = Date.now();
    if (root.hasAttribute("data-idle")) root.removeAttribute("data-idle");
  };
  set();
  window.addEventListener("focus", set);
  window.addEventListener("blur", set);
  document.addEventListener("visibilitychange", set);
  const inputs = ["pointermove", "pointerdown", "keydown", "wheel"] as const;
  for (const one of inputs) window.addEventListener(one, input, { passive: true, capture: true });
  const timer = window.setInterval(look, LOOK_MS);
  return () => {
    window.removeEventListener("focus", set);
    window.removeEventListener("blur", set);
    document.removeEventListener("visibilitychange", set);
    for (const one of inputs) window.removeEventListener(one, input, { capture: true });
    window.clearInterval(timer);
    root.removeAttribute("data-still");
    root.removeAttribute("data-idle");
  };
}
