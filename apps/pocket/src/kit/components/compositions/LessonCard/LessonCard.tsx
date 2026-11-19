import { type Who, WhoMark, whoWord } from "../WhoMark/WhoMark";
import { type Lands, LandsMark, landsWord } from "../LandsMark/LandsMark";
import { MoveRight } from "lucide-react";

/**
 * Whose way it got in, an arrow, and where the fix lands: each end a word with its mark and its
 * own hue. **One arrow, one component**, so the Lessons list, a retro sheet and the retro a
 * Session is writing in its thread draw the same pair. An end that is absent is not drawn.
 */
export function LessonRoute({ who, landsIn }: { who?: Who; landsIn?: Lands }) {
  return (
    <span className="armada-lesson__route">
      {who === undefined ? null : (
        <span className="armada-lesson__label" data-hue={who === "owner" ? "you" : who}>
          <WhoMark who={who} />
          <span className="armada-lesson__word">{whoWord(who)}</span>
        </span>
      )}
      {landsIn === undefined ? null : (
        <>
          {who === undefined ? null : (
            <MoveRight className="armada-lesson__arrow" size={14} strokeWidth={2} aria-hidden />
          )}
          <span className="armada-lesson__label" data-hue={landsIn}>
            <LandsMark lands={landsIn} />
            <span className="armada-lesson__word">{landsWord(landsIn)}</span>
          </span>
        </>
      )}
    </span>
  );
}
