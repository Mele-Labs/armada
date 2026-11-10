// The Operator's conversation — Bridge's mirror of `OperatorTalk` and what rides on its lines in
// `crates/ipc/src/operator_talk.rs`. `crates/ipc/operations/operator_talk.toml` and `operator_in_front.toml`.

import type { OperatorRead, WaitingItem } from "./sessions";

/**
 * One Needs you item the Operator raised to the person in its conversation: what a card for it needs. Its kind is
 * `item.source`, the thing `item.text`, its acts `item.act` and `item.options`.
 */
export type OperatorRaise = {
  /** The Job whose list holds it. Absent on a suggestion of the Operator's that no Job holds. */
  session_id?: string;
  /** That Job's title. */
  job?: string;
  /** The item as it was raised. */
  item: WaitingItem;
  /** What the Operator made of it and what it would pick, where a call read it. An empty `pick` is none. */
  read?: OperatorRead;
  /** When the item left the list. Absent while it waits. */
  settled_at?: string;
};

/** One Job that ended, or one pull request of a Job that merged, since the last briefing. */
export type BriefedChange = {
  kind: "merged" | "ended";
  session_id: string;
  /** The Job's title. */
  job: string;
  /** On `merged`: the pull request. */
  pull?: {
    number: string;
    title: string;
  };
  /** On `ended`: what its agent said it achieved, where it said. */
  said?: string;
};

/** The Operator's briefing of the day, as data a card draws from. Fleet assembles it; only `top` comes from a call. */
export type OperatorBriefing = {
  /** What moved since the last briefing, or since the local day before began where there was none. Absent when nothing did. */
  since?: BriefedChange[];
  /** The raises still waiting on him, each naming its raise line by the Job and the item. */
  waiting?: {
    /** Absent on a suggestion no Job holds. */
    session_id?: string;
    item_id: string;
  }[];
  /** What the Operator has in hand, a line each. */
  on?: string[];
  /** The call's one line on what matters most today. Absent where no call answered. */
  top?: string;
};

/** One line of the conversation. `text` reads alone: a raise's is markdown, and a briefing's only says it is ready. */
export type OperatorTalkLine = {
  from_person: boolean;
  text: string;
  at: string;
  /** The item the message was about, where it named one. */
  about?: string;
  /** On the Operator's line raising a Needs you item. */
  raise?: OperatorRaise;
  /** On the Operator's briefing, the first time that day Bridge came to the front: what it held. */
  briefing?: OperatorBriefing;
};

/** `operator_talk` (`GET /operator/talk`): the last hundred lines, oldest first. */
export type OperatorTalk = {
  lines: OperatorTalkLine[];
};

/** `operator_in_front` (`POST /operator/front`): whether the day's briefing is on its way. */
export type OperatorInFront = {
  briefing: boolean;
};
