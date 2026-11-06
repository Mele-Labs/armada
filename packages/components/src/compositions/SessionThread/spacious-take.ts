import { createContext } from "react";

/**
 * The spacious look of a Job's thread, which the app draws. Only the Job view provides it, so a thread drawn
 * anywhere else (a call card, a subagent, a pane) stays as it is.
 */
export const SpaciousTake = createContext(false);
