import { createContext } from "react";

/**
 * The spacious take on a Job's thread (`?thread=spacious` in the mock). Only the Job view provides it, so a thread drawn
 * anywhere else (a call card, a subagent, a pane) stays as it is.
 */
export const SpaciousTake = createContext(false);
