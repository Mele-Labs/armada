import { Link, Pin } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import type { RunLink } from "../RunControl/runs";

/** A found link as one mark: the glyph in the attention hue, and what is wrong in its tooltip. */
export function RunLinkMark({ link, to }: { link: RunLink; to: string }) {
  const label = `${to} runs on a leased port, not ${link.port}`;
  return (
    <Tooltip label={label}>
      <span className="armada-run-link__mark" role="img" aria-label={label} tabIndex={0}>
        <Link size={12} strokeWidth={2} aria-hidden="true" />
      </span>
    </Tooltip>
  );
}

/** A process's port pinned: the pin and the port. A press undoes it where `onUnpin` is given; without it the mark is a fact. */
export function RunPinned({ port, name, onUnpin }: { port: number; name: string; onUnpin?: () => void }) {
  const label = `Pinned to ${port}. A Job's bay can't run it at the same time`;
  const body = (
    <>
      <Pin size={12} strokeWidth={2} aria-hidden="true" />
      {port}
    </>
  );
  return (
    <Tooltip label={label}>
      {onUnpin === undefined ? (
        <span className="armada-run-link__pinned" tabIndex={0} aria-label={label}>
          {body}
        </span>
      ) : (
        <Button variant="ghost" size="sm" aria-label={`Unpin ${name}`} onClick={onUnpin}>
          {body}
        </Button>
      )}
    </Tooltip>
  );
}

/**
 * One found link as a fact with its two acts: which process expects which on what port, and where Fleet found it. **Fix with a Job** hands
 * the change to a Job; **Pin the port** (**Unpin the port** once it is) keeps the other process on that port in this repository. Neither
 * is needed to run. `marked` leads the line with the link's mark, and is off where a row above already carries it.
 */
export function RunLinkLine({
  link,
  from,
  to,
  pinned = false,
  marked = true,
  onFix,
  onPin,
}: {
  link: RunLink;
  /** The names of the two processes, as the run shows them. */
  from: string;
  to: string;
  pinned?: boolean;
  marked?: boolean;
  onFix: () => void;
  onPin: () => void;
}) {
  return (
    <div className="armada-run-link" role="group" aria-label={`${from} expects ${to}`}>
      <span className="armada-run-link__lead">{marked ? <RunLinkMark link={link} to={to} /> : null}</span>
      <p className="armada-run-link__fact">
        <span className="armada-run-link__name">{from}</span> expects <span className="armada-run-link__name">{to}</span> on <code>{link.port}</code>
      </p>
      <div className="armada-run-link__acts">
        <Button variant="secondary" size="sm" onClick={onFix}>
          Fix with a Job
        </Button>
        <Tooltip
          label={
            pinned
              ? `Start ${to} on a leased port again`
              : `Always start ${to} on ${link.port} in this repository. A Job's bay can't run it at the same time`
          }
        >
          <Button variant="secondary" size="sm" onClick={onPin}>
            {pinned ? "Unpin the port" : "Pin the port"}
          </Button>
        </Tooltip>
      </div>
      <code className="armada-run-link__found">{link.found}</code>
    </div>
  );
}
