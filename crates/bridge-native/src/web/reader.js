// What the reader asks of a page it shows: where its headings are, what he selected, and a scroll to a block or to words
// he marked. Evaluated in the finder's content world, which shares the page's DOM and nothing else, so the page's scripts see
// no global of this. It reads text and moves one thing, the page's scroll. A block is named by its place among the page's
// block elements, `b0`, `b1`, in document order.
(function () {
  if (typeof globalThis.ArmadaReader === "object") return;
  const BLOCKS = "h1,h2,h3,h4,h5,h6,p,li,td,th,pre,blockquote,figcaption,armada-sketch";
  const flat = (said) => String(said == null ? "" : said).replace(/\s+/g, " ").trim();
  const blocks = () => Array.from(document.querySelectorAll(BLOCKS)).filter((el) => !el.closest("script,style,template"));
  const named = (el) => {
    const at = el ? blocks().indexOf(el) : -1;
    return at < 0 ? "" : "b" + at;
  };
  const reader = {
    // The headings in the order the page holds them, each with the block it is.
    words() {
      const heads = [];
      blocks().forEach((el, at) => {
        if (/^H[1-6]$/.test(el.tagName)) heads.push({ level: Number(el.tagName[1]), text: flat(el.textContent), block: "b" + at });
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
      return JSON.stringify({ quote, block: named(el) });
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
  };
  globalThis.ArmadaReader = reader;
})();
