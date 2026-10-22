import { Prose } from "../../primitives/Prose/Prose";

/**
 * A markdown file's text, drawn as the markdown it is. Read-only: the edit is made in the file beside it, and this follows.
 * Each block carries the line it starts on, which is what the scroll lock reads.
 */
export function MarkdownPreview({ text }: { text: string }) {
  return (
    <div className="armada-markdown-preview">
      <Prose text={text} lines />
    </div>
  );
}
