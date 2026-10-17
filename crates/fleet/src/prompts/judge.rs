//! `verification`'s pieces: the key that overrides each and the piece it
//! is. Generated from one table with the config entries.

use config::settings as keys;
use config::settings::Words;

pub(super) fn judge() -> Vec<(Words, verification::Piece)> {
    vec![
        (
            keys::PROMPT_JUDGE_OPENING,
            verification::pieces::JUDGE_OPENING,
        ),
        (
            keys::PROMPT_JUDGE_QUESTION,
            verification::pieces::JUDGE_QUESTION,
        ),
        (
            keys::PROMPT_JUDGE_ANSWER,
            verification::pieces::JUDGE_ANSWER,
        ),
        (keys::PROMPT_JUDGE_RISK, verification::pieces::JUDGE_RISK),
        (
            keys::PROMPT_JUDGE_READABLE,
            verification::pieces::JUDGE_READABLE,
        ),
        (
            keys::PROMPT_JUDGE_STANDING,
            verification::pieces::JUDGE_STANDING,
        ),
        (
            keys::PROMPT_JUDGE_STANDING_UNREADABLE,
            verification::pieces::JUDGE_STANDING_UNREADABLE,
        ),
        (
            keys::PROMPT_JUDGE_STANDING_CUT,
            verification::pieces::JUDGE_STANDING_CUT,
        ),
    ]
}
