// The module a Mod Job writes in the mock for the `review-queue` tab, as `validate_mod` hands it to Bridge: the same text a real Job would leave in `tab.js`, plain JavaScript over `host`.
// It is rewritten a step at a time, so the tab on screen changes under the Job's card: the oldest lead, how long each has waited, then the heading says so. Mock only.

/** The module at `step`, 0 to 3. */
export function reviewQueueModule(step: number): string {
  return `export default function Tab({ host }) {
  const { createElement: h, useEffect, useState } = host.React;
  const [rows, setRows] = useState([]);
  useEffect(() => {
    let live = true;
    host.armada.getReviewLists().then((read) => live && setRows(read.ok ? read.value.requested : []));
    return () => { live = false; };
  }, []);
  const waited = (at) => {
    const minutes = Math.max(1, Math.round((Date.now() - Date.parse(at)) / 60000));
    return minutes < 60 ? minutes + "m" : minutes < 1440 ? Math.round(minutes / 60) + "h" : Math.round(minutes / 1440) + "d";
  };
  const repositories = [...new Set(rows.map((one) => one.repository))];
  const order = (a, b) => (${step >= 1} ? a.updated_at.localeCompare(b.updated_at) : b.updated_at.localeCompare(a.updated_at));
  return h("div", { className: "job-pane__scroll" }, repositories.map((repository) =>
    h("section", { key: repository, className: "sv-layer__group", "aria-label": repository },
      h("h4", { className: "sv-layer__group-label" }, ${step >= 3 ? 'repository + " · oldest first"' : "repository"}),
      h("ul", { className: "sv-layer__rows" }, rows.filter((one) => one.repository === repository).sort(order).map((pr) =>
        h("li", { key: pr.number, className: "sv-ledger" },
          h("button", { type: "button", className: "sv-ledger__main", "aria-label": "Pull request #" + pr.number + " " + pr.title, onClick: () => window.open(pr.address) },
            h("span", { className: "sv-ledger__glyph", "aria-hidden": "true" }),
            h("span", { className: "sv-ledger__title" }, h("code", { className: "sv-ledger__num" }, "#" + pr.number), pr.title),
            h("span", { className: "sv-ledger__meta" },
              h("code", { className: "sv-ledger__sub" }, pr.branch),
              h("span", { className: "sv-ledger__word" }, pr.author),
              ${step >= 2 ? 'h("span", { className: "sv-ledger__word" }, waited(pr.updated_at))' : "null"}))))))));
}
`;
}
