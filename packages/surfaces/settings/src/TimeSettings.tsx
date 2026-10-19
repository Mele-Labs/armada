// Settings → Time: how every stamp Bridge draws is written. Three switches, and a sample that is the
// current moment in the format they make, so a change is read before it is left.

import { Stamp, Switch, saveTimeFormat, useNow, useTimeFormat } from "@armada/components";

export function TimeSettings() {
  const format = useTimeFormat();
  const now = useNow();
  return (
    <div className="armada-time-settings">
      <Switch checked={format.hour12 === false} onChange={(event) => saveTimeFormat({ ...format, hour12: !event.currentTarget.checked })}>
        24-hour clock
      </Switch>
      <Switch checked={format.seconds} onChange={(event) => saveTimeFormat({ ...format, seconds: event.currentTarget.checked })}>
        Seconds
      </Switch>
      <Switch checked={format.date} onChange={(event) => saveTimeFormat({ ...format, date: event.currentTarget.checked })}>
        Date
      </Switch>
      <div className="armada-time-settings__sample">
        <span className="armada-time-settings__caption">Sample</span>
        <Stamp at={now} />
      </div>
    </div>
  );
}
