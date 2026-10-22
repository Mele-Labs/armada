// The dialog Report opens over the whole window: the owner types what went wrong in the Job's own message box, with `@` files, `#` issues
// and pull requests and files pasted, dropped or attached, and Send or Enter hands it to a new Session. Esc or Cancel puts it away and keeps
// what was typed; a send clears it. What starts the Session and what it is told is in `SessionTiles.tsx` and `report-brief.ts`. The owner's
// ask of 15 Oct 2026; drawn over the app rather than in the flight controls' place since his walk note of 21 Oct 2026, where the panel clipped
// it, and given the message box since his note of 22 Oct 2026.

import { Dialog, SessionComposer, type SentFromComposer } from "@armada/components";

import { searchFiles } from "../../commands";
import { useNumbered } from "../../numbered";
import { OverTheApp } from "../../session-view/OverTheApp";

const NONE: readonly never[] = [];
const NOBODY = () => undefined;

/** `keep` names the draft, so leaving and coming back finds it. `manifestId` is the repository `@` and `#` read. `send` resolves to whether the Session was started and told. */
export function ReportBox({ keep, manifestId, send, onClose }: { keep: string; manifestId?: string | undefined; send: (sent: SentFromComposer) => Promise<boolean>; onClose: () => void }) {
  const numbered = useNumbered(manifestId);
  return (
    <OverTheApp open>
      <Dialog open tone="neutral" width="wide" title="Report an issue with the Job" onCancel={onClose}>
        <SessionComposer
          attachOnly
          working={false}
          label="Report"
          model={null}
          effort={null}
          models={NONE}
          efforts={NONE}
          onTune={NOBODY}
          commands={NONE}
          taggable={NONE}
          searchFiles={(query) => searchFiles(query, manifestId)}
          {...(numbered === undefined ? {} : { numbered })}
          tags={NONE}
          onTags={NOBODY}
          drawn={NONE}
          onDraw={NOBODY}
          onRemoveDrawn={NOBODY}
          draftKey={keep}
          focusFor={keep}
          grows="panel"
          onSend={async (sent) => {
            const done = await send(sent);
            if (done) onClose();
            return done;
          }}
        />
      </Dialog>
    </OverTheApp>
  );
}
