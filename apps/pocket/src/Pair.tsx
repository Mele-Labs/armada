// Pairing a Mac: the first, another, or one that forgot the phone. The pairing link from that Mac's
// Bridge (Settings, Phone) names the Mac; opened from the link, its code is already in the address.

import { useState } from "react";
import { Button } from "./kit/components/primitives/Button/Button";

import { refusal } from "./client";
import { macOf, useFleet } from "./fleet";
import { claim, defaultName, keepPaired, linkOf, linkTrouble, waitForConfirm } from "./pairing";
import { go } from "./router";
import { Band, Trouble } from "./ui";
import { YourMacs } from "./YourMacs";

export function Pair({ code: fromUrl, origin: again }: { code: string; origin?: string }) {
  const fleet = useFleet();
  const [phase, setPhase] = useState<"link" | "waiting">("link");
  const [name, setName] = useState(() => defaultName(navigator.userAgent));
  const [link, setLink] = useState("");
  const [trouble, setTrouble] = useState(again === undefined ? "" : (macOf(again)?.why ?? ""));
  const here = window.location.origin;
  const target =
    fromUrl !== ""
      ? {
          origin: here,
          code: fromUrl,
        }
      : linkOf(link, here);
  const pair = async () => {
    if (target === undefined) return;
    setTrouble("");
    let machine;
    try {
      machine = await claim(target.origin, name, target.code);
    } catch (why) {
      setTrouble(refusal(why));
      return;
    }
    setPhase("waiting");
    const waited = await waitForConfirm(machine);
    if (waited !== "paired") {
      if (waited === "declined")
        setTrouble("The Mac declined. Start pairing again on the Mac to try once more.");
      setPhase("link");
      return;
    }
    await keepPaired(
      machine,
      fleet.macs.map((mac) => mac.machine),
    );
    go("/", true);
  };
  const paired = fleet.macs.length > 0;
  return (
    <section className="pk-screen" aria-label="Pair">
      <Band
        title="Pair"
        {...(paired
          ? {
              onBack: () => go("/"),
              back: "Home",
            }
          : {})}
      />
      <div className="pk-body pk-pair">
        {phase === "link" && (
          <label className="pk-field">
            <span className="pk-field__label">This phone's name</span>
            <input
              className="pk-line"
              value={name}
              autoCapitalize="words"
              onChange={(event) => setName(event.target.value)}
            />
          </label>
        )}
        <YourMacs name={name} />
        {phase === "link" ? (
          <section className="pk-group pk-with-link" aria-label="With a pairing link">
            <h2 className="pk-group__head">With a pairing link</h2>
            <div className="pk-code" data-phase={phase} aria-hidden="true">
              {Array.from(
                {
                  length: 169,
                },
                (_, i) => (
                  <i key={i} data-on={(i * 7 + (i >> 3) * 5) % 3 === 0 || i % 13 === 0} />
                ),
              )}
            </div>
            {fromUrl === "" && (
              <input
                className="pk-line"
                aria-label="Pairing link"
                placeholder="Pairing link"
                value={link}
                autoCapitalize="none"
                autoCorrect="off"
                spellCheck={false}
                onChange={(event) => setLink(event.target.value)}
              />
            )}
            <Trouble says={trouble || (fromUrl === "" ? linkTrouble(link) : "")} />
            <Button
              variant={target === undefined ? "secondary" : "primary"}
              disabled={target === undefined || name.trim() === ""}
              onClick={() => void pair()}
            >
              Pair
            </Button>
          </section>
        ) : (
          <p className="pk-fact pk-waiting pk-ring pk-ring--line">
            Waiting for Confirm on your Mac
          </p>
        )}
      </div>
    </section>
  );
}
