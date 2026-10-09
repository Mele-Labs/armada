// Settings → Phone, over a `PhoneSource`. Draws nothing where the window has none.

import { Card, CardContent, CardHeader, CardTitle, PhonePairing, type PhoneProblem } from "@armada/components";
import { useEffect, useState } from "react";

import { usePhone } from "./phone-source";

/** "2 minutes ago", for the Gateway's last-seen time. */
function ago(at: number, now: number): string {
  const seconds = Math.max(0, Math.round((now - at) / 1000));
  if (seconds < 60) return "just now";
  const unit = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"} ago`;
  if (seconds < 3600) return unit(Math.floor(seconds / 60), "minute");
  if (seconds < 86400) return unit(Math.floor(seconds / 3600), "hour");
  return unit(Math.floor(seconds / 86400), "day");
}

/** The clock, once a second while a code is counting down and once a minute otherwise. */
function useNow(every: number): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), every);
    return () => clearInterval(id);
  }, [every]);
  return now;
}

export function PhoneSettings() {
  const phone = usePhone();
  const now = useNow(1000);
  if (phone === null) return null;
  const { source, state } = phone;
  const problem: PhoneProblem | undefined =
    state.gateway.state === "not_running"
      ? { kind: "not_running" }
      : state.gateway.state === "tailscale"
        ? { kind: "tailscale", said: state.gateway.said }
        : undefined;
  return (
    <Card>
      <CardHeader>
        <CardTitle>Phone</CardTitle>
      </CardHeader>
      <CardContent>
    <PhonePairing
      {...(problem === undefined ? {} : { problem })}
      {...(state.pairing === null ? {} : { url: state.pairing.url, secondsLeft: (state.pairing.expiresAt - now) / 1000 })}
      {...(state.claim === null ? {} : { claim: state.claim })}
      phones={state.phones.map((one) => ({ id: one.id, name: one.name, seen: ago(one.seenAt, now) }))}
      onPair={() => source.pair()}
      onConfirm={() => source.confirm()}
      onUnpair={(id) => source.unpair(id)}
    />
      </CardContent>
    </Card>
  );
}
