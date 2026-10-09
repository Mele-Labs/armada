// The Mods mock: Settings → Theme, the Mods list and a Session that writes a theme. The window is
// `../mods-themes/ModsMock.tsx`, drawn instead of the app because none of it is wired. The walk
// `mods-themes` plays it.

import { onBoard } from "../moment";
import type { Scenario } from "../moment";

export const modsThemes: Scenario = {
  ...onBoard([]),
  name: "mods-themes",
  says: "Theme picker, Mods list and a Session that writes a theme, mock only",
};
