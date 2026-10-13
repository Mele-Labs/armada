// Sleep mode on the wire, mirrored by hand from `crates/ipc/src/sleep.rs`.

/**
 * A question answered for the owner: `chose` is the option labelled "(Recommended)". A Job told to decide
 * for itself leaves it empty. `session_id` is the session the question came from, absent on a Job's row.
 */
export type SleepDecided = {
  id: string;
  who: string;
  asked: string;
  chose: string;
  corrected?: string;
  session_id?: string;
};

/** Something only the owner can settle. */
export type SleepBlocked = { id: string; who: string; text: string };

/** A pull request that merged during the night. */
export type SleepLanded = { id: string; who: string; title: string; pr: string };

/** A walk held for the owner to look at. */
export type SleepWalk = { id: string; who: string; title: string };

/**
 * `GET /sleep`, what `POST /sleep` and `POST /sleep/override` answer, and `sleep.changed` published
 * whole: the switch and the night's rows.
 */
export type SleepState = {
  on: boolean;
  decided: SleepDecided[];
  blocked: SleepBlocked[];
  landed: SleepLanded[];
  walks: SleepWalk[];
};

/** `POST /sleep`: turn the mode on, which starts a new night, or off. */
export type SetSleep = { on: boolean };

/** `POST /sleep/review`: the owner kept one decision as it was. */
export type ReviewSleep = { id: string };

/** `POST /sleep/override`: the owner's correction of one decision, sent to the session it answered. */
export type OverrideSleep = { id: string; text: string };
