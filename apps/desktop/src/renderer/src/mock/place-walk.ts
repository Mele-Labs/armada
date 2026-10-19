// What the first-run walks look at for where Armada configuration is kept, which follows `?take=`. Mock only.

import { placeTake } from "./first-run";
import { button, role } from "./walk";

export const take = placeTake();
/** The menu of places, or the card that lists them at the top of the thread. */
export const places = role("group", "Where Armada configuration is kept");
/** The first Job's chip, which is the card in the thread take. */
export const chip = take === "thread" ? places : button(/^Armada configuration: On this Mac/);
/** Where the first Job shows it. */
export const where = take === "thread" ? "a card at the top of its thread" : take === "toolbar" ? "a chip in its box's toolbar, after Effort" : "a chip in its header";
