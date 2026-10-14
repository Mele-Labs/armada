// Set up with a session, from the Manifest's Set up workspaces tab: a storefront whose workspaces
// are in every state the picker draws (already set up, ready to write, no checks proposed, nothing
// here could be read), over a Fleet that also hosts Sessions. The walk `setup-with-a-session` plays it.

import type { Scenario } from "../moment";
import { FakeSessionsFleet } from "../sessions-fleet";
import { SHEET_READ, settingUp } from "../setup-fake";

function build(): Scenario {
  const setting = settingUp({ sheet: SHEET_READ, unreadable: ["test/bats"] });
  const served = new FakeSessionsFleet().scenario(setting);
  return {
    ...served,
    name: "setup-with-a-session",
    says: "Set up workspaces on a storefront in mixed states, and a Session started to set it up",
    // Setup's Fleet and a Sessions one of its own for each window, so a walk told twice starts with none.
    behaves: (handle) => {
      const setup = setting.behaves!(handle);
      const own = new FakeSessionsFleet().scenario(setting).behaves!(handle);
      return {
        ...setup,
        ...own,
        // A Session Fleet starts takes its title from the start, so the fake names it here.
        startSession: async (title, root) => {
          const started = await own.startSession!(title, root);
          if (started.ok && title !== undefined) await own.renameSession!({ session_id: started.value.id, title });
          return started.ok && title !== undefined ? { ...started, value: { ...started.value, title } } : started;
        },
        // A thread is held before the message lands in it, which a window opening the Session does later.
        sendSessionMessage: async (send) => {
          await own.watchSession!(send.session_id);
          return own.sendSessionMessage!(send);
        },
      };
    },
  };
}

export const s237SetupWithASession: Scenario = build();
