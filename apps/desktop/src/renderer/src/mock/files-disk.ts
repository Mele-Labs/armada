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

const PNG = "iVBORw0KGgoAAAANSUhEUgAAAPAAAACgCAIAAAC9uXYyAAACc0lEQVR42u3TMQ2AQBREwfNDRUnoEYAsKBCBBTCAi3ODjE3uT7IKNvPaNC+RXXePrJ9vZOu2R/Y9R2QpVw1ooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBro4aEcLeKSAgQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmigq4N2tIBHChhooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBro6aEcLeKSAgQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmiggQYaaKCBBhpooIEGGmigi4P+AUj4x8OnQEduAAAAAElFTkSuQmCC";

const bytesOf = (base64: string): Uint8Array => Uint8Array.from(atob(base64), (one) => one.charCodeAt(0));

export type DiskFile = { text?: string; bytes?: Uint8Array; type: string; version: number; elsewhere?: string };

/** A fresh disk for one window: the settings file, a note, a Rust file and a picture. */
export function pretendDisk(): Map<string, DiskFile> {
  return new Map<string, DiskFile>([
    [SETTINGS_PATH, { text: SETTINGS, type: "text/json", version: 1, elsewhere: SETTINGS_ELSEWHERE }],
    [NOTES_PATH, { text: NOTES, type: "text/markdown", version: 1 }],
    [CLOCK_PATH, { text: CLOCK, type: "text/x-rust", version: 1 }],
    [PICTURE_PATH, { bytes: bytesOf(PNG), type: "image/png", version: 1 }],
  ]);
}
