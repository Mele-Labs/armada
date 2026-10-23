// Settings › Links, under the setting that is already there: the link rules the owner taught, one row each, with Forget.
// Nothing is drawn while nothing is taught.

import { Card, CardContent, TaughtLinkRow } from "@armada/components";

import { forgetLesson, useLessons } from "./teach";

export function TaughtLinks() {
  const taught = useLessons().filter((one) => one.link !== undefined);
  if (taught.length === 0) return null;
  return (
    <Card>
      <CardContent>
        {taught.map((one) => (
          <TaughtLinkRow
            key={one.id}
            example={one.link!.example}
            address={one.link!.address.replaceAll("{match}", one.link!.example)}
            reach={one.reach === "all" ? "Every repository" : one.repository}
            onForget={() => forgetLesson(one.id)}
          />
        ))}
      </CardContent>
    </Card>
  );
}
