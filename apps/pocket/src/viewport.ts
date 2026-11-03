// iOS leaves the layout as it was when the keyboard opens: only the visual viewport shrinks, and it may pan. The app follows
// the visual viewport, so a screen's bottom bar and a Job's message box sit above the keyboard.

/** A visual viewport this much shorter than the layout has a keyboard over it, and not a browser's toolbar. */
const KEYBOARD_AT_LEAST = 120;

type Visible = Pick<VisualViewport, "height" | "offsetTop" | "scale">;

/** The visible screen, written where `pocket.css` and `styles.css` read it. A pinch zoom leaves the app as it was. */
export function fit(view: Visible, layoutHeight: number, root: HTMLElement): void {
  if (Math.abs(view.scale - 1) > 0.01) return;
  root.style.setProperty("--pk-visible-height", `${view.height}px`);
  root.style.setProperty("--pk-visible-top", `${view.offsetTop}px`);
  root.toggleAttribute("data-keyboard", layoutHeight - view.height >= KEYBOARD_AT_LEAST);
}

export function fitToVisibleScreen(): void {
  const view = window.visualViewport;
  if (view === null) return;
  const follow = () => fit(view, window.innerHeight, document.documentElement);
  view.addEventListener("resize", follow);
  view.addEventListener("scroll", follow);
  follow();
}
