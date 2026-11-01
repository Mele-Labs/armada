/** A quick response: one press on a line of an agent's message. `send` quotes the line back with the emoji; `ask` puts the quote in the message box instead. */
export type QuickResponse = { id: string; emoji: string; name: string; kind: "send" | "ask" };

/** How many responses the bar holds. */
export const MAX_RESPONSES = 6;

// The owner asked for emoji on this one surface ("Quick Responses", 31 Oct 2026), over the design system's no-emoji rule.
export const DEFAULT_RESPONSES: readonly QuickResponse[] = [
  { id: "pick", emoji: "✅", name: "Pick this", kind: "send" },
  { id: "yes", emoji: "👍", name: "Yes to this", kind: "send" },
  { id: "no", emoji: "👎", name: "No to this", kind: "send" },
  { id: "ask", emoji: "❓", name: "Ask about this", kind: "ask" },
];

/** What he does to the responses, from the bar's menu or from the settings row. */
export type ResponseAct =
  | { act: "edit"; id: string; emoji: string; name: string }
  | { act: "add"; emoji: string; name: string }
  | { act: "remove"; id: string }
  | { act: "move"; id: string; by: -1 | 1 };

/** The list after one act. Added responses send; at most `MAX_RESPONSES`, and never fewer than one. */
export function actOnResponses(list: readonly QuickResponse[], act: ResponseAct): QuickResponse[] {
  switch (act.act) {
    case "add":
      return list.length >= MAX_RESPONSES ? [...list] : [...list, { id: `response:${Date.now()}`, emoji: act.emoji, name: act.name, kind: "send" }];
    case "edit":
      return list.map((one) => (one.id === act.id ? { ...one, emoji: act.emoji, name: act.name } : one));
    case "remove":
      return list.length <= 1 ? [...list] : list.filter((one) => one.id !== act.id);
    case "move": {
      const from = list.findIndex((one) => one.id === act.id);
      const to = from + act.by;
      if (from < 0 || to < 0 || to >= list.length) return [...list];
      const next = [...list];
      const [one] = next.splice(from, 1);
      next.splice(to, 0, one!);
      return next;
    }
  }
}

/** One emoji as a person sees it: pictographs with their joiners, modifiers and variation selectors, or a flag. */
const EMOJI = "(?:\\p{Extended_Pictographic}(?:\\p{Extended_Pictographic}|\\p{Emoji_Modifier}|[\\u200d\\ufe0f\\u20e3])*|\\p{Regional_Indicator}{2})";
const SENT = new RegExp(`^> (.+)\\n\\n(${EMOJI})$`, "u");
const ONE_EMOJI = new RegExp(`^${EMOJI}$`, "u");

/** The message a press sends: the line quoted, a blank line, then the emoji, as a reaction reads. */
export const responseMessage = (line: string, emoji: string): string => `> ${line}\n\n${emoji}`;

/** The line and the emoji a sent message answers with, or null where it is not one. */
export function readResponse(message: string): { line: string; emoji: string } | null {
  const said = SENT.exec(message);
  return said === null ? null : { line: said[1]!, emoji: said[2]! };
}

/** Whether `text` is exactly one emoji. */
export const isEmoji = (text: string): boolean => ONE_EMOJI.test(text);

/** The first emoji or character a field holds, so a paste of several leaves one. */
export function firstOne(text: string): string {
  const first = [...new Intl.Segmenter(undefined, { granularity: "grapheme" }).segment(text.trim())][0];
  return first === undefined ? "" : first.segment;
}

/** About forty common reactions, in rows of eight, each named for its tooltip. */
export const REACTIONS: readonly { emoji: string; name: string }[] = [
  { emoji: "✅", name: "Check" },
  { emoji: "👍", name: "Thumbs up" },
  { emoji: "👎", name: "Thumbs down" },
  { emoji: "❓", name: "Question" },
  { emoji: "❌", name: "Cross" },
  { emoji: "👀", name: "Eyes" },
  { emoji: "🎉", name: "Party" },
  { emoji: "🔥", name: "Fire" },
  { emoji: "💯", name: "Hundred" },
  { emoji: "🙏", name: "Thanks" },
  { emoji: "👏", name: "Clap" },
  { emoji: "🙌", name: "Raised hands" },
  { emoji: "💡", name: "Idea" },
  { emoji: "🤔", name: "Thinking" },
  { emoji: "😄", name: "Smile" },
  { emoji: "😕", name: "Confused" },
  { emoji: "❤️", name: "Heart" },
  { emoji: "⭐", name: "Star" },
  { emoji: "🚀", name: "Rocket" },
  { emoji: "🐛", name: "Bug" },
  { emoji: "⚠️", name: "Warning" },
  { emoji: "🛑", name: "Stop" },
  { emoji: "⏳", name: "Waiting" },
  { emoji: "🔁", name: "Repeat" },
  { emoji: "👌", name: "OK hand" },
  { emoji: "🤝", name: "Handshake" },
  { emoji: "💪", name: "Strong" },
  { emoji: "🧪", name: "Test" },
  { emoji: "📌", name: "Pin" },
  { emoji: "📝", name: "Note" },
  { emoji: "🔍", name: "Search" },
  { emoji: "🧹", name: "Sweep" },
  { emoji: "🆗", name: "OK" },
  { emoji: "➕", name: "Plus" },
  { emoji: "➖", name: "Minus" },
  { emoji: "⏭️", name: "Skip" },
  { emoji: "🏁", name: "Finish" },
  { emoji: "🎯", name: "Target" },
  { emoji: "🤷", name: "Shrug" },
  { emoji: "😅", name: "Relief" },
];
