// The files the file panel opens in the mock: what is on this pretend disk, and the one change made to it from outside. Mock only.
// The walk `file-panel` plays them.

import type { PanelFileItem } from "../../../shared/api/files";

export const SETTINGS_PATH = "/Users/user/Library/Application Support/Armada/settings.json";
export const NOTES_PATH = "/Users/user/Development/armada/docs/notes/store-clock.md";
export const CLOCK_PATH = "/Users/user/Development/armada/crates/store/src/clock.rs";
export const PICTURE_PATH = "/Users/user/Development/armada/docs/notes/clock-blocks.png";

export const SETTINGS_FILE: PanelFileItem = { kind: "file", path: SETTINGS_PATH, from: "settings" };
export const NOTES_FILE: PanelFileItem = { kind: "file", path: NOTES_PATH, from: "session" };
export const CLOCK_FILE: PanelFileItem = { kind: "file", path: CLOCK_PATH, from: "picked" };
export const PICTURE_FILE: PanelFileItem = { kind: "file", path: PICTURE_PATH, from: "picked" };

const SETTINGS = `{
  "limits": {
    "dronesAtOnce": 2,
    "checksAtOnce": 4
  },
  "timeouts": {
    "checkSeconds": 900
  },
  "density": "comfortable"
}

`;

const SETTINGS_ELSEWHERE = SETTINGS.replace('"density": "comfortable"', '"density": "compact"');

const NOTES = `# Store clock

The store reads the clock in three places. Each takes the time as an argument now,
so a test can hold it still.

## What changed

- \`Clock\` is a trait with one method, \`now\`.
- \`FixedClock\` answers the same instant every time.
- The sleep in \`flush\` is gone.

## Still open

\`flush_interval\` reads the wall clock. It fails 4 of 30 runs when the second turns.
`;

const CLOCK = `use std::time::{Duration, Instant};

/// Where the store gets the time. A test holds it still.
pub trait Clock {
    fn now(&self) -> Instant;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

pub struct FixedClock {
    at: Instant,
}

impl FixedClock {
    pub fn new(at: Instant) -> Self {
        Self { at }
    }

    pub fn advance(&mut self, by: Duration) {
        self.at += by;
    }
}

impl Clock for FixedClock {
    fn now(&self) -> Instant {
        self.at
    }
}
`;

export const SKILL_PATH = "/Users/user/.agent/skills/humanizer/SKILL.md";
/** The skill the walk `session-loadout` edits from its Loadout row. */
export const DEEP_RESEARCH_PATH = "/Users/user/.harness/skills/deep-research/SKILL.md";
export const IMPLEMENTER_PATH = "/Users/user/.agent/plugins/cache/armada/1.0.0/agents/implementer.md";
export const AGENT_PATH = "/Users/user/.agent/agents/reviewer.md";

const DEEP_RESEARCH = `---
name: deep-research
description: Research a question across many sources, and say which source each claim came from.
---

# Deep research

Write the question down in one sentence before reading anything.

## How to work

1. List the sources worth reading, and why each is.
2. Read each one, and note what it says that bears on the question.
3. Answer the question, citing a source for every claim.

Say where the sources disagree, and leave the disagreement standing.
`;

const SKILL = `---
name: humanizer
description: Rewrite AI-sounding text so it reads like the writer without changing what it says.
---

# Humanizer

Read the draft once for what it says, then again for how it sounds.

## What to remove

- Contrasts of the form "not X but Y".
- A closing line that restates the paragraph.
- Dashes used where a full stop would do.
- Claims bigger than the evidence behind them.

## What to keep

Every fact, number and name in the original. If a sentence loses one, put it back.
`;

const IMPLEMENTER = `---
name: implementer
description: Builds a change whose shape is still being decided: a feature, a design choice, a change that crosses a module boundary.
---

You build the change the brief names.

Read the brief and the code it points at before the first edit.
Make the smallest change that does what the brief asks.
Say what you decided that the brief did not decide for you.
`;

const AGENT = `---
name: reviewer
description: Reads a diff against the brief it was written for and reports what does not hold.
---

You review a change against the brief that asked for it.

Read the brief first, then the diff.
Report each place the diff does not do what the brief says, with the file and line.
Do not suggest changes the brief did not ask for.
`;

const PNG = "iVBORw0KGgoAAAANSUhEUgAAAPAAAACgCAIAAAC9uXYyAAACc0lEQVR42u3TMQ2AQBREwfNDRUnoEYAsKBCBBTCAi3ODjE3uT7IKNvPaNC+RXXePrJ9vZOu2R/Y9R2QpVw1ooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBro4aEcLeKSAgQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmigq4N2tIBHChhooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBro6aEcLeKSAgQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmigi4P+AUj4x8OnQEduAAAAAElFTkSuQmCC";

const bytesOf = (base64: string): Uint8Array => Uint8Array.from(atob(base64), (one) => one.charCodeAt(0));

export type DiskFile = { text?: string; bytes?: Uint8Array; type: string; version: number; elsewhere?: string };

/** A fresh disk for one window: the settings file, a note, a Rust file, a picture, and a skill and a sub agent from Kit. */
export function pretendDisk(): Map<string, DiskFile> {
  return new Map<string, DiskFile>([
    [SETTINGS_PATH, { text: SETTINGS, type: "text/json", version: 1, elsewhere: SETTINGS_ELSEWHERE }],
    [NOTES_PATH, { text: NOTES, type: "text/markdown", version: 1 }],
    // The file the `session-ledger` Session wrote.
    ["/Users/user/armada/docs/spikes/store-clock.md", { text: "# Store clock\n\nThe store reads the clock once per write.\n", type: "text/markdown", version: 1 }],
    [SKILL_PATH, { text: SKILL, type: "text/markdown", version: 1 }],
    [DEEP_RESEARCH_PATH, { text: DEEP_RESEARCH, type: "text/markdown", version: 1 }],
    [IMPLEMENTER_PATH, { text: IMPLEMENTER, type: "text/markdown", version: 1 }],
    [AGENT_PATH, { text: AGENT, type: "text/markdown", version: 1 }],
    [CLOCK_PATH, { text: CLOCK, type: "text/x-rust", version: 1 }],
    [PICTURE_PATH, { bytes: bytesOf(PNG), type: "image/png", version: 1 }],
  ]);
}
