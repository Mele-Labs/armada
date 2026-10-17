// Settings → Features: the folders every session may reach past its slots with no card, and the ones
// it may not. An "always" answer on a session's card writes them into settings.json under
// `features.reachFolders`; Remove takes one out, and the next call there draws a card again.

import { Card, CardContent, CardHeader, CardTitle, ReachFolderList } from "@armada/components";
import { REACH_FOLDERS_KEY, reachFoldersOf, reachFoldersWithout, type Outcome, type SaveSettings, type SettingsList } from "@armada/protocol";
import { refusalWords } from "@armada/screens/src/refusal-words";
import { useState } from "react";

export type ReachFoldersSettingsProps = {
  settings: SettingsList | null;
  live: boolean;
  onSave: (changes: SaveSettings["changes"]) => Promise<Outcome>;
};

export function ReachFoldersSettings({ settings, live, onSave }: ReachFoldersSettingsProps) {
  const [saving, setSaving] = useState(false);
  const [refused, setRefused] = useState<string | undefined>(undefined);
  const value = settings?.settings.find((one) => one.key === REACH_FOLDERS_KEY)?.value;
  const { allow, block } = reachFoldersOf(value);

  function remove(folder: string): void {
    const left = reachFoldersWithout(value, folder);
    const empty = left.allow === undefined && left.block === undefined;
    setSaving(true);
    setRefused(undefined);
    void onSave({ [REACH_FOLDERS_KEY]: empty ? null : left }).then((outcome) => {
      setSaving(false);
      if (!outcome.ok) setRefused(refusalWords(outcome));
    });
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Folders jobs reach</CardTitle>
      </CardHeader>
      <CardContent>
        <ReachFolderList allowed={allow} blocked={block} onRemove={remove} disabled={!live || saving} />
        {refused === undefined ? null : (
          <p className="armada-settings__quiet" role="alert">
            {refused}
          </p>
        )}
      </CardContent>
    </Card>
  );
}
