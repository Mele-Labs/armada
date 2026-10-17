// A scene as one `.html` file that plays by itself: the drawing as SVG (`export-svg.ts`), its steps as
// JSON, and a short player. Opens in any browser with no network and no Armada. **Nothing in it is
// fetched**: the CSS is inline, the tokens are written in, and the script reads only what is in the page.
//
// The player does what the canvas does: a step lights its boxes and stands the rest back, and a dot
// crosses the step's arrows, looping. With reduced motion it plays nothing, and the button steps.

import { tokenDeclarations, xml, type StepPlan } from "./export-svg";

/** The words the page's controls carry: the canvas rail's own, so the two read the same. */
export const PLAYER_WORDS = { play: "Play the steps", stop: "Stop the steps", next: "Next step", restart: "Restart the steps", steps: "Steps" } as const;

/** What the player reads out of the page. */
export type PlayerData = { pulse: number; steps: StepPlan; words: typeof PLAYER_WORDS };

const PAGE_CSS = `
* { box-sizing: border-box; }
html, body { margin: 0; min-block-size: 100%; }
body { display: flex; flex-direction: column; gap: var(--space-3); padding: var(--space-4); background: var(--bg-base); color: var(--fg-default); font-family: var(--font-sans); font-size: var(--text-base); }
.page-bar { display: flex; align-items: center; justify-content: space-between; gap: var(--space-4); flex-wrap: wrap; }
.page-bar h1 { margin: 0; font-size: var(--text-base); font-weight: var(--weight-heading); }
.page-bar [role="group"] { display: flex; gap: var(--space-2); }
.page-bar button { padding: var(--space-1) var(--space-3); border: var(--edge-active) solid var(--border-default); border-radius: var(--radius-md); background: var(--bg-raised); color: var(--fg-default); font: inherit; cursor: pointer; }
.page-bar button:hover { background: var(--bg-hover); }
.page-bar button[aria-pressed="true"] { border-color: var(--edge-focus); }
.page-bar button:focus-visible { outline: var(--edge-active) solid var(--edge-focus); outline-offset: var(--edge-active); }
main { flex: 1; min-block-size: 0; border: var(--edge-active) solid var(--border-subtle); border-radius: var(--radius-lg); overflow: hidden; background: var(--bg-sunken); }
main svg { display: block; inline-size: 100%; block-size: auto; max-block-size: calc(100vh - var(--space-6) * 3); }
.sk-node, .sk-edge { transition: opacity var(--duration-base) var(--ease); }
.sk-edge__dot { r: var(--space-2); fill: var(--edge-focus); offset-rotate: 0deg; animation: sk-travel calc(var(--duration-pulse) * 0.75) var(--ease) both; }
.armada-sketch-export:not([data-playing]) .sk-edge[data-flow] .sk-edge__line { animation: sk-flow var(--duration-pulse) linear infinite; }
@keyframes sk-flow { to { stroke-dashoffset: calc(var(--space-2) * -2); } }
@keyframes sk-travel { from { offset-distance: 0%; } to { offset-distance: 100%; } }
@media (prefers-reduced-motion: reduce) {
  .sk-edge__dot { display: none; }
  .armada-sketch-export .sk-edge[data-flow] .sk-edge__line { animation: none; stroke-dasharray: none; }
}
`;

/** The player. Plain script on purpose: it is read by a browser that has nothing of Armada's. */
const PLAYER = `
(function () {
  var data = JSON.parse(document.getElementById("sketch-steps").textContent);
  var svg = document.querySelector("svg.armada-sketch-export");
  var play = document.getElementById("sketch-play");
  var restart = document.getElementById("sketch-restart");
  if (!svg || data.steps.length === 0) return;
  var reduced = window.matchMedia && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  var nodes = {}, edges = {};
  svg.querySelectorAll("[data-node]").forEach(function (el) { nodes[el.getAttribute("data-node")] = el; });
  svg.querySelectorAll("[data-edge]").forEach(function (el) { edges[el.getAttribute("data-edge")] = el; });
  var at = -1, timer = 0;
  function flag(el, name, on) { if (on) el.setAttribute(name, ""); else el.removeAttribute(name); }
  function show(index) {
    at = index;
    svg.querySelectorAll(".sk-edge__dot").forEach(function (dot) { dot.remove(); });
    var step = index < 0 ? null : data.steps[index];
    flag(svg, "data-playing", step !== null);
    Object.keys(nodes).forEach(function (id) {
      var el = nodes[id], lit = step !== null && step.nodes.indexOf(id) >= 0;
      // A screen that holds the step in one of its elements shows itself as it stands at that step, and does not stand back.
      var inner = step !== null && !lit && step.inner !== undefined && step.inner.indexOf(id) >= 0;
      flag(el, "data-current", lit);
      flag(el, "data-dim", step !== null && !lit && !inner && !el.hasAttribute("data-ground"));
      el.querySelectorAll(".sk-screen").forEach(function (view) {
        flag(view, "data-off", view.getAttribute("data-at") !== (inner ? String(step.at) : "rest"));
      });
    });
    Object.keys(edges).forEach(function (id) {
      var el = edges[id], active = step !== null && step.edges.indexOf(id) >= 0;
      flag(el, "data-active", active);
      flag(el, "data-dim", step !== null && !active);
      if (active) {
        var dot = document.createElementNS(svg.namespaceURI, "circle");
        dot.setAttribute("class", "sk-edge__dot");
        dot.style.offsetPath = 'path("' + el.querySelector(".sk-edge__line").getAttribute("d") + '")';
        el.appendChild(dot);
      }
    });
    label();
  }
  function label() {
    play.textContent = reduced ? data.words.next : at >= 0 ? data.words.stop : data.words.play;
    play.setAttribute("aria-pressed", at >= 0 ? "true" : "false");
  }
  function tick() {
    var next = at + 1 < data.steps.length ? at + 1 : reduced ? -1 : 0;
    show(next);
    if (!reduced && next >= 0) timer = setTimeout(tick, data.pulse);
  }
  function stop() { clearTimeout(timer); timer = 0; show(-1); }
  play.addEventListener("click", function () {
    if (reduced) { tick(); return; }
    if (at >= 0) stop(); else tick();
  });
  restart.addEventListener("click", function () {
    clearTimeout(timer);
    show(0);
    if (!reduced) timer = setTimeout(tick, data.pulse);
  });
  label();
  if (!reduced) tick();
})();
`;

/** JSON that is safe to sit inside a `<script>`: nothing in it can close the tag or open a comment. */
const inertJson = (value: unknown) => JSON.stringify(value).replace(/</g, "\\u003c").replace(/>/g, "\\u003e").replace(/&/g, "\\u0026");

export type SketchHtmlInput = {
  /** From `sketchSvg` with `standalone: false`, so it holds no namespace address. */
  svg: string;
  title: string;
  tokens: Readonly<Record<string, string>>;
  steps: StepPlan;
  /** One step's length, in ms. */
  pulse: number;
};

export function sketchHtml({ svg, title, tokens, steps, pulse }: SketchHtmlInput): string {
  const data: PlayerData = { pulse, steps, words: PLAYER_WORDS };
  const controls =
    steps.length === 0
      ? ""
      : `<div role="group" aria-label="${PLAYER_WORDS.steps}"><button type="button" id="sketch-play" aria-pressed="false">${PLAYER_WORDS.play}</button><button type="button" id="sketch-restart">${PLAYER_WORDS.restart}</button></div>`;
  return [
    `<!doctype html>`,
    `<html lang="en">`,
    `<head>`,
    `<meta charset="utf-8">`,
    `<meta name="viewport" content="width=device-width, initial-scale=1">`,
    `<title>${xml(title)}</title>`,
    `<style>:root { ${tokenDeclarations({ ...tokens, "--duration-pulse": `${String(pulse)}ms` })} }${PAGE_CSS}</style>`,
    `</head>`,
    `<body>`,
    `<header class="page-bar"><h1>${xml(title)}</h1>${controls}</header>`,
    `<main>${svg}</main>`,
    `<script type="application/json" id="sketch-steps">${inertJson(data)}</script>`,
    `<script>${PLAYER}</script>`,
    `</body>`,
    `</html>`,
    ``,
  ].join("\n");
}
