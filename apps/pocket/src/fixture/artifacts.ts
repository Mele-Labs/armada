// The Job `Pick the step bar` as Fleet serves its artifacts, for the fixture Gateway and the acceptance test: one artifact
// of every part kind still asking (words, takes with a mock among them, a drawing, buttons and a page that holds a vote),
// one already answered, which the thread folds to a line, and a Ready to try card. The page is sealed as Fleet seals it,
// with the kit's own script, so its vote posts to the phone as it does on a Mac.

import { readFileSync } from "node:fs";
import type { JobArtifact, ThreadReadyToTry, WaitingItem } from "../wire";

export const PARTS_JOB = "f2";
export const STEP_BAR = "sketch:Step bar";
export const SLOT_COLOURS = "sketch:Slot colours";
export const DESIGN_PAGE = "/Users/user/armada/.armada/documents/step-bar.html";
export const TRY_CARD = "try:armada/step-bar@4e735d6c1f0a9b2e7d3c5a8f0e1b2c3d4e5f6a7b";

const at = (now: number, minutes: number) => new Date(now - minutes * 60_000).toISOString();

const scene = (value: unknown) => JSON.stringify(value);

const boxes = (titles: string[], color: string, down: boolean) => ({
  nodes: titles.map((title, n) => ({
    id: `n${String(n)}`,
    kind: "box",
    x: down ? 0 : n * 180,
    y: down ? n * 90 : 0,
    w: 140,
    h: 48,
    title,
    color,
  })),
  edges: titles.slice(1).map((_, n) => ({
    id: `e${String(n)}`,
    from: `n${String(n)}`,
    to: `n${String(n + 1)}`,
  })),
});

/** A take that is a mock of the screen: a Job's row with its steps as dots under the title. */
const DOTS_MOCK = {
  nodes: [
    {
      id: "row",
      kind: "mock",
      x: 0,
      y: 0,
      w: 360,
      title: "Job row",
      html: '<div class="row" data-part="row"><div class="title">Pick the step bar</div><div class="dots"><i class="on"></i><i class="on"></i><i></i><i></i></div></div>',
      css: ".row{padding:14px 16px;border-radius:10px;background:#1e2128;color:#e6e8ee;font:14px -apple-system,sans-serif}.title{font-weight:600;margin-bottom:10px}.dots{display:flex;gap:8px}.dots i{width:12px;height:12px;border-radius:50%;background:#343844}.dots i.on{background:#3fae84}",
    },
  ],
  edges: [],
};

const FLOW = {
  nodes: [
    {
      id: "plan",
      kind: "box",
      x: 0,
      y: 0,
      w: 130,
      h: 44,
      title: "Plan",
      color: "green",
    },
    {
      id: "build",
      kind: "box",
      x: 200,
      y: 0,
      w: 130,
      h: 44,
      title: "Build",
      color: "blue",
    },
    {
      id: "walk",
      kind: "box",
      x: 400,
      y: 0,
      w: 130,
      h: 44,
      title: "Walk",
      color: "amber",
    },
    {
      id: "land",
      kind: "box",
      x: 600,
      y: 0,
      w: 130,
      h: 44,
      title: "Land",
      color: "grey",
    },
  ],
  edges: [
    {
      id: "a",
      from: "plan",
      to: "build",
    },
    {
      id: "b",
      from: "build",
      to: "walk",
    },
    {
      id: "c",
      from: "walk",
      to: "land",
    },
  ],
};

export function partsArtifacts(now: number): JobArtifact[] {
  return [
    {
      id: STEP_BAR,
      title: "Step bar",
      form: "sketch",
      revisions: [
        {
          number: 1,
          at: at(now, 14),
          by: "agent",
          form: "sketch",
        },
        {
          number: 2,
          at: at(now, 6),
          by: "agent",
          form: "sketch",
        },
      ],
      state: "asking",
      item: STEP_BAR,
      for_line: {
        text: "Picks how a Job's steps are drawn under its title.",
        links: [],
      },
      parts: [
        {
          kind: "words",
          text: "Each Job shows its workflow as steps under its title. Two ways to draw them, and a third that drops the words.\n\n- **A** keeps every step's name.\n- **B** stacks them, which is taller.\n- **C** is a row of dots.",
          id: "intro",
          asks: true,
          state: "open",
          revision: 1,
          drawn: "intro-1",
          added: at(now, 14),
        },
        {
          kind: "takes",
          question: "Which step bar reads better?",
          form: "pick",
          takes: [
            {
              id: "bar",
              title: "A · Bar under the title",
              scene: scene(
                boxes(
                  [
                    "Plan",
                    "Build",
                    "Walk",
                    "Land",
                  ],
                  "blue",
                  false,
                ),
              ),
            },
            {
              id: "rail",
              title: "B · Rail (Job's pick)",
              scene: scene(
                boxes(
                  [
                    "Plan",
                    "Build",
                    "Walk",
                    "Land",
                  ],
                  "teal",
                  true,
                ),
              ),
            },
            {
              id: "dots",
              title: "C · Dots",
              scene: scene(DOTS_MOCK),
            },
          ],
          id: "bar",
          asks: true,
          state: "open",
          revision: 1,
          drawn: "bar-1",
          added: at(now, 13),
        },
        {
          kind: "drawing",
          scene: scene(FLOW),
          id: STEP_BAR,
          asks: true,
          state: "open",
          revision: 2,
          drawn: "flow-2",
          added: at(now, 12),
        },
        {
          kind: "buttons",
          actions: [
            {
              id: "try",
              act: "message",
              label: "Try it on this Job",
              item: "Try the rail on this Job now",
            },
          ],
          id: "try",
          asks: false,
          state: "open",
          revision: 1,
          drawn: "try-1",
          added: at(now, 11),
        },
        {
          kind: "page",
          path: DESIGN_PAGE,
          id: DESIGN_PAGE,
          asks: false,
          state: "open",
          revision: 2,
          drawn: "2",
          added: at(now, 10),
        },
      ],
    },
    {
      id: SLOT_COLOURS,
      title: "Slot colours",
      form: "sketch",
      revisions: [
        {
          number: 1,
          at: at(now, 40),
          by: "agent",
          form: "sketch",
        },
      ],
      state: "approved",
      parts: [
        {
          kind: "takes",
          question: "Which colour says a slot is free?",
          form: "pick",
          takes: [
            {
              id: "green",
              title: "A · Green",
              scene: scene(
                boxes(
                  [
                    "Free",
                  ],
                  "green",
                  false,
                ),
              ),
            },
            {
              id: "grey",
              title: "B · Grey",
              scene: scene(
                boxes(
                  [
                    "Free",
                  ],
                  "grey",
                  false,
                ),
              ),
            },
          ],
          id: "colour",
          asks: true,
          state: "picked",
          revision: 1,
          drawn: "colour-1",
          added: at(now, 40),
          answer: {
            take: "green",
            words: "Green, and keep the grey for held.",
            at: at(now, 35),
          },
        },
        {
          kind: "words",
          text: "Free slots read green on the pool band.",
          id: "why",
          asks: false,
          state: "open",
          revision: 1,
          drawn: "why-1",
          added: at(now, 40),
        },
      ],
    },
  ];
}

const kit = (name: string) =>
  readFileSync(
    new URL(`../../../../crates/fleet/src/session_host/${name}`, import.meta.url),
    "utf8",
  );
/** The tokens Fleet writes into a sealed page's `:root`, read from the file they are generated into. */
const tokens = () =>
  readFileSync(new URL("../../../../packages/tokens/tokens.css", import.meta.url), "utf8").replace(
    /@import[^;]*;/g,
    "",
  );

/** A revision of the design page as Fleet serves it sealed: the kit's look and script, and the record they read. */
export function sealedPage(revision: number): string {
  const record = JSON.stringify({
    artifact: DESIGN_PAGE,
    revision,
    presses: [],
    votes: [],
  });
  const vote =
    revision < 2
      ? ""
      : `<armada-vote id="label" form="pick" question="Should a step's name show on hover?">
<armada-take id="hover" title="On hover"><p>The name shows when the pointer rests on a step.</p></armada-take>
<armada-take id="always" title="Always"><p>Every step's name is written under it.</p></armada-take>
</armada-vote>`;
  return `<!doctype html><html><head><meta charset="utf-8"><title>Step bar</title><style>${tokens()}
${kit("page_kit.css")}
body{font:15px -apple-system,sans-serif;margin:16px;color:var(--fg-default);background:var(--bg-base)}</style></head><body>
<h1>Step bar</h1>
<p>${revision < 2 ? "The step bar sits under the title." : "The step bar sits under the title, and a step's name can wait for a hover."}</p>
${vote}
<h2>Detail</h2><p>The bar takes the Job's own steps, in the workflow's order, and colours each by its state.</p>
<script type="application/json" id="armada-page-record">${record}</script><script>${kit("page_kit.js")}</script></body></html>`;
}

/** A shot of the built row, drawn as an SVG so the fixture carries no file. */
const BUILT_SHOT = btoa(
  `<svg xmlns="http://www.w3.org/2000/svg" width="720" height="200" viewBox="0 0 720 200"><rect width="720" height="200" rx="16" fill="#1e2128"/><text x="32" y="64" font-family="-apple-system,sans-serif" font-size="28" font-weight="600" fill="#e6e8ee">Pick the step bar</text>${[
    0,
    1,
    2,
    3,
  ]
    .map(
      (n) =>
        `<rect x="${String(32 + n * 168)}" y="104" width="148" height="56" rx="10" fill="${n < 2 ? "#24443a" : "#262a33"}"/><text x="${String(52 + n * 168)}" y="140" font-family="-apple-system,sans-serif" font-size="20" fill="#e6e8ee">${[
          "Plan",
          "Build",
          "Walk",
          "Land",
        ][n]!}</text>`,
    )
    .join("")}</svg>`,
);

export const SHOTS: Record<
  string,
  {
    kind: "picture";
    bytes: string;
    type: string;
  }
> = {
  "shots/step-bar.svg": {
    kind: "picture",
    bytes: BUILT_SHOT,
    type: "image/svg+xml",
  },
};

export function tryCard(now: number): ThreadReadyToTry {
  return {
    id: TRY_CARD,
    at: at(now, 3),
    offered_at: at(now, 3),
    branch: "armada/step-bar",
    tip: "4e735d6c1f0a9b2e7d3c5a8f0e1b2c3d4e5f6a7b",
    about: "Steps drawn as a bar under each Job's title",
    look: "Open any Job in the list: the bar is under its title.",
    steps: [
      {
        say: "Press Refresh preview on your Mac",
        act: "refresh_preview",
      },
      {
        say: "Open any Job",
      },
      {
        say: "Look under its title",
      },
    ],
    approved: {
      artifact: SLOT_COLOURS,
      title: "Slot colours",
      form: "sketch",
      revision: 1,
      approved_at: at(now, 35),
    },
    pictures: Object.keys(SHOTS),
    words: "Show each Job's steps, so I can see where it is at a glance.",
    in_build: true,
    state: "open",
  };
}

/** A second Job's open Ready to try card, so To review has two Jobs to try in one card: the `rv` Job's, with its item. */
export const TRY_TO_REVIEW = "try:armada/pocket-to-review@28ecff5d1bcd1e481a2db9e0c3f26d9828ac2ec5";
export function toReviewTry(now: number): {
  card: ThreadReadyToTry;
  item: WaitingItem;
} {
  return {
    card: {
      id: TRY_TO_REVIEW,
      at: at(now, 2),
      offered_at: at(now, 2),
      branch: "armada/pocket-to-review",
      tip: "28ecff5d1bcd1e481a2db9e0c3f26d9828ac2ec5",
      about: "Pocket opens on To review, every Job's asks answered in place",
      look: "Open Armada on the phone: it opens on To review.",
      steps: [
        {
          say: "Press Refresh preview on your Mac",
          act: "refresh_preview",
        },
      ],
      in_build: true,
      state: "open",
    },
    item: {
      id: TRY_TO_REVIEW,
      text: "Try: Pocket opens on To review",
      since: at(now, 2),
      source: "ready_to_try",
    },
  };
}
