// The sketch a Job draws of "one card for every list" as hi-fi parts: HTML in the app's own classes over the app's own
// stylesheets, which the mock's host reads from this repository. Mock only: `show_sketch` hands Fleet the same shape, and
// `parseScene` checks it where it is drawn. The walk `hi-fi-sketch` plays it.

import type { Scene, SceneNode } from "@armada/components";

import { arrow, scene } from "../scenes";

export const ONE_CARD_TITLE = "One card";

/** The repository's own stylesheets, by path, that the app's parts are drawn with. The mock's host reads these files. */
export const TOKENS = "packages/tokens/tokens.css";
export const BASE = "packages/tokens/src/base.css";
export const JOB_CARD = "packages/components/src/compositions/JobCard/JobCard.css";

const APP = [TOKENS, BASE, JOB_CARD];

/** What the parts share that the app's stylesheets do not say: the border box the app's reset sets, the panel round the cards, and the two states the card has no tone for. */
const PANEL = `
*, *::before, *::after { box-sizing: border-box; }
.panel { display: flex; flex-direction: column; min-width: 0; border: var(--border-width) solid var(--border-subtle); border-radius: var(--radius-md); background: var(--bg-base); box-shadow: var(--shadow-card); }
.cards { display: flex; flex-direction: column; gap: var(--space-2); padding: var(--space-2); }
.armada-job-card[data-tone="bad"] { --hue: var(--status-completed-failed); }
.armada-job-card[data-tone="good"] { --hue: var(--status-completed-success); }
body { padding: var(--space-4); background: var(--bg-sunken); }
`;

type Card = { tone?: string; kind?: string; title: string; mark: string; issue?: string; says?: string; age: string };

const card = (one: Card) => `
<div class="armada-job-card"${one.tone === undefined ? "" : ` data-tone="${one.tone}"`}>
  <span class="armada-job-card__kind"><i data-icon="${one.kind ?? "bot"}" data-size="14"></i></span>
  <span class="armada-job-card__title">${one.title}</span>
  <span class="armada-job-card__mark"><i data-icon="${one.mark}" data-size="14"></i></span>
  <span class="armada-job-card__says">${one.issue === undefined ? "" : `<span class="armada-job-card__issue"><i data-icon="ticket" data-size="12"></i><code>${one.issue}</code></span>`}${one.says ?? ""}</span>
  <span class="armada-job-card__at">${one.age}</span>
</div>`;

const panel = (cards: Card[]) => `<div class="panel"><div class="cards">${cards.map(card).join("")}</div></div>`;

const mock = (id: string, x: number, y: number, w: number, title: string, html: string, css: string, styles: string[] | undefined): SceneNode => ({
  id, kind: "mock", x, y, w, title, html, ...(css === "" ? {} : { css }), ...(styles === undefined ? {} : { styles }),
});

const ANATOMY = `
<div class="panel">
  <div class="cards">
    ${`<div class="armada-job-card" data-tone="ask" data-part="Card">
      <span class="armada-job-card__kind" data-part="Kind"><i data-icon="bot" data-size="14"></i></span>
      <span class="armada-job-card__title" data-part="Title">Walk bar loses Approve after a reload</span>
      <span class="armada-job-card__mark" data-part="State"><i data-icon="circle-alert" data-size="14"></i></span>
      <span class="armada-job-card__says" data-part="Says"><span class="armada-job-card__issue" data-part="Issue"><i data-icon="ticket" data-size="12"></i><code>#3131</code></span>Drawing the card</span>
      <span class="armada-job-card__at" data-part="Age">12m</span>
    </div>`}
    <dl class="parts">
      <dt>Kind</dt><dd>The glyph for a Job</dd>
      <dt>Title</dt><dd>The only bold line</dd>
      <dt>State</dt><dd>A glyph in the state's hue</dd>
      <dt>Says</dt><dd>The issue, then the last words</dd>
      <dt>Age</dt><dd>Mono, to the right</dd>
    </dl>
  </div>
</div>`;

const ANATOMY_CSS = `${PANEL}
.parts { display: grid; grid-template-columns: auto minmax(0, 1fr); column-gap: var(--space-3); row-gap: var(--space-1); margin: 0; padding: var(--space-1) var(--space-2); font-size: var(--text-xs); line-height: var(--leading-xs); }
.parts dt { color: var(--fg-muted); font-family: var(--font-mono); font-size: var(--text-2xs); }
.parts dd { margin: 0; color: var(--fg-default); }
`;

const LOGBOOK = `
<div class="panel">
  <ul class="log">
    <li><i data-icon="rocket" data-size="14"></i><span class="at">14:40</span><span>Slot 3 leased, branch cut</span></li>
    <li><i data-icon="git-branch" data-size="14"></i><span class="at">14:52</span><span>First commit on fix/walk-bar</span></li>
    <li><i data-icon="circle-alert" data-size="14"></i><span class="at">15:03</span><span>Asked which bar keeps Approve</span></li>
    <li><i data-icon="git-pull-request" data-size="14"></i><span class="at">15:20</span><span>Pull request opened</span></li>
  </ul>
</div>`;

const LOGBOOK_CSS = `${PANEL}
.log { display: flex; flex-direction: column; gap: var(--space-1); margin: 0; padding: var(--space-2) var(--space-3); list-style: none; font-size: var(--text-xs); line-height: var(--leading-xs); }
.log li { display: grid; grid-template-columns: auto auto minmax(0, 1fr); align-items: center; column-gap: var(--space-2); }
.log svg { color: var(--fg-muted); }
.at { color: var(--fg-muted); font-family: var(--font-mono); font-size: var(--text-2xs); font-variant-numeric: tabular-nums; }
`;

const RULES = `
<div class="panel">
  <table class="rules">
    <thead><tr><th>Part</th><th>Carries</th><th>Never</th></tr></thead>
    <tbody>
      <tr><td>Frame</td><td>The state, in hue</td><td>A badge for it</td></tr>
      <tr><td>Left edge</td><td>The same hue, heavier</td><td>A second colour</td></tr>
      <tr><td>Glyph</td><td>What it is now</td><td>A phrase</td></tr>
      <tr><td>Age</td><td>Time, in mono</td><td>A count</td></tr>
    </tbody>
  </table>
</div>`;

const RULES_CSS = `${PANEL}
.rules { inline-size: 100%; border-collapse: collapse; font-size: var(--text-xs); line-height: var(--leading-xs); }
.rules th { padding: var(--space-2) var(--space-3); border-block-end: var(--border-width) solid var(--border-subtle); color: var(--fg-muted); font-size: var(--text-2xs); font-weight: var(--weight-medium); text-align: start; }
.rules td { padding: var(--space-2) var(--space-3); border-block-end: var(--border-width) solid var(--border-subtle); }
.rules tr:last-child td { border-block-end: 0; }
.rules td:first-child { font-weight: var(--weight-heading); }
`;

/** A part with no stylesheet of the app: the plain rules of the agent's own, so its caption says so. */
const PLAIN = `<div class="plain"><strong>Without the app's sheets</strong><p>Browser defaults and a few rules of its own.</p><button>Approve</button></div>`;
const PLAIN_CSS = `.plain { font-family: sans-serif; padding: 8px; background: #eee; color: #222; } .plain p { margin: 4px 0; }`;

export const ONE_CARD: Scene = scene(
  [
    mock("anatomy", 0, 0, 360, "The card", ANATOMY, ANATOMY_CSS, APP),
    mock(
      "issues", 520, 0, 320, "Issues",
      panel([
        { tone: "ask", title: "Walk bar loses Approve after a reload", mark: "circle-alert", issue: "#3131", says: "Drawing the card", age: "12m" },
        { title: "Skeleton rows jump on the Usage page", mark: "circle", issue: "#3098", age: "2d" },
      ]),
      PANEL, APP,
    ),
    mock(
      "prs", 520, 250, 320, "Pull requests",
      panel([
        { tone: "bad", title: "Spell the lengths with tokens", mark: "circle-x", issue: "#3150", age: "4m" },
        { tone: "good", title: "Fix the joblayouts walk holds", mark: "circle-check", issue: "#3149", age: "1h" },
      ]),
      PANEL, APP,
    ),
    mock(
      "jobs", 520, 500, 320, "Cockpit Jobs: asked first",
      panel([{ tone: "live", title: "One card for every list", kind: "bot", mark: "circle-dot", says: "Drawing the card", age: "now" }]),
      PANEL, APP,
    ),
    mock("logbook", 960, 0, 360, "Logbook", LOGBOOK, LOGBOOK_CSS, APP),
    mock("rules", 960, 280, 420, "The one rule", RULES, RULES_CSS, APP),
    mock("plain", 0, 420, 240, "Without a sheet", PLAIN, PLAIN_CSS, undefined),
  ],
  [arrow("anatomy", "issues"), arrow("anatomy", "prs"), arrow("anatomy", "jobs")],
);
