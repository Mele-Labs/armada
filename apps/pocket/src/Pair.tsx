// Pairing a Mac: the first, another, or one that forgot the phone. The pairing link from that Mac's
// Bridge (Settings, Phone) names the Mac; opened from the link, its code is already in the address.

import { useState } from "react";
import { LoaderCircle } from "lucide-react";
import { Button } from "@armada/components";

import { refusal } from "./client";
import { addMac, macOf, useFleet } from "./fleet";
import { claim, defaultName, linkOf, named, waitForConfirm } from "./pairing";
import { subscribe } from "./push";
import { go } from "./router";
import { Band, Trouble } from "./ui";

export function Pair({ code: fromUrl, origin: again }: { code: string; origin?: string }) {
  const fleet = useFleet();
  const [phase, setPhase] = useState<"link" | "waiting">("link");
  const [name, setName] = useState(() => defaultName(navigator.userAgent));
  const [link, setLink] = useState("");
  const [trouble, setTrouble] = useState(again === undefined ? "" : (macOf(again)?.why ?? ""));
  const here = window.location.origin;
  const target = fromUrl !== "" ? { origin: here, code: fromUrl } : linkOf(link, here);
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
    if (!(await waitForConfirm(machine))) {
      setPhase("link");
      return;
    }
    const kept = await named(machine);
    await addMac(kept);
    const held = fleet.macs.map((mac) => (mac.machine.origin === kept.origin ? kept : mac.machine));
    void subscribe(held.some((one) => one.origin === kept.origin) ? held : [...held, kept]).catch(() => undefined);
    go("/", true);
  };
  const paired = fleet.macs.length > 0;
  return (
    <section className="pk-screen" aria-label="Pair">
      <Band title="Pair" {...(paired ? { onBack: () => go("/"), back: "Home" } : {})} />
      <div className="pk-body pk-pair">
        <div className="pk-code" data-phase={phase} aria-hidden="true">
          {Array.from({ length: 169 }, (_, i) => <i key={i} data-on={((i * 7 + (i >> 3) * 5) % 3 === 0) || i % 13 === 0} />)}
        </div>
        {phase === "link" ? (
          <>
            {fromUrl === "" && (
              <input className="pk-line" aria-label="Pairing link" placeholder="Pairing link" value={link} autoCapitalize="none" autoCorrect="off" spellCheck={false} onChange={(event) => setLink(event.target.value)} />
            )}
            <input className="pk-line" aria-label="Name" value={name} autoCapitalize="words" onChange={(event) => setName(event.target.value)} />
            <Trouble says={trouble} />
          </>
        ) : (
          <p className="pk-fact pk-waiting"><LoaderCircle className="pk-spin" aria-hidden="true" /> Waiting for Confirm on your Mac</p>
        )}
      </div>
      {phase === "link" && (
        <footer className="pk-bar">
          <Button variant="primary" className="pk-big" disabled={target === undefined || name.trim() === ""} onClick={() => void pair()}>Pair</Button>
        </footer>
      )}
    </section>
  );
}
