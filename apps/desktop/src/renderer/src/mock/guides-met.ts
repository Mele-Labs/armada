// Every guide already met, before each browser test.
//
// **A test is not a first-time reader.** A card opening by itself is correct
// behaviour — the owner's own, #1603 — and it is a framed layer with a scrim,
// so in a test about something else it swallows the first press. Eight tests
// across six files went that way the moment the marks landed, each failing on
// a click the card was over.
//
// So the window every other test opens is one whose person has been here
// before. `guides.test.tsx` clears this in its own `beforeEach` to become a
// first-time reader, which is what makes that file the one place the claim is
// made and checked.

import { beforeEach } from "vitest";

import { forgetHowItWasRead, meetEveryGuide } from "./remembered";

/**
 * **A test starts from a window that remembers nothing**, unless it says
 * otherwise. The keys `remembered.ts` lists survive between files in one
 * browser origin, so a test that assumes a default is really asserting on
 * whatever ran before it — and which file that is changes with how the
 * workers are scheduled.
 *
 * Three tests paid for this on 30 Sep 2026, all reading as load flakes and
 * none of them load: `canvas-pan` arriving after a test that left Plan on
 * **List** had no canvas to pan, and `board-gone` arriving after one that
 * expanded **Done** found `Collapse Done` where it wanted `Expand Done`.
 * `job-detail-plan-views` and `job-detail-workflow` were already clearing
 * their own two by hand, which is the same fix made three files too narrow.
 *
 * `guides.test.tsx` clears the guides key in its own `beforeEach` to become a
 * first-time reader, which is what keeps that one claim in one place.
 */
beforeEach(forgetHowItWasRead);

beforeEach(meetEveryGuide);
