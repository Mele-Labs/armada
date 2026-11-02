// Mock-only: three instruments as `instrument.js` texts, the same a real Job leaves for Fleet to check, plain JavaScript over `host`. Reviews draws, Spend throws, and a Job's
// Main build starts as Fleet's starter and is rewritten when the Job finishes.

/** What `scaffold_mod` writes for an instrument (`crates/fleet/src/mods/scaffold.rs`). */
export const STARTER = `const paint = (draw, { w, h, ink }) => {
  draw.fillStyle = ink.off;
  draw.fillRect(8, h / 2 - 2, w - 16, 4);
  draw.fillStyle = ink.run;
  draw.fillRect(8, h / 2 - 2, (w - 16) / 3, 4);
};

export default function Instrument({ host }) {
  const { createElement: el, useEffect } = host.React;
  useEffect(() => {
    host.face(paint);
  }, []);
  const bar = (width, at) => el("div", { key: at, style: { width, height: "var(--space-2)", background: "color-mix(in srgb, var(--fg-default) 7%, transparent)" } });
  return el("div", { style: { display: "grid", gap: "var(--space-3)", padding: "var(--space-4)" } }, ["40%", "90%", "75%"].map(bar));
}
`;

// The rows both panels list, in tokens only: a lamp, a figure, what it is and how old.
const ROWS = `const LIST = { margin: 0, padding: 0, listStyle: "none", display: "grid", gap: "var(--space-2)" };
const ROW = { display: "grid", gridTemplateColumns: "var(--space-3) var(--space-12) 1fr auto", alignItems: "center", gap: "var(--space-3)" };
const LAMP = { new: "var(--status-running)", due: "var(--status-awaiting-review)", old: "var(--status-completed-failed)" };
const rows = (el, list) =>
  el("ul", { style: LIST }, list.map((one) =>
    el("li", { key: one.figure, style: ROW },
      el("i", { style: { width: "var(--space-3)", height: "var(--space-3)", background: LAMP[one.lamp] } }),
      el("code", { style: { fontFamily: "var(--font-mono)" } }, one.figure),
      el("span", null, one.what),
      one.age === undefined ? null : el("time", { style: { fontFamily: "var(--font-mono)", color: "var(--fg-muted)" } }, one.age))));
`;

/** A still face: lamps by age and the oldest's number. Drawn once. */
export const REVIEWS = `${ROWS}
const WAITING = [
  { number: 3171, title: "Seat a mod's instrument in a bay", age: 22 },
  { number: 3168, title: "Keep the rack's tray to one row", age: 9 },
  { number: 3164, title: "Fold the merge queue with the console", age: 5 },
  { number: 3160, title: "Name the Calls HUD by its Job", age: 2 },
  { number: 3158, title: "Draw hi-fi sketches with the repo's stylesheets", age: 1 },
];
const hue = (age, ink) => (age >= 12 ? ink.bad : age >= 4 ? ink.ask : ink.run);

const paint = (draw, { w, h, ink }) => {
  const lamp = Math.min(h * 0.34, (w - 16) / WAITING.length - 4);
  WAITING.forEach((one, at) => {
    draw.fillStyle = hue(one.age, ink);
    draw.fillRect(8 + at * (lamp + 4), 8, lamp, lamp);
  });
  draw.font = "600 " + Math.round(h * 0.2) + "px ui-monospace, Menlo, monospace";
  draw.textBaseline = "alphabetic";
  draw.fillStyle = ink.fg;
  draw.fillText("#" + WAITING[0].number, 8, h - 8);
  const age = WAITING[0].age + "h";
  draw.fillStyle = hue(WAITING[0].age, ink);
  draw.fillText(age, w - 8 - draw.measureText(age).width, h - 8);
};

export default function Reviews({ host }) {
  const { createElement: el, useEffect } = host.React;
  useEffect(() => {
    host.face(paint);
  }, []);
  return rows(el, WAITING.map((one) => ({ figure: "#" + one.number, what: one.title, age: one.age + "h", lamp: one.age >= 12 ? "old" : one.age >= 4 ? "due" : "new" })));
}
`;

/** Throws as it paints, so its socket stands empty with the reason. */
export const SPEND = `const paint = () => {
  throw new Error("usage.month is undefined");
};

export default function Spend({ host }) {
  host.React.useEffect(() => {
    host.face(paint);
  }, []);
  return null;
}
`;

/** What the Job hands back: one that asks to move, as a bar walks along the runs. */
export const MAIN_BUILD = `${ROWS}
const RUNS = [38, 41, 36, 52, 44, 39, 61, 47, 42, 40, 35, 37, 58, 43, 41, 36];

const paint = (draw, { w, h, t, ink }) => {
  const top = Math.max(...RUNS);
  const each = (w - 16) / RUNS.length;
  const moving = Math.floor(t * 2) % RUNS.length;
  RUNS.forEach((one, at) => {
    const bar = ((h - 16) * one) / top;
    draw.fillStyle = at === moving ? ink.fg : one > 50 ? ink.ask : ink.run;
    draw.fillRect(8 + at * each, h - 8 - bar, Math.max(1, each - 2), bar);
  });
};

export default function MainBuild({ host }) {
  const { createElement: el, useEffect } = host.React;
  useEffect(() => {
    host.face(paint, { moves: true });
  }, []);
  return rows(el, RUNS.slice(-6).map((one, at) => ({ figure: one + "m", what: "ci on main, run " + (at + 1), lamp: one > 50 ? "due" : "new" })));
}
`;
