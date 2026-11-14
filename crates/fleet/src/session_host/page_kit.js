// The kit's script on a sealed page, the one script the page's policy lets run. It draws each <armada-action> as a
// labelled button and each <armada-vote> as a vote, and hands his presses and votes to the host with
// window.parent.postMessage. It reaches nothing else: the host makes the Fleet call and posts the answer back.
// docs/practices/protocol/a-pages-sealed-serving.md holds the messages both ways.
(() => {
  "use strict";
  const record = JSON.parse(document.getElementById("armada-page-record").textContent);
  const { artifact, revision } = record;
  const post = (message) => window.parent.postMessage({ armada: 1, ...message, artifact, revision }, "*");
  const said = (element, name) => (element.getAttribute(name) || "").trim();
  const make = (name, attributes = {}, text) => {
    const element = document.createElement(name);
    for (const [key, value] of Object.entries(attributes)) element.setAttribute(key, value);
    if (text !== undefined) element.textContent = text;
    return element;
  };
  const sendKey = (event) => (event.metaKey || event.ctrlKey) && event.key === "Enter";

  // Actions.
  const actions = new Map();

  const lineOf = (action) => {
    if (action.line) return action.line;
    action.line = make("div", { class: "armada-sent", role: "status" });
    if (action.row?.tagName === "TR") action.row.cells[0].append(action.line);
    else if (action.row) action.row.querySelector(":scope > .armada-row-body").append(action.line);
    else action.element.after(action.line);
    return action.line;
  };

  const closeWords = (action) => {
    action.words?.remove();
    action.words = null;
  };

  const press = (action, words) => {
    action.button.disabled = true;
    post({ kind: "press", action: action.id, ...(words ? { words } : {}) });
  };

  const openWords = (action) => {
    if (action.words) {
      action.words.querySelector("textarea").focus();
      return;
    }
    const field = make("textarea", { "aria-label": `${action.label}: your words to the Job` });
    const send = make("button", { type: "button", class: "armada-send", title: "Send (⌘↵)" }, "Send");
    const box = make("div", { class: "armada-words" });
    box.append(field, send);
    const submit = () => press(action, field.value.trim());
    send.addEventListener("click", submit);
    field.addEventListener("keydown", (event) => {
      if (sendKey(event)) {
        event.preventDefault();
        submit();
      } else if (event.key === "Escape") {
        event.preventDefault();
        closeWords(action);
        action.button.focus();
      }
    });
    if (action.row?.tagName === "TR") {
      const row = make("tr", { class: "armada-words-row" });
      const cell = make("td", { colspan: String(action.row.cells.length) });
      cell.append(box);
      row.append(cell);
      action.row.after(row);
      action.words = row;
    } else {
      (action.row?.querySelector(":scope > .armada-row-body") ?? action.element.parentElement).append(box);
      action.words = box;
    }
    field.focus();
  };

  const did = (press) => {
    if (press.did.kind === "started") return `, started Job ${press.did.job.slice(0, 8)}`;
    if (press.did.kind === "answered") return ", answered";
    return "";
  };

  const pressed = (press) => {
    const action = actions.get(press.action);
    if (!action) return;
    action.button.disabled = false;
    action.row?.classList.add("armada-pressed");
    const line = lineOf(action);
    line.classList.remove("armada-refused");
    line.textContent = `Sent to the Job: ${action.label}${did(press)}`;
    closeWords(action);
  };

  for (const element of document.querySelectorAll("armada-action")) {
    const id = said(element, "id");
    const label = said(element, "label") || id;
    const act = said(element, "act");
    const button = make("button", { type: "button", class: "armada-act" }, label);
    element.replaceChildren(button);
    const row = element.closest("tr, li");
    if (row?.tagName === "LI") {
      if (!row.classList.contains("armada-row")) {
        const body = make("div", { class: "armada-row-body" });
        body.append(...row.childNodes);
        row.append(body, make("span", { class: "armada-acts" }));
        row.classList.add("armada-row");
      }
      row.querySelector(":scope > .armada-acts").append(element);
    } else if (row) {
      element.closest("td, th")?.classList.add("armada-acts");
      row.classList.add("armada-row");
    }
    const action = { id, label, act, element, row, button, line: null, words: null };
    // The first action with an id is the one Fleet reads.
    if (!actions.has(id)) actions.set(id, action);
    button.addEventListener("click", () => (act === "message" ? openWords(action) : press(action)));
  }

  // Votes.
  const votes = new Map();

  const pick = (vote, id) => {
    vote.picked = id;
    for (const take of vote.takes) {
      take.radio.checked = take.id === id;
      take.element.toggleAttribute("data-picked", take.id === id);
      take.note.hidden = take.id === id;
    }
  };

  const renumber = (vote) => {
    vote.takes.forEach((take, at) => {
      take.number.textContent = `${at + 1}.`;
    });
  };

  const placeInOrder = (vote, order) => {
    const byId = new Map(vote.takes.map((take) => [take.id, take]));
    vote.takes = order.map((id) => byId.get(id)).filter(Boolean);
    for (const take of vote.takes) vote.list.append(take.element);
    renumber(vote);
  };

  const move = (vote, take, by) => {
    const at = vote.takes.indexOf(take);
    const to = at + by;
    if (to < 0 || to >= vote.takes.length) return;
    const order = vote.takes.map((one) => one.id);
    order.splice(at, 1);
    order.splice(to, 0, take.id);
    placeInOrder(vote, order);
    take.element.focus();
  };

  const mark = (take, part, keep) => {
    if (take.marks.get(part) === keep) take.marks.delete(part);
    else take.marks.set(part, keep);
    for (const button of take.segments.get(part)) {
      button.setAttribute("aria-checked", String(take.marks.get(part) === (button.dataset.keep === "true")));
    }
  };

  const expand = (vote, take) => {
    vote.full = take;
    vote.element.classList.add("armada-expanded");
    for (const one of vote.takes) {
      one.element.classList.toggle("armada-full", one === take);
      one.expand.setAttribute("aria-expanded", String(one === take));
    }
    take.element.focus();
  };

  const collapse = (vote) => {
    const take = vote.full;
    vote.full = null;
    vote.element.classList.remove("armada-expanded");
    for (const one of vote.takes) {
      one.element.classList.remove("armada-full");
      one.expand.setAttribute("aria-expanded", "false");
    }
    take?.element.focus();
  };

  const resultOf = (vote) => {
    const notes = {};
    for (const take of vote.takes) {
      const words = take.note?.value.trim();
      if (words && take.id !== vote.picked) notes[take.id] = words;
    }
    if (vote.form === "pick") return vote.picked ? { form: "pick", take: vote.picked, ...(Object.keys(notes).length ? { notes } : {}) } : null;
    if (vote.form === "rank") return { form: "rank", order: vote.takes.map((take) => take.id) };
    const marks = vote.takes.flatMap((take) => [...take.marks].map(([part, keep]) => ({ take: take.id, part, keep })));
    return { form: "score", marks };
  };

  const submit = (vote) => {
    const result = resultOf(vote);
    if (!result) {
      vote.status.className = "armada-vote-status";
      vote.status.textContent = "Pick a take first";
      return;
    }
    vote.send.disabled = true;
    vote.status.textContent = "";
    post({ kind: "vote", vote: vote.id, result });
  };

  const voted = (cast) => {
    const vote = votes.get(cast.vote);
    if (!vote) return;
    const { result } = cast;
    if (result.form === "pick" && vote.form === "pick") {
      for (const take of vote.takes) take.note.value = result.notes?.[take.id] ?? take.note.value;
      pick(vote, result.take);
    } else if (result.form === "rank" && vote.form === "rank") {
      placeInOrder(vote, result.order);
    } else if (result.form === "score" && vote.form === "score") {
      for (const take of vote.takes) {
        take.marks.clear();
        for (const one of result.marks.filter((one) => one.take === take.id)) take.marks.set(one.part, one.keep);
        for (const [part, buttons] of take.segments) {
          for (const button of buttons) button.setAttribute("aria-checked", String(take.marks.get(part) === (button.dataset.keep === "true")));
        }
      }
    }
    vote.send.disabled = false;
    vote.status.className = "armada-vote-status armada-done";
    vote.status.textContent = "Sent to the Job";
  };

  const isField = (element) => element.tagName === "TEXTAREA" || element.tagName === "INPUT";

  for (const element of document.querySelectorAll("armada-vote")) {
    const id = said(element, "id");
    const form = said(element, "form");
    const question = said(element, "question");
    const parts = said(element, "parts").split(",").map((part) => part.trim()).filter(Boolean);
    const takeElements = [...element.querySelectorAll("armada-take")].filter((take) => take.closest("armada-vote") === element);
    element.dataset.form = form;
    const list = make("div", { class: "armada-takes", role: "group", "aria-label": question || id });
    const vote = { id, form, element, list, takes: [], picked: null, full: null };
    for (const take of takeElements) {
      const takeId = said(take, "id");
      const title = said(take, "title") || takeId;
      take.tabIndex = 0;
      take.setAttribute("aria-label", title);
      const head = make("div", { class: "armada-take-head" });
      const one = { id: takeId, title, element: take, marks: new Map(), segments: new Map() };
      if (form === "pick") {
        one.radio = make("input", { type: "radio", name: `armada-pick-${id}`, value: takeId, "aria-label": `Pick ${title}` });
        one.radio.addEventListener("change", () => pick(vote, takeId));
        head.append(one.radio);
      }
      if (form === "rank") {
        one.number = make("span", { class: "armada-rank-number" });
        head.append(one.number);
      }
      head.append(make("span", { class: "armada-take-title" }, title));
      if (form === "rank") {
        for (const [word, by, key] of [["Up", -1, "⌥↑"], ["Down", 1, "⌥↓"]]) {
          const button = make("button", { type: "button", class: "armada-move", title: `Move ${word.toLowerCase()} (${key})`, "aria-label": `Move ${title} ${word.toLowerCase()}` }, word);
          button.addEventListener("click", () => move(vote, one, by));
          head.append(button);
        }
      }
      one.expand = make("button", { type: "button", class: "armada-expand", title: "Open full width (F)", "aria-label": `Open ${title} full width`, "aria-expanded": "false" }, "Expand");
      one.expand.addEventListener("click", () => (vote.full === one ? collapse(vote) : expand(vote, one)));
      head.append(one.expand);
      take.prepend(head);
      if (form === "pick") {
        one.note = make("textarea", { class: "armada-note", "aria-label": `Note on ${title}` });
        one.note.hidden = true;
        take.append(one.note);
      }
      if (form === "score") {
        const rows = make("div", { class: "armada-parts" });
        for (const part of parts) {
          const group = make("div", { class: "armada-segmented", role: "radiogroup", "aria-label": `${part} of ${title}` });
          const buttons = [["Keep", true], ["Drop", false]].map(([word, keep]) => {
            const button = make("button", { type: "button", role: "radio", "aria-checked": "false", "data-keep": String(keep) }, word);
            button.addEventListener("click", () => mark(one, part, keep));
            return button;
          });
          group.append(...buttons);
          one.segments.set(part, buttons);
          rows.append(make("span", { class: "armada-part" }, part), group);
        }
        take.append(rows);
      }
      vote.takes.push(one);
      list.append(take);
    }
    if (question) element.prepend(make("p", { class: "armada-vote-question" }, question));
    vote.send = make("button", { type: "button", class: "armada-send", title: "Send (⌘↵)" }, "Send");
    vote.send.addEventListener("click", () => submit(vote));
    vote.status = make("span", { class: "armada-vote-status", role: "status" });
    const foot = make("div", { class: "armada-vote-foot" });
    foot.append(vote.send, vote.status);
    element.append(list, foot);
    if (form === "rank") renumber(vote);
    element.addEventListener("keydown", (event) => {
      if (sendKey(event)) {
        event.preventDefault();
        submit(vote);
        return;
      }
      if (event.key === "Escape" && vote.full) {
        event.preventDefault();
        collapse(vote);
        return;
      }
      if (vote.full && (event.key === "ArrowLeft" || event.key === "ArrowRight") && !isField(event.target)) {
        event.preventDefault();
        const at = vote.takes.indexOf(vote.full) + (event.key === "ArrowRight" ? 1 : -1);
        if (at >= 0 && at < vote.takes.length) expand(vote, vote.takes[at]);
        return;
      }
      const take = vote.takes.find((one) => one.element.contains(event.target));
      if (!take) return;
      if (form === "rank" && event.altKey && (event.key === "ArrowUp" || event.key === "ArrowDown")) {
        event.preventDefault();
        move(vote, take, event.key === "ArrowUp" ? -1 : 1);
        return;
      }
      const plain = !event.metaKey && !event.ctrlKey && !event.altKey;
      if (plain && event.target === take.element && (event.key === "f" || event.key === "F" || event.key === " ")) {
        event.preventDefault();
        expand(vote, take);
      }
    });
    if (!votes.has(id)) votes.set(id, vote);
  }

  // The host's answers.
  const refused = (message) => {
    const action = actions.get(message.action);
    if (action) {
      action.button.disabled = false;
      const line = lineOf(action);
      line.classList.add("armada-refused");
      line.textContent = message.message;
    }
    const vote = votes.get(message.vote);
    if (vote) {
      vote.send.disabled = false;
      vote.status.className = "armada-vote-status armada-refused";
      vote.status.textContent = message.message;
    }
  };

  window.addEventListener("message", (event) => {
    const message = event.data;
    if (event.source !== window.parent || !message || message.armada !== 1) return;
    if (message.kind === "pressed" && message.press) pressed(message.press);
    else if (message.kind === "voted" && message.cast) voted(message.cast);
    else if (message.kind === "refused") refused(message);
  });

  for (const one of record.presses) pressed(one);
  for (const one of record.votes) voted(one);
})();
