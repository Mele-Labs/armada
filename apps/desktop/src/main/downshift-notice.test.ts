// When the OS is told: the state entering `offered` or `reset`, once for each entry. The words and the
// tone are pinned here too, since they are what the notification is.

import { describe, expect, it } from "vitest";

import type { ModelDownshiftState } from "@armada/protocol";
import { DownshiftNotice } from "./downshift-notice";
import type { Notice } from "./downshift-notice";
import { soundOf } from "./telling";

const offered = (percent: number): ModelDownshiftState => ({ downshift: { phase: "offered", percent, was: "opus", to: "sonnet" } });
const SHIFTED: ModelDownshiftState = { downshift: { phase: "shifted", was: "opus", to: "sonnet", until: "2026-10-18T00:00:00Z" } };
const RESET: ModelDownshiftState = { downshift: { phase: "reset", was: "opus", to: "sonnet" } };
const NONE: ModelDownshiftState = {};

function watching(): { notice: DownshiftNotice; shown: Notice[] } {
  const shown: Notice[] = [];
  return { notice: new DownshiftNotice({ show: (one) => shown.push(one) }), shown };
}

describe("the offer", () => {
  it("is told once when the state enters offered, as bare facts", () => {
    const { notice, shown } = watching();
    notice.read(offered(83));
    expect(shown).toEqual([{ title: "Default to sonnet offered", body: "Weekly limit 83% used", sound: soundOf("waiting") }]);
  });

  it("is not told again while the percent moves", () => {
    const { notice, shown } = watching();
    notice.read(offered(83));
    notice.read(offered(84));
    notice.read(offered(97));
    expect(shown).toHaveLength(1);
  });

  it("is told again for the next window, after the state has been none", () => {
    const { notice, shown } = watching();
    notice.read(offered(83));
    notice.read(NONE);
    notice.read(offered(81));
    expect(shown).toHaveLength(2);
  });

  it("is not told for accepting it", () => {
    const { notice, shown } = watching();
    notice.read(offered(83));
    notice.read(SHIFTED);
    expect(shown).toHaveLength(1);
  });
});

describe("the reset", () => {
  it("is told once when the state enters reset, and not for what comes after it", () => {
    const { notice, shown } = watching();
    notice.read(SHIFTED);
    notice.read(RESET);
    notice.read(RESET);
    expect(shown).toEqual([{ title: "Back to opus offered", body: "Weekly limit reset", sound: soundOf("waiting") }]);
    notice.read(NONE);
    expect(shown).toHaveLength(1);
  });
});

describe("the state found on connecting", () => {
  it("tells nobody of an offer that was already standing, or of its percent moving after", () => {
    const { notice, shown } = watching();
    notice.seed(offered(83));
    notice.read(offered(85));
    expect(shown).toEqual([]);
  });

  it("is where a later entry is measured from", () => {
    const { notice, shown } = watching();
    notice.seed(SHIFTED);
    notice.read(RESET);
    expect(shown).toHaveLength(1);
  });

  it("is the tone a job waiting for review takes, never the one for work that stopped", () => {
    expect(soundOf("waiting")).not.toBe(soundOf("blocked"));
    const { notice, shown } = watching();
    notice.read(offered(83));
    expect(shown[0]?.sound).toBe(soundOf("waiting"));
  });
});
