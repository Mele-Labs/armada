// What the map draws for a picked event and for a replay: the caption chip with Replay from here, the replay bar along the
// bottom of the map, and the marker in its corner so a replayed field is never taken for a live one. Mock only.
// proto/cockpit-flight-deck.

import type { PointerEvent as ReactPointerEvent } from "react";
import { History, Pause, Play, Radio } from "lucide-react";
import { Button, Tooltip } from "@armada/components";
import type { TimelineEvent } from "@armada/protocol";

import { age } from "../../Dashboard";
import { SPEEDS, inOrder } from "./deck-replay";
import { changeReplay, type Replay } from "./deck-model";

const when = (event: TimelineEvent) => Date.parse(event.at);

/** The picked event, named: who, what it said, and how long ago, with the way into a replay from it. */
export function FocusChip({ event, now, onReplay }: { event: TimelineEvent; now: number; onReplay: () => void }) {
  return (
    <div className="deck-focus" role="region" aria-label="Event">
      <span className="deck-focus__who">{event.title ?? ""}</span>
      <span className="deck-focus__says">{event.says}</span>
      <time className="deck-focus__ago" dateTime={event.at}>
        {age(event.at, now)}
      </time>
      <Tooltip label="Replay from here" asChild>
        <Button variant="ghost" size="sm" iconOnly aria-label="Replay from here" onClick={onReplay}>
          <History size={16} aria-hidden />
        </Button>
      </Tooltip>
    </div>
  );
}

/** Quiet, in the map's corner, for as long as it is a replay. */
export function ReplayMark() {
  return (
    <Tooltip label="Replay, not live">
      <div className="deck-replay-mark" role="status" aria-label="Replay">
        <History size={16} aria-hidden />
        Replay
      </div>
    </Tooltip>
  );
}

/** Play and pause, the speed, the track from where it began to now with a tick for each event, and the way back to Live. */
export function ReplayBar({ events, replay, end, onLive }: { events: readonly TimelineEvent[]; replay: Replay; end: number; onLive: () => void }) {
  const ticks = inOrder(events).filter((one) => when(one) >= replay.from && when(one) <= end);
  const span = Math.max(1, end - replay.from);
  const at = (ms: number) => `${(Math.min(1, Math.max(0, (ms - replay.from) / span)) * 100).toFixed(2)}%`;
  const seek = (event: ReactPointerEvent<HTMLDivElement>) => {
    const box = event.currentTarget.getBoundingClientRect();
    changeReplay({ at: replay.from + Math.min(1, Math.max(0, (event.clientX - box.left) / box.width)) * span, plays: [] });
  };
  return (
    <div className="deck-replay" role="region" aria-label="Replay">
      <Tooltip label={replay.playing ? "Pause" : "Play"} shortcut="Space" asChild>
        <Button variant="ghost" size="sm" iconOnly aria-label={replay.playing ? "Pause" : "Play"} onClick={() => changeReplay({ playing: !replay.playing })}>
          {replay.playing ? <Pause size={16} aria-hidden /> : <Play size={16} aria-hidden />}
        </Button>
      </Tooltip>
      <div className="deck-replay__speeds" role="group" aria-label="Speed">
        {SPEEDS.map((one) => (
          <button key={one} type="button" className="deck-replay__speed" aria-pressed={replay.speed === one} onClick={() => changeReplay({ speed: one })}>
            {one}x
          </button>
        ))}
      </div>
      <div className="deck-replay__track" role="slider" aria-label="Playhead" aria-valuemin={replay.from} aria-valuemax={end} aria-valuenow={Math.round(replay.at)} onPointerDown={seek}>
        <i className="deck-replay__fill" style={{ width: at(replay.at) }} />
        {ticks.map((one) => (
          <Tooltip key={one.id} label={[one.title, one.says].filter((part) => part !== undefined && part !== "").join(" ")} asChild>
            <i className="deck-replay__tick" data-passed={when(one) <= replay.at || undefined} style={{ left: at(when(one)) }} />
          </Tooltip>
        ))}
        <i className="deck-replay__head" style={{ left: at(replay.at) }} />
      </div>
      <Tooltip label="Back to Live" shortcut="Esc" asChild>
        <Button variant="ghost" size="sm" aria-label="Live" onClick={onLive}>
          <Radio size={16} aria-hidden />
          Live
        </Button>
      </Tooltip>
    </div>
  );
}
