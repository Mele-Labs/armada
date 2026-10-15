// What the recipient of a shared sketch opens, drawn on the mock's page so a walk shows it. Mock-only: the
// real Bridge hands the bytes to a save dialog and keeps nothing on screen. **And the mock's clipboard**: a walk
// that pressed Copy against the machine's would put its words there, so it is replaced with one that shows them.

const VIEWER = "sketch-share-view";

type Fill = (body: HTMLElement) => void;

/** One panel at a time, over the right of the window, with the one control a person needs: Close. */
function show(label: string, fill: Fill): void {
  document.getElementById(VIEWER)?.remove();
  const panel = document.createElement("section");
  panel.id = VIEWER;
  panel.setAttribute("role", "region");
  panel.setAttribute("aria-label", label);
  const style = panel.style;
  style.position = "fixed";
  // Over the left of the window: the walk's own card sits at the right and must stay pressable.
  style.insetBlock = "var(--space-6)";
  style.insetInlineStart = "var(--space-6)";
  style.inlineSize = "min(58vw, 960px)";
  style.display = "flex";
  style.flexDirection = "column";
  style.gap = "var(--space-2)";
  style.padding = "var(--space-3)";
  style.border = "var(--border-width) solid var(--border-strong)";
  style.borderRadius = "var(--radius-lg)";
  style.background = "var(--bg-raised)";
  style.color = "var(--fg-default)";
  style.fontFamily = "var(--font-sans)";
  style.zIndex = "calc(var(--z-palette) - 1)";

  const bar = document.createElement("div");
  bar.style.display = "flex";
  bar.style.justifyContent = "space-between";
  bar.style.alignItems = "center";
  const heading = document.createElement("span");
  heading.textContent = label;
  const close = document.createElement("button");
  close.type = "button";
  close.textContent = "Close what was saved";
  close.addEventListener("click", () => panel.remove());
  bar.append(heading, close);
  panel.append(bar);
  fill(panel);
  document.body.append(panel);
}

const decode = (bytes: Uint8Array) => new TextDecoder().decode(bytes);

/** A file as it opens for the person it is sent to. */
export function showSaved(file: { kind: "html" | "png" | "svg"; name: string; bytes: Uint8Array }): void {
  show(`Opened ${file.name}`, (panel) => {
    if (file.kind === "html") {
      const page = document.createElement("iframe");
      page.title = file.name;
      page.srcdoc = decode(file.bytes);
      page.style.flex = "1";
      page.style.border = "0";
      page.style.borderRadius = "var(--radius-md)";
      panel.append(page);
      return;
    }
    const picture = document.createElement("img");
    picture.alt = file.name;
    picture.src = URL.createObjectURL(new Blob([file.bytes as BlobPart], { type: file.kind === "png" ? "image/png" : "image/svg+xml" }));
    picture.style.maxInlineSize = "100%";
    picture.style.objectFit = "contain";
    panel.append(picture);
  });
}

/** What reached the clipboard, as the page that pastes it would see it. */
function showCopied(text: string): void {
  show("On the clipboard", (panel) => {
    const words = document.createElement("pre");
    words.textContent = text;
    words.style.margin = "0";
    words.style.overflow = "auto";
    words.style.fontFamily = "var(--font-mono)";
    words.style.fontSize = "var(--text-sm)";
    panel.append(words);
  });
}

/** Replaces the page's clipboard with one that writes nowhere but the panel. */
export function useWalkClipboard(): void {
  Object.defineProperty(navigator, "clipboard", {
    configurable: true,
    value: { writeText: async (text: string) => showCopied(text) },
  });
}
