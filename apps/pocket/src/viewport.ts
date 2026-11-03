// iOS leaves the layout as it was when the keyboard opens: only the visual viewport shrinks, and it may pan. The app follows
// the visual viewport, so a screen's bottom bar and a Job's message box sit above the keyboard.

/** A visual viewport this much shorter than the screen has a keyboard over it, and not a browser's toolbar. */
const KEYBOARD_AT_LEAST = 120;

type Visible = Pick<VisualViewport, "height" | "offsetTop" | "scale">;
type Screen = Visible & Pick<VisualViewport, "width">;

const zoomed = (view: Pick<Visible, "scale">): boolean => Math.abs(view.scale - 1) > 0.01;

/** The visible screen, written where `pocket.css` and `styles.css` read it. A pinch zoom leaves the app as it was. */
export function fit(view: Visible, fullHeight: number, root: HTMLElement): void {
  if (zoomed(view)) return;
  root.style.setProperty("--pk-visible-height", `${view.height}px`);
  root.style.setProperty("--pk-visible-top", `${view.offsetTop}px`);
  root.toggleAttribute("data-keyboard", fullHeight - view.height >= KEYBOARD_AT_LEAST);
}

/**
 * Reads each change of the visual viewport. The screen's height is the tallest it has been at this width, since iOS may shorten
 * `innerHeight` with the keyboard. A pan is undone here, before it paints: the app would follow it a frame late, and flicker.
 */
export function followScreen(
  view: Screen,
  root: HTMLElement,
  unpan: () => void,
): () => void {
  let width = view.width;
  let full = view.height;
  return () => {
    if (view.width !== width) {
      width = view.width;
      full = view.height;
    }
    full = Math.max(full, view.height);
    if (view.offsetTop > 0 && !zoomed(view)) unpan();
    fit(view, full, root);
  };
}

export function fitToVisibleScreen(): void {
  const view = window.visualViewport;
  if (view === null) return;
  const follow = followScreen(view, document.documentElement, () => window.scrollTo(0, 0));
  view.addEventListener("resize", follow);
  view.addEventListener("scroll", follow);
  follow();
}
