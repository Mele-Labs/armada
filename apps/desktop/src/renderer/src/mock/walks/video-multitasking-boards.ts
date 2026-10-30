// The boards for the multitasking film, as one walk: each step puts the real Bridge shell into the state of one shot, from the Projects tab
// on Relay 2.4 to the deck at rest. `hud=1` on the page adds the mission clock and the status stamps (`video-hud.tsx`). The steps are in
// `video-boards-beats.ts`, beside the overlay's text. Over the scenario of the same name.

import { BEATS } from "../video-boards-beats";
import { walk } from "../walk";

const videoMultitaskingBoards = walk("video-multitasking-boards", BEATS.map((one) => one.step));

export { videoMultitaskingBoards as "video-multitasking-boards" };
