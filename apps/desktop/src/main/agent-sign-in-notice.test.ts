// When the OS is told the agents are signed out: once for each spell, named by its `signed_out_since`.

import { describe, expect, it } from "vitest";

import type { AgentSignIn } from "@armada/protocol";
import type { Notice } from "./downshift-notice";
import { AgentSignInNotice } from "./agent-sign-in-notice";
import { soundOf } from "./telling";

const out = (since: string, signing_in = false): AgentSignIn => ({ signed_out_since: since, signing_in });
const SPELL = "2026-10-25T09:00:00Z";
const IN: AgentSignIn = {};

function watching(): { notice: AgentSignInNotice; shown: Notice[] } {
  const shown: Notice[] = [];
  return { notice: new AgentSignInNotice({ show: (one) => shown.push(one) }), shown };
}

describe("the signed-out notification", () => {
  it("is told once when the state moves from signed in to signed out, with a title and no body", () => {
    const { notice, shown } = watching();
    notice.read(IN);
    notice.read(out(SPELL));
    expect(shown).toEqual([{ title: "Claude is signed out", body: "", sound: soundOf("blocked") }]);
  });

  it("is not told again for the same spell: the press, a failed attempt, or a repeated event", () => {
    const { notice, shown } = watching();
    notice.read(out(SPELL));
    notice.read(out(SPELL));
    notice.read(out(SPELL, true));
    notice.read(out(SPELL));
    expect(shown).toHaveLength(1);
  });

  it("says nothing for signing back in", () => {
    const { notice, shown } = watching();
    notice.read(out(SPELL));
    notice.read(IN);
    expect(shown).toHaveLength(1);
  });

  it("is told again for the next spell", () => {
    const { notice, shown } = watching();
    notice.read(out(SPELL));
    notice.read(IN);
    notice.read(out("2026-10-25T14:00:00Z"));
    expect(shown).toHaveLength(2);
  });
});

describe("the state found on connecting", () => {
  it("tells nobody of a spell already standing, or of its sign-in running after", () => {
    const { notice, shown } = watching();
    notice.seed(out(SPELL));
    notice.read(out(SPELL, true));
    expect(shown).toEqual([]);
  });

  it("is where a later spell is measured from", () => {
    const { notice, shown } = watching();
    notice.seed(out(SPELL));
    notice.read(IN);
    notice.read(out("2026-10-25T14:00:00Z"));
    expect(shown).toHaveLength(1);
  });
});
