// What the reader asks of a page it shows: where its headings are, what he selected, a scroll to a block or to words he
// marked, and the margin's marks. Evaluated in the finder's content world, which shares the page's DOM and nothing else, so
// the page's scripts see no global of this. It reads text and moves the page's scroll. What it changes: a leading h1 that
// says the title the head already says, which it hides, and the margin's marks, each a span of its own: a passage a thread
// is on (`data-armada-mark`), the words a suggested edit strikes (`data-armada-struck`) and the words it adds
// (`data-armada-added`). Taking the marks out leaves the page's own text as it was. A block is named by its place among the
// page's block elements, `b0`, `b1`, in document order.
(function () {
  if (typeof globalThis.ArmadaReader === "object") return;
  const BLOCKS = "h1,h2,h3,h4,h5,h6,p,li,td,th,pre,blockquote,figcaption,armada-sketch";
  const flat = (said) => String(said == null ? "" : said).replace(/\s+/g, " ").trim();
  const blocks = () => Array.from(document.querySelectorAll(BLOCKS)).filter((el) => !el.closest("script,style,template"));
  const named = (el) => {
    const at = el ? blocks().indexOf(el) : -1;
    return at < 0 ? "" : "b" + at;
  };
  // The h2 a block stands under, where one is above it.
  const sectionOf = (el) => {
    const all = blocks();
    for (let at = el ? all.indexOf(el) : -1; at >= 0; at--) {
      if (all[at].tagName === "H2") return flat(all[at].textContent);
    }
    return "";
  };
  const MARKS = "[data-armada-mark],[data-armada-struck]";
  const unmark = () => {
    document.querySelectorAll("[data-armada-added]").forEach((el) => el.remove());
    document.querySelectorAll(MARKS).forEach((el) => {
      const parent = el.parentNode;
      while (el.firstChild) parent.insertBefore(el.firstChild, el);
      parent.removeChild(el);
      parent.normalize();
    });
  };
  // The words under `root` as `flat` reads them, and the text node and offset of each character.
  const textIn = (root) => {
    const skip = (node) => node.parentElement && node.parentElement.closest("script,style,template,[data-armada-added]");
    const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, { acceptNode: (node) => (skip(node) ? NodeFilter.FILTER_REJECT : NodeFilter.FILTER_ACCEPT) });
    let words = "";
    const at = [];
    let space = true;
    for (let node = walker.nextNode(); node; node = walker.nextNode()) {
      const text = node.nodeValue;
      for (let i = 0; i < text.length; i++) {
        const blank = /\s/.test(text[i]);
        if (blank && space) continue;
        space = blank;
        words += blank ? " " : text[i];
        at.push([node, i]);
      }
    }
    return { words, at };
  };
  // Where `quote` stands: in its block first, then anywhere in the page.
  const find = (quote, block) => {
    const want = flat(quote);
    if (!want) return null;
    const holder = /^b\d+$/.test(String(block || "")) ? blocks()[Number(String(block).slice(1))] : null;
    for (const root of [holder, document.body]) {
      if (!root) continue;
      const read = textIn(root);
      const start = read.words.indexOf(want);
      if (start >= 0) return { first: read.at[start], last: read.at[start + want.length - 1] };
    }
    return null;
  };
  // Each text node of what `find` found wrapped in a span of its own, so no element of the page is split.
  const wrap = (found, name, thread, style) => {
    const range = document.createRange();
    range.setStart(found.first[0], found.first[1]);
    range.setEnd(found.last[0], found.last[1] + 1);
    const common = range.commonAncestorContainer.nodeType === 1 ? range.commonAncestorContainer : range.commonAncestorContainer.parentNode;
    const nodes = [];
    const walker = document.createTreeWalker(common, NodeFilter.SHOW_TEXT);
    for (let node = walker.nextNode(); node; node = walker.nextNode()) if (range.intersectsNode(node)) nodes.push(node);
    const made = [];
    nodes.forEach((node) => {
      const from = node === range.startContainer ? range.startOffset : 0;
      const to = node === range.endContainer ? range.endOffset : node.nodeValue.length;
      if (to <= from) return;
      if (to < node.nodeValue.length) node.splitText(to);
      const piece = from > 0 ? node.splitText(from) : node;
      if (!/\S/.test(piece.nodeValue)) return;
      const span = document.createElement("span");
      span.setAttribute(name, thread);
      Object.entries(style).forEach(([key, value]) => span.style.setProperty(key, value));
      piece.parentNode.insertBefore(span, piece);
      span.appendChild(piece);
      made.push(span);
    });
    return made;
  };
  const reader = {
    // The headings in the order the page holds them, each with the block it is.
    words(title) {
      const heads = [];
      const all = blocks();
      const want = flat(title).toLowerCase();
      // The page's first block is its own title where an h1 says what the head says: drawn once, in the head.
      const first = all.find((el) => flat(el.textContent) !== "");
      if (want && first && first.tagName === "H1" && flat(first.textContent).toLowerCase() === want) first.style.setProperty("display", "none");
      all.forEach((el, at) => {
        if (/^H[1-6]$/.test(el.tagName) && el.style.display !== "none") heads.push({ level: Number(el.tagName[1]), text: flat(el.textContent), block: "b" + at });
      });
      return JSON.stringify({ heads });
    },
    // The words he selected and the block they start in; with none selected, the block at the top of the view.
    selection() {
      const picked = getSelection();
      let quote = "";
      let el = null;
      if (picked && picked.rangeCount > 0 && !picked.isCollapsed) {
        quote = flat(picked.toString());
        const node = picked.getRangeAt(0).startContainer;
        const holder = node.nodeType === 1 ? node : node.parentElement;
        el = holder ? holder.closest(BLOCKS) : null;
      }
      if (!el) el = blocks().find((one) => one.getBoundingClientRect().bottom > 0) || null;
      return JSON.stringify({ quote, block: named(el), section: sectionOf(el) });
    },
    // The block scrolled to the top of the view.
    go(block) {
      const el = blocks()[Number(String(block).slice(1))];
      if (!el) return JSON.stringify(false);
      el.scrollIntoView({ block: "start", inline: "nearest" });
      return JSON.stringify(true);
    },
    // The words he marked, selected and scrolled to the middle; the block they were on where they are not found.
    show(quote, block) {
      const picked = getSelection();
      if (picked) picked.removeAllRanges();
      if (quote && typeof window.find === "function" && window.find(quote, true, false, true)) {
        const node = getSelection().getRangeAt(0).startContainer;
        const el = node.nodeType === 1 ? node : node.parentElement;
        if (el) el.scrollIntoView({ block: "center", inline: "nearest" });
        return JSON.stringify(true);
      }
      return reader.go(block);
    },
    // The margin's marks, drawn again from `list`: each thread's passage in his colour, the one he is on stronger, and a
    // suggested edit as its struck words with the added words after them. A passage the page no longer holds draws nothing.
    mark(list) {
      unmark();
      const tint = (share) => `color-mix(in srgb, var(--you) ${share}%, transparent)`;
      (Array.isArray(list) ? list : []).forEach((one) => {
        const thread = String(one.thread || "");
        if (one.struck) {
          const found = find(one.struck, one.block);
          if (!found) return;
          const struck = wrap(found, "data-armada-struck", thread, { "text-decoration": "line-through", color: "var(--diff-del-fg)", background: "var(--diff-del-bg)", "border-radius": "var(--radius-sm)" });
          const last = struck[struck.length - 1];
          if (!last) return;
          const added = document.createElement("span");
          added.setAttribute("data-armada-added", thread);
          added.textContent = String(one.added || "");
          [["color", "var(--diff-add-fg)"], ["background", "var(--diff-add-bg)"], ["border-radius", "var(--radius-sm)"], ["margin-inline-start", "var(--space-1)"]].forEach(([key, value]) => added.style.setProperty(key, value));
          last.after(added);
          return;
        }
        const found = find(one.quote, one.block);
        if (!found) return;
        wrap(found, "data-armada-mark", thread, { background: tint(one.on ? 34 : 16), "border-block-end": "var(--border-width) solid var(--you-edge)", "border-radius": "var(--radius-sm)" });
      });
      return JSON.stringify(true);
    },
    // A thread's mark scrolled to the middle of the view; its words found where it draws no mark.
    reach(thread, quote, block) {
      const el = Array.from(document.querySelectorAll("[data-armada-mark],[data-armada-struck]")).find((one) => one.getAttribute("data-armada-mark") === thread || one.getAttribute("data-armada-struck") === thread);
      if (el) {
        el.scrollIntoView({ block: "center", inline: "nearest" });
        return JSON.stringify(true);
      }
      return reader.show(quote, block);
    },
    // Where he is in the page: the first block in view, by its words and its place, and how far it stands from the top.
    place() {
      const all = blocks();
      const el = all.find((one) => one.getBoundingClientRect().bottom > 0);
      if (!el) return JSON.stringify({ text: "", at: -1, top: 0, y: window.scrollY });
      return JSON.stringify({ text: flat(el.textContent).slice(0, 120), at: all.indexOf(el), top: el.getBoundingClientRect().top, y: window.scrollY });
    },
    // The place `place` read, in this page: the block with the same words, else the one at the same place, back where it was.
    restore(place) {
      if (!place) return JSON.stringify(false);
      const all = blocks();
      const el = all.find((one) => place.text && flat(one.textContent).slice(0, 120) === place.text) || (place.at >= 0 ? all[place.at] : null);
      if (el) {
        el.scrollIntoView({ block: "start", inline: "nearest" });
        window.scrollBy(0, -Number(place.top || 0));
      } else {
        window.scrollTo(0, Number(place.y || 0));
      }
      return JSON.stringify(true);
    },
  };
  globalThis.ArmadaReader = reader;
})();
