// Text that is cut with an ellipsis says what it is on hover. One listener for the whole window: the pointer
// enters a cell, and if that cell or an ancestor is drawing an ellipsis over words that do not fit, the full words
// are set as its `title`, and taken off again when the pointer leaves or the words fit. Text that fits has no
// title, so nothing says what is already on screen. A caller that set a `title` of its own keeps it.

const OURS = "data-clip-title";



let canvas: CanvasRenderingContext2D | null = null;

function widthOf(words: string, style: CSSStyleDeclaration): number {
  canvas ??= document.createElement("canvas").getContext("2d");
  if (canvas === null) return 0;
  canvas.font = `${style.fontWeight} ${style.fontSize} ${style.fontFamily}`;
  return canvas.measureText(words).width;
}

/** The words an element is cutting, or undefined where it is not cutting any. */
function cutWords(el: Element): string | undefined {
  const style = getComputedStyle(el);
  if (el instanceof HTMLSelectElement) {
    const words = el.selectedOptions[0]?.text.trim() ?? "";
    const room = el.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight);
    return words !== "" && widthOf(words, style) > room ? words : undefined;
  }
  const oneLine = style.textOverflow === "ellipsis";
  const lines = style.getPropertyValue("-webkit-line-clamp");
  if (!oneLine && (lines === "" || lines === "none")) return undefined;
  const clipped = oneLine ? el.scrollWidth > el.clientWidth + 1 : el.scrollHeight > el.clientHeight + 1;
  if (!clipped) return undefined;
  const words = (el instanceof HTMLElement ? el.innerText : (el.textContent ?? "")).replace(/\s+/g, " ").trim();
  return words === "" ? undefined : words;
}

/** Starts the listener. Returns the way to stop it. */
export function watchClippedText(): () => void {
  const over = (event: Event): void => {
    let el = event.target instanceof Element ? event.target : null;
    for (let depth = 0; el !== null && el !== document.body && depth < 6; depth += 1, el = el.parentElement) {
      if (el.hasAttribute("title") && !el.hasAttribute(OURS)) return;
      const words = cutWords(el);
      if (words !== undefined) {
        el.setAttribute("title", words);
        el.setAttribute(OURS, "");
        return;
      }
      if (el.hasAttribute(OURS)) {
        el.removeAttribute("title");
        el.removeAttribute(OURS);
      }
    }
  };
  const off = (event: Event): void => {
    const el = event.target;
    if (el instanceof Element && el.hasAttribute(OURS)) {
      el.removeAttribute("title");
      el.removeAttribute(OURS);
    }
  };
  document.addEventListener("pointerover", over, true);
  document.addEventListener("pointerout", off, true);
  document.addEventListener("focusin", over, true);
  return () => {
    document.removeEventListener("pointerover", over, true);
    document.removeEventListener("pointerout", off, true);
    document.removeEventListener("focusin", over, true);
  };
}
