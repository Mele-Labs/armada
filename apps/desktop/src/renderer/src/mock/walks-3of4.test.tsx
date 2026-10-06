// A quarter of the checked-in walks, played as tests: a walk that stops
// matching the app fails here, rather than being found broken by the person it
// was sent to. `play-walks.ts` says which quarter and why it is four files.
//
// **The walk's own engine plays it**, `walk.ts`, and not this suite's
// locators: what passes here is what the link does in his browser.
// A scratch walk, in `walks/scratch/`, is not here.

import { playWalks } from "./play-walks";

playWalks(3, 4);
