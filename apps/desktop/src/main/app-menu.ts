// The application menu: Electron's default, with one item added to the app menu after About. A pure
// function of its inputs so a test reads it without Electron. The item runs in main, so it works
// with the page blank — `revert-asked.ts`.

import type { MenuItemConstructorOptions } from "electron";

export const REVERT_ITEM = "Revert to the build before…";

/**
 * What the app menu is titled. **Not `app.name`**: that is `@armada/desktop`, from `package.json`, and
 * setting `productName` there would move `userData` (window place, held notes, threads) to a new folder.
 */
export const APP_NAME = "Armada Classic";

/**
 * `name` is the app's. **The app menu is spelled out rather than the `appMenu` role**, because the
 * role is one item that expands to a fixed list and takes no addition after About.
 */
export function appMenuTemplate(
  name: string,
  revert: {
    enabled: boolean;
    click: () => void;
  },
): MenuItemConstructorOptions[] {
  return [
    {
      label: name,
      submenu: [
        {
          role: "about",
        },
        {
          label: REVERT_ITEM,
          enabled: revert.enabled,
          click: revert.click,
        },
        {
          type: "separator",
        },
        {
          role: "services",
        },
        {
          type: "separator",
        },
        {
          role: "hide",
        },
        {
          role: "hideOthers",
        },
        {
          role: "unhide",
        },
        {
          type: "separator",
        },
        {
          role: "quit",
        },
      ],
    },
    {
      role: "fileMenu",
    },
    {
      role: "editMenu",
    },
    {
      role: "viewMenu",
    },
    {
      role: "windowMenu",
    },
  ];
}
