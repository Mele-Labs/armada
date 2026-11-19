// Edit on a page: the words of its text, headings, lists and table cells typed in place. Evaluated in the finder's content
// world, as reader.js is, so the page's scripts see none of it. Each block is an editing host of its own, so a selection
// never runs from one block into the next, and nothing but words is typed: Return, pasted markup, a drop and formatting
// are refused. Every text node is noted as it stood, so the host reads back what each one says now and makes the page
// from the revision's own markup (editing.rs). What the kit draws is never editable.
(function () {
  if (typeof globalThis.ArmadaEdit === "object") return;
  const WORDS = "p,h1,h2,h3,h4,h5,h6,li,td,th,dt,dd,blockquote,figcaption,caption";
  const SKIP = "script,style,textarea,title,noscript,template,svg,armada-sketch,armada-action,armada-vote,.armada-sent,.armada-words,.armada-words-row";
  const REFUSED = ["insertParagraph", "insertLineBreak", "insertFromDrop", "insertFromPasteAsQuotation", "insertOrderedList", "insertUnorderedList", "insertHorizontalRule", "insertLink"];
  const flat = (said) => String(said == null ? "" : said).replace(/\s+/g, " ").trim();
  let held = null;

  // The text nodes of a block that are words, in document order.
  const textsOf = (block) => {
    const found = [];
    const walk = document.createTreeWalker(block, NodeFilter.SHOW_TEXT, {
      acceptNode: (node) => (node.parentElement && node.parentElement.closest(SKIP) ? NodeFilter.FILTER_REJECT : NodeFilter.FILTER_ACCEPT),
    });
    for (let node = walk.nextNode(); node; node = walk.nextNode()) found.push(node);
    return found;
  };
  const wordsOf = (block) => flat(textsOf(block.el).map((node) => node.data).join(""));

  const mark = (block) => {
    const changed = wordsOf(block) !== block.was;
    block.el.style.setProperty("box-shadow", changed ? "inset 3px 0 0 " + held.look.edge : "inset 0 -1px 0 " + held.look.line);
    block.el.style.setProperty("background-color", changed ? held.look.fill : block.background);
  };

  const edit = {
    // Every block with words becomes editable, with what he kept typed back in where it fits the page. Answers how many
    // text nodes are words.
    start(look, kept) {
      if (held) return JSON.stringify({ texts: held.nodes.length });
      held = { look, blocks: [], nodes: [], index: new Map() };
      const all = Array.from(document.querySelectorAll(WORDS)).filter((el) => !el.closest(SKIP));
      const outer = all.filter((el) => !(el.parentElement && el.parentElement.closest(WORDS)));
      for (const el of outer) {
        const nodes = textsOf(el);
        const block = { el, first: held.nodes.length, count: nodes.length, was: "", background: el.style.getPropertyValue("background-color") };
        for (const node of nodes) {
          held.index.set(node, held.nodes.length);
          held.nodes.push({ node, was: node.data });
        }
        block.was = flat(nodes.map((node) => node.data).join(""));
        if (block.was === "") continue;
        held.blocks.push(block);
      }
      if (Array.isArray(kept) && kept.length === held.nodes.length) kept.forEach((text, at) => (held.nodes[at].node.data = String(text)));
      for (const block of held.blocks) {
        const el = block.el;
        el.setAttribute("contenteditable", "true");
        for (const kit of el.querySelectorAll(SKIP)) kit.setAttribute("contenteditable", "false");
        el.addEventListener("beforeinput", (event) => {
          if (REFUSED.includes(event.inputType) || event.inputType.startsWith("format")) event.preventDefault();
        });
        el.addEventListener("keydown", (event) => {
          if (event.key === "Enter" && !event.metaKey) event.preventDefault();
        });
        el.addEventListener("paste", (event) => {
          event.preventDefault();
          const text = event.clipboardData ? event.clipboardData.getData("text/plain") : "";
          if (text) document.execCommand("insertText", false, text.replace(/\s*\n\s*/g, " "));
        });
        el.addEventListener("drop", (event) => event.preventDefault());
        el.addEventListener("input", () => mark(block));
        mark(block);
      }
      const first = held.blocks.find((block) => block.el.offsetParent !== null);
      if (first) {
        first.el.focus();
        const caret = getSelection();
        if (caret) caret.collapse(first.el, 0);
      }
      return JSON.stringify({ texts: held.nodes.length });
    },
    // What each noted text node said and says now. Text typed into a node the browser made is its block's: it joins the
    // noted node before it, or the block's first where none is before it.
    read() {
      if (!held) return JSON.stringify(null);
      const was = held.nodes.map((one) => one.was);
      const now = was.map(() => "");
      for (const block of held.blocks) {
        let last = -1;
        let lead = "";
        for (const node of textsOf(block.el)) {
          const at = held.index.get(node);
          if (at !== undefined && at >= block.first && at < block.first + block.count) {
            now[at] += node.data;
            last = at;
          } else if (last >= 0) now[last] += node.data;
          else lead += node.data;
        }
        if (lead && block.count > 0) now[block.first] = lead + now[block.first];
      }
      // A node in no block with words keeps what it said.
      held.nodes.forEach((one, at) => {
        if (!held.blocks.some((block) => at >= block.first && at < block.first + block.count)) now[at] = one.was;
      });
      return JSON.stringify({ was, now });
    },
  };
  globalThis.ArmadaEdit = edit;
})();
