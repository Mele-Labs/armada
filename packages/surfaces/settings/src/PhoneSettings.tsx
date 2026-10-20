// Settings → Phone, over a `PhoneSource`. Draws nothing where the window has none.

import { Card, CardContent, CardHeader, CardTitle, PhonePairing, useNow, type PhoneProblem } from "@armada/components";

import { usePhone } from "./phone-source";

export function PhoneSettings({ onCopied }: { onCopied?: (value: string) => void }) {
  const phone = usePhone();
  const now = useNow();
  if (phone === null) return null;
  const { source, state } = phone;
  const problem: PhoneProblem | undefined =
    state.gateway.state === "not_running"
      ? { kind: "not_running" }
      : state.gateway.state === "said"
        ? { kind: "said", said: state.gateway.said }
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
      phones={state.phones.map((one) => ({ id: one.id, name: one.name, seenAt: one.seenAt }))}
      onPair={() => source.pair()}
      onConfirm={() => source.confirm()}
      onUnpair={(id) => source.unpair(id)}
      {...(onCopied === undefined ? {} : { onCopied })}
    />
      </CardContent>
    </Card>
  );
}
