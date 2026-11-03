// System › Browser: what a site may use unless its own choice in a Job browser's lock says otherwise. One set, read by every Job browser,
// drawn from the same rows as the lock's All sites view.

import { useState } from "react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@armada/components";

import { LocalNetworkConfirm, PermissionRows } from "./SitePermissionRows";
import { setBrowserDefault, useBrowserDefaults } from "./store";

export function BrowserDefaults() {
  const defaults = useBrowserDefaults();
  const [asking, setAsking] = useState(false);
  return (
    <Card>
      <CardHeader>
        <CardTitle>Browser</CardTitle>
      </CardHeader>
      <CardContent>
        <CardDescription>
          Defaults for every Job browser. A site&apos;s own choice in the lock comes first.
        </CardDescription>
        <div className="job-browser__defaults">
          <PermissionRows
            valueOf={(permission) => defaults[permission]}
            onChoose={setBrowserDefault}
            onAskLocalNetwork={() => setAsking(true)}
          />
        </div>
        <LocalNetworkConfirm
          asking={
            asking
              ? {
                  scope: "all",
                }
              : null
          }
          onCancel={() => setAsking(false)}
          onConfirm={() => {
            setBrowserDefault("local_network", "allow");
            setAsking(false);
          }}
        />
      </CardContent>
    </Card>
  );
}
