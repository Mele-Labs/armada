// The Job's shell as a Terminal tab: + opens a new one and the pane tells it its size, keys reach it, End shell stops it and New shell opens another, and ⌥⇧0 brings the tab forward or opens one. A shell the agent opens
// draws a Terminal tab, closing a tab ends its shell, and Move to window takes it to a window of its own. Through `App` on `session-shell`, whose Fleet holds a fake zsh.
// The whole story is the walk `job-shell`.

import { beforeEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { forgetPanes } from "../job-panes/store";
import {
  closeTerminalWindow,
  s322SessionShell,
  SHELL_JOB,
  shellCalls,
} from "./scenarios/session-shell";
import { mount, onScreen, toSession, unmountAfterEach } from "./testing";
import { timePasses } from "./time-passes";

unmountAfterEach();
// The panes each Job keeps survive a restart, so a test starts with none kept.
beforeEach(() => forgetPanes());

const pane = () =>
  page.getByRole("group", {
    name: "Terminal",
    exact: true,
  });
const threadPane = () =>
  page.getByRole("region", {
    name: "Pane: Thread",
    exact: true,
  });
const terminalPane = () =>
  page.getByRole("region", {
    name: /^Pane: Terminal/,
  });
// The terminal's window, which the mock draws on the page as a dialog named as the real window's title.
const popOut = () =>
  page.getByRole("dialog", {
    name: "Terminal 1: Deploy the store",
    exact: true,
  });
const PROMPT = "slot-1 %";
const PRINTED = "~/Development/armada/.armada/slots/slot-1";
const SERVED = /Local:.*localhost:5173/;

async function open() {
  mount(s322SessionShell);
  await onScreen();
  await toSession(/Deploy the store/);
  await expect.element(threadPane()).toBeVisible();
}

/** + on the thread's pane, then Terminal. */
async function addTerminal() {
  await threadPane()
    .getByRole("button", {
      name: "Add tab",
    })
    .click();
  await page
    .getByRole("menuitem", {
      name: "Terminal",
    })
    .click();
}

test("a Terminal tab opens a new shell and tells it the pane's size, and keys reach it", async () => {
  await open();
  expect(document.querySelector(".armada-terminal")).toBeNull();
  await addTerminal();

  await expect.element(pane().getByText(PROMPT)).toBeVisible();
  const asked = shellCalls()!.opened;
  expect(asked).toHaveLength(1);
  expect(asked[0]!.new).toBe(true);
  // The size the pane fitted, and not Fleet's 80 by 24.
  await expect.poll(() => shellCalls()!.resized.length).toBeGreaterThan(0);
  const { cols, rows } = shellCalls()!.resized.at(-1)!;
  expect([
    cols,
    rows,
  ]).not.toEqual([
    80,
    24,
  ]);
  await expect
    .poll(() => document.activeElement?.getAttribute("aria-label"))
    .toBe("Terminal input");
  await userEvent.keyboard("ls{Enter}");
  await expect.element(pane().getByText(/CONTRIBUTING\.md/)).toBeVisible();
});

test("Ctrl+C in the shell reaches it as the interrupt byte", async () => {
  await open();
  await addTerminal();
  await expect.element(pane().getByText(PROMPT)).toBeVisible();
  await expect
    .poll(() => document.activeElement?.getAttribute("aria-label"))
    .toBe("Terminal input");
  await userEvent.keyboard("{Control>}c{/Control}");
  await expect.poll(() => shellCalls()!.written).toContain("\u0003");
  await expect.element(pane().getByText("^C")).toBeVisible();
});

test("End shell stops the shell, and New shell opens another with a fresh screen", async () => {
  await open();
  await addTerminal();
  await expect.element(pane().getByText(PROMPT)).toBeVisible();
  await userEvent.keyboard("pwd{Enter}");
  await expect
    .element(
      pane().getByText(PRINTED, {
        exact: true,
      }),
    )
    .toBeVisible();

  await pane()
    .getByRole("button", {
      name: "End shell",
    })
    .click();
  await expect.element(pane().getByText("Shell ended")).toBeVisible();
  expect(shellCalls()!.stopped).toEqual([
    "shell-1",
  ]);
  await pane()
    .getByRole("button", {
      name: "New shell",
    })
    .click();
  await expect
    .element(
      pane().getByRole("button", {
        name: "End shell",
      }),
    )
    .toBeVisible();
  await expect.poll(() => shellCalls()!.opened.length).toBe(2);
  expect(pane().element().textContent).not.toContain(PRINTED);
});

test("⌥⇧0 opens a Terminal tab where the Job has none, and brings the pane forward where it has one", async () => {
  await open();
  expect(terminalPane().query()).toBeNull();
  await userEvent.keyboard("{Alt>}{Shift>}[Digit0]{/Shift}{/Alt}");
  await expect.element(pane().getByText(PROMPT)).toBeVisible();
  const tabs = () =>
    page
      .getByRole("tab", {
        name: /^Terminal/,
      })
      .elements().length;
  expect(tabs()).toBe(1);

  await threadPane()
    .getByRole("tab", {
      name: "Thread",
    })
    .click();
  await expect.element(threadPane()).toHaveAttribute("data-focused", "true");
  await userEvent.keyboard("{Alt>}{Shift>}[Digit0]{/Shift}{/Alt}");
  await expect.element(terminalPane()).toHaveAttribute("data-focused", "true");
  // Brought forward, not opened again.
  expect(tabs()).toBe(1);
  expect(shellCalls()!.opened).toHaveLength(1);
});

test("the agent opening the Job's shell draws a Terminal tab, and the shell ending says so", async () => {
  await open();
  expect(document.querySelector(".armada-terminal")).toBeNull();
  timePasses();

  await expect.element(pane()).toBeVisible();
  await expect.element(pane().getByText(SERVED)).toBeVisible();
  await expect
    .element(
      pane().getByRole("button", {
        name: "End shell",
      }),
    )
    .toBeVisible();
  expect(shellCalls()!.opened).toEqual([]);
  await userEvent.click(
    pane().getByRole("textbox", {
      name: "Terminal input",
    }),
  );
  await userEvent.keyboard("{Control>}c{/Control}");
  await expect.poll(() => shellCalls()!.written).toContain("\u0003");
  await userEvent.keyboard("exit{Enter}");
  await expect.element(pane().getByText("Shell ended")).toBeVisible();
});

test("the agent's terminal arrives as a tab behind, the focus stays on the pane it was in, and the person's own tab is not doubled", async () => {
  await open();
  await addTerminal();
  await expect.element(pane().getByText(PROMPT)).toBeVisible();
  await page
    .getByRole("tab", {
      name: "Thread",
      exact: true,
    })
    .click();
  await expect.element(threadPane()).toHaveAttribute("data-focused", "true");
  timePasses();
  await expect
    .element(
      page.getByRole("tab", {
        name: "Terminal 2",
        exact: true,
      }),
    )
    .toBeVisible();
  await expect.element(threadPane()).toHaveAttribute("data-focused", "true");
  expect(
    page
      .getByRole("tab", {
        name: /^Terminal/,
      })
      .elements(),
  ).toHaveLength(2);
});

test("closing the Terminal tab ends its shell, and a new tab is a new shell with a fresh screen", async () => {
  await open();
  await addTerminal();
  await expect.element(pane().getByText(PROMPT)).toBeVisible();
  await userEvent.keyboard("pwd{Enter}");
  await expect
    .element(
      pane().getByText(PRINTED, {
        exact: true,
      }),
    )
    .toBeVisible();

  await page
    .getByRole("button", {
      name: /^Close Terminal/,
    })
    .click();
  await expect.element(terminalPane()).not.toBeInTheDocument();
  expect(document.querySelector(".armada-terminal")).toBeNull();
  expect(shellCalls()!.stopped).toEqual([
    "shell-1",
  ]);

  await addTerminal();
  await expect.element(pane().getByText(PROMPT)).toBeVisible();
  expect(pane().element().textContent).not.toContain(PRINTED);
  expect(shellCalls()!.opened).toHaveLength(2);
  expect(shellCalls()!.started).toHaveLength(2);
});

/** + on a pane that holds a Terminal tab, then Terminal: a second shell. */
async function addAnotherTerminal(paneName: string) {
  await page
    .getByRole("region", {
      name: paneName,
      exact: true,
    })
    .getByRole("button", {
      name: "Add tab",
    })
    .click();
  await page
    .getByRole("menuitem", {
      name: "Terminal",
    })
    .click();
}

test("closing an idle tab ends only its shell, and the other terminal is left as it was", async () => {
  await open();
  await addTerminal();
  await expect.element(pane().getByText(PROMPT)).toBeVisible();
  await addAnotherTerminal("Pane: Terminal 1");
  await expect.poll(() => shellCalls()!.started.length).toBe(2);
  await page
    .getByRole("button", {
      name: "Close Terminal 2",
    })
    .click();
  await expect
    .poll(() => shellCalls()!.stopped)
    .toEqual([
      "shell-2",
    ]);
  await expect
    .element(
      page.getByRole("tab", {
        name: "Terminal 1",
        exact: true,
      }),
    )
    .toBeVisible();
  expect(
    page
      .getByRole("dialog", {
        name: "End Terminal 2",
      })
      .query(),
  ).toBeNull();
});

test("closing a tab with a program running in it asks first, and ends the shell on yes", async () => {
  await open();
  await addTerminal();
  await expect.element(pane().getByText(PROMPT)).toBeVisible();
  timePasses();
  await expect
    .element(
      page.getByRole("tab", {
        name: "Terminal 2",
        exact: true,
      }),
    )
    .toBeVisible();
  await page
    .getByRole("tab", {
      name: "Terminal 2",
      exact: true,
    })
    .click();
  await expect.element(pane().getByText(SERVED)).toBeVisible();
  await page
    .getByRole("button", {
      name: "Close Terminal 2",
    })
    .click();
  await expect.element(page.getByText("A program is running in Terminal 2.")).toBeVisible();
  expect(shellCalls()!.stopped).toEqual([]);
  await page
    .getByRole("button", {
      name: "End terminal",
    })
    .click();
  await expect
    .poll(() => shellCalls()!.stopped)
    .toEqual([
      "shell-2",
    ]);
});

test("closing a tab whose running check failed asks too, rather than lose a program", async () => {
  await open();
  await addTerminal();
  await expect.element(pane().getByText(PROMPT)).toBeVisible();
  const armada = window.armada as unknown as {
    listTerminals: () => Promise<unknown>;
  };
  const was = armada.listTerminals;
  armada.listTerminals = async () => ({
    ok: false,
    outcome: {
      ok: false,
      kind: "unreachable",
    },
  });
  try {
    await page
      .getByRole("button", {
        name: "Close Terminal 1",
      })
      .click();
    await expect.element(page.getByText("A program is running in Terminal 1.")).toBeVisible();
    expect(shellCalls()!.stopped).toEqual([]);
  } finally {
    armada.listTerminals = was;
  }
});

test("renaming a terminal tells Fleet its name, and the tab keeps the name it was given", async () => {
  await open();
  await addTerminal();
  await expect.element(pane().getByText(PROMPT)).toBeVisible();
  await page
    .getByRole("tab", {
      name: /^Terminal/,
    })
    .click({
      button: "right",
    });
  await page
    .getByRole("menuitem", {
      name: "Rename",
    })
    .click();
  const field = page.getByRole("textbox", {
    name: "Rename Terminal 1",
  });
  await field.fill("dev");
  await userEvent.keyboard("{Enter}");
  await expect
    .element(
      page.getByRole("tab", {
        name: "dev",
        exact: true,
      }),
    )
    .toBeVisible();
  expect(shellCalls()!.renamed).toEqual([
    {
      sessionId: SHELL_JOB,
      terminal: "t1",
      name: "dev",
    },
  ]);
});

test("Move to window takes the shell to a window of its own, its tab says where it is, and Move to the pane or the window's Dock brings it back", async () => {
  await open();
  await addTerminal();
  await expect.element(pane().getByText(PROMPT)).toBeVisible();
  await userEvent.keyboard("ls{Enter}");
  await expect.element(pane().getByText(/CONTRIBUTING\.md/)).toBeVisible();

  await page
    .getByRole("tab", {
      name: /^Terminal/,
    })
    .click({
      button: "right",
    });
  await page
    .getByRole("menuitem", {
      name: "Move to window",
    })
    .click();
  await expect.element(popOut().getByText(/CONTRIBUTING\.md/)).toBeVisible();
  // One screen sizes the shell: the pane has none while the window has it.
  expect(
    terminalPane()
      .getByRole("textbox", {
        name: "Terminal input",
      })
      .query(),
  ).toBeNull();
  await expect
    .poll(
      () =>
        popOut().element().contains(document.activeElement) &&
        document.activeElement?.getAttribute("aria-label"),
    )
    .toBe("Terminal input");
  expect(shellCalls()!.windows).toEqual([
    {
      sessionId: SHELL_JOB,
      terminal: "t1",
    },
  ]);

  await terminalPane()
    .getByRole("button", {
      name: "Move to the pane",
    })
    .click();
  await expect.element(popOut()).not.toBeInTheDocument();
  await expect.element(pane().getByText(/CONTRIBUTING\.md/)).toBeVisible();

  await page
    .getByRole("tab", {
      name: /^Terminal/,
    })
    .click({
      button: "right",
    });
  await page
    .getByRole("menuitem", {
      name: "Move to window",
    })
    .click();
  await expect.element(popOut()).toBeVisible();
  await popOut()
    .getByRole("button", {
      name: "Dock",
      exact: true,
    })
    .click();
  await expect.element(popOut()).not.toBeInTheDocument();
  await expect.element(pane().getByText(/CONTRIBUTING\.md/)).toBeVisible();

  // The window's own close puts the shell away, and the tab shows it again.
  await page
    .getByRole("tab", {
      name: /^Terminal/,
    })
    .click({
      button: "right",
    });
  await page
    .getByRole("menuitem", {
      name: "Move to window",
    })
    .click();
  await expect.element(popOut()).toBeVisible();
  closeTerminalWindow({
    sessionId: SHELL_JOB,
    terminal: "t1",
  });
  await expect.element(pane().getByText(/CONTRIBUTING\.md/)).toBeVisible();
  expect(shellCalls()!.stopped).toEqual([]);
  expect(shellCalls()!.started).toHaveLength(1);
});
