// The button that asks a ring's owner for an update, wherever a ring is drawn: the tall column's header, the drone's pinned card and a
// Job's mission. A bare glyph, so its tooltip names it. After a press it reads sent until the next reply, and a refusal is its tooltip.

import { MessageCircleMore } from "lucide-react";
import { Button, Tooltip } from "@armada/components";

import { useUpdateAsk } from "./ask-update";

/** `subject` is the drone's key: `session:<id>` or `job:<id>`. */
export function AskUpdate({ subject }: { subject: string }) {
  const { held, press } = useUpdateAsk(subject);
  const sent = held?.state === "sent";
  const label = sent ? "Update asked" : held?.state === "refused" ? held.said : "Ask for an update";
  return (
    <span className="bf-ask" data-sent={sent || undefined}>
      <Tooltip label={label} asChild>
        <Button variant="ghost" size="sm" iconOnly aria-label={label} aria-pressed={sent} pending={held?.state === "sending"} {...(held?.state === "refused" ? { answer: "refused" as const } : {})} onClick={press}>
          <MessageCircleMore size={16} aria-hidden="true" />
        </Button>
      </Tooltip>
    </span>
  );
}
