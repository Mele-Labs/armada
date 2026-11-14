// What Bridge asks of a page to find the place a walk step aims at. Evaluated in a content world of
// its own (`WKContentWorld`), which shares the page's DOM and nothing else: the page's scripts see no
// global and no listener of this. It reads the element a target names and its box, and moves one thing,
// the page's scroll, as a person's own scroll would. Never a node, an attribute or a style.
(function () {
  if (typeof globalThis.ArmadaFinder === "object") return;
  const IMPLICIT = {
    a: (el) => (el.hasAttribute("href") ? "link" : null),
    area: (el) => (el.hasAttribute("href") ? "link" : null),
    article: () => "article",
    aside: () => "complementary",
    button: () => "button",
    dialog: () => "dialog",
    footer: () => "contentinfo",
    form: () => "form",
    h1: () => "heading",
    h2: () => "heading",
    h3: () => "heading",
    h4: () => "heading",
    h5: () => "heading",
    h6: () => "heading",
    header: () => "banner",
    hr: () => "separator",
    img: (el) => (el.getAttribute("alt") === "" ? "presentation" : "img"),
    input: (el) => {
      const type = (el.getAttribute("type") || "text").toLowerCase();
      if (type === "checkbox") return "checkbox";
      if (type === "radio") return "radio";
      if (type === "range") return "slider";
      if (type === "number") return "spinbutton";
      if (type === "search") return "searchbox";
      if (type === "hidden") return null;
      if (["button", "submit", "reset", "image"].includes(type)) return "button";
      return "textbox";
    },
    li: () => "listitem",
    main: () => "main",
    nav: () => "navigation",
    ol: () => "list",
    option: () => "option",
    progress: () => "progressbar",
    section: (el) => (el.hasAttribute("aria-label") || el.hasAttribute("aria-labelledby") ? "region" : null),
    select: (el) => (el.multiple || el.size > 1 ? "listbox" : "combobox"),
    summary: () => "button",
    table: () => "table",
    td: () => "cell",
    textarea: () => "textbox",
    th: () => "columnheader",
    tr: () => "row",
    ul: () => "list",
  };
  const FROM_CONTENT = new Set(["button", "link", "tab", "heading", "menuitem", "option", "cell", "columnheader", "row", "listitem", "checkbox", "radio", "switch", "treeitem", "article", "dialog", "status", "alert"]);

  const flat = (said) => String(said == null ? "" : said).replace(/\s+/g, " ").trim().toLowerCase();
  const roleOf = (el) => {
    const given = el.getAttribute("role");
    if (given) return given.trim().split(/\s+/)[0].toLowerCase();
    const make = IMPLICIT[el.tagName.toLowerCase()];
    return make ? make(el) : null;
  };
  const textOf = (el) => (el.innerText != null ? el.innerText : el.textContent) || "";
  const nameOf = (el, role) => {
    const by = el.getAttribute("aria-labelledby");
    if (by) {
      const said = by.split(/\s+/).map((id) => document.getElementById(id)).filter(Boolean).map(textOf).join(" ");
      if (flat(said)) return flat(said);
    }
    const label = el.getAttribute("aria-label");
    if (flat(label)) return flat(label);
    if (el.labels && el.labels.length) return flat(Array.from(el.labels).map(textOf).join(" "));
    if (el.tagName === "IMG") return flat(el.getAttribute("alt"));
    if (el.tagName === "INPUT") {
      const type = (el.getAttribute("type") || "text").toLowerCase();
      if (["button", "submit", "reset"].includes(type) && flat(el.value)) return flat(el.value);
      if (flat(el.getAttribute("placeholder"))) return flat(el.getAttribute("placeholder"));
    }
    if (FROM_CONTENT.has(role) && flat(textOf(el))) return flat(textOf(el));
    return flat(el.getAttribute("title"));
  };
  const shows = (el) => {
    const box = el.getBoundingClientRect();
    if (box.width <= 0 || box.height <= 0) return false;
    const style = getComputedStyle(el);
    return style.visibility !== "hidden" && style.display !== "none" && Number(style.opacity) !== 0;
  };
  const matches = (el, target) => {
    if (target.role === "text") {
      const own = Array.from(el.childNodes).filter((n) => n.nodeType === 3).map((n) => n.textContent).join(" ");
      return flat(own).includes(flat(target.name));
    }
    const role = roleOf(el);
    if (role !== flat(target.role)) return false;
    if (target.name == null) return true;
    const name = nameOf(el, role);
    return target.exact ? name === flat(target.name) : name.includes(flat(target.name));
  };
  const find = (target, root) => {
    let scope = root || document;
    if (target.within) {
      const outer = find(target.within, root);
      if (!outer) return null;
      scope = outer;
    }
    const all = Array.from(scope.querySelectorAll("*")).filter((el) => matches(el, target));
    return all.find(shows) || null;
  };
  const boxOf = (el) => {
    const box = el.getBoundingClientRect();
    const radius = parseFloat(getComputedStyle(el).borderTopLeftRadius) || 0;
    return { x: Math.round(box.x), y: Math.round(box.y), width: Math.round(box.width), height: Math.round(box.height), radius: Math.round(radius) };
  };
  let held = null;
  const finder = {
    // The target's element is scrolled to the middle of the viewport and held; its box answers, or null where nothing shows it yet.
    look(target) {
      held = find(target);
      if (!held) return JSON.stringify(null);
      held.scrollIntoView({ block: "center", inline: "nearest" });
      return JSON.stringify(boxOf(held));
    },
    // A click on the held element, which the page's listeners hear as they hear a person's.
    press() {
      if (!held || !held.isConnected) return JSON.stringify(false);
      held.click();
      return JSON.stringify(true);
    },
  };
  globalThis.ArmadaFinder = { finder };
})();
