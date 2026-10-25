import { CircleDashed } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { useKept } from "../../keep";
import { Input } from "../../primitives/Input/Input";
import { Popover } from "../../primitives/Popover/Popover";
import { Radio, RadioGroup } from "../../primitives/Radio/Radio";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * Where the first Sync makes the shared home, chosen before it exists: the account's own login or one of
 * its organisations as owner, and a name that starts as `armada-home`. Kept per login so the Kit page's
 * chip and the Account page's steps read the same choice, until a window is given Fleet's own (`SharedHomeActs.setHome`
 * and `Profile.home`), which win. Once the home exists it is shown, not changed.
 */
export const DEFAULT_HOME_NAME = "armada-home";

export type HomeChoice = { owner: string; name: string };

/** The choice for `login`, kept across leaving the panel that holds it. */
export function useHomeChoice(login: string): [HomeChoice, (next: Partial<HomeChoice>) => void] {
  const [kept, set] = useKept<HomeChoice>(`home-choice:${login}`, { owner: login, name: DEFAULT_HOME_NAME });
  // A login that arrives after the first draw leaves the first reading with no owner.
  const owner = kept.owner === "" ? login : kept.owner;
  return [{ ...kept, owner }, (next) => set({ ...kept, owner, ...next })];
}

/** A repository name as the forge takes it: letters, digits, dot, dash and underscore. */
const clean = (typed: string): string => typed.replace(/\s+/g, "-").replace(/[^A-Za-z0-9._-]/g, "");

/** `owner/name`, the name falling back to the default while the box is empty. */
export const homePath = ({ owner, name }: HomeChoice): string => `${owner}/${name === "" ? DEFAULT_HOME_NAME : name}`;

/** The owner and the name as a pair, an empty name being the default. */
const named = (name: string): string => (name === "" ? DEFAULT_HOME_NAME : name);

/**
 * The chip on the Kit page's Sync bar while no home exists: opens the owner and the name. With `chosen` and `onChoose` the
 * choice is the one given and acts through `onChoose`: an owner on press, the name once the popover is put away or on Enter.
 * Without them it is kept on this window alone.
 */
export function HomePicker({
  login,
  owners,
  chosen,
  onChoose,
}: {
  login: string;
  owners: readonly string[];
  chosen?: HomeChoice;
  onChoose?: (owner: string, name: string) => void;
}) {
  const [kept, set] = useHomeChoice(login);
  const given = chosen !== undefined && onChoose !== undefined;
  const choice = given ? chosen : kept;
  const all = [login, ...owners.filter((one) => one !== login)];
  return (
    <Popover
      label="Where the shared home is made"
      trigger={
        <Tooltip label={`${homePath(choice)}. Where the first Sync makes the shared home. Private, made through gh`}>
          <button type="button" className="armada-shared-chip" data-pick="" data-made="no" aria-label={`Home: ${homePath(choice)}`}>
            <CircleDashed size={12} strokeWidth={2} aria-hidden="true" />
            <span>
              Home: {choice.owner}
              <span className="armada-shared-chip__repo">/{named(choice.name)}</span>
            </span>
          </button>
        </Tooltip>
      }
    >
      {(close) => (
        <HomeFields
          login={login}
          all={all}
          choice={choice}
          close={close}
          {...(given ? { onChoose } : { set })}
        />
      )}
    </Popover>
  );
}

/** The owner and the name. The name is typed as a draft here, and sent once the layer holding it is put away. */
function HomeFields({
  login,
  all,
  choice,
  close,
  set,
  onChoose,
}: {
  login: string;
  all: readonly string[];
  choice: HomeChoice;
  close: () => void;
  set?: (next: Partial<HomeChoice>) => void;
  onChoose?: (owner: string, name: string) => void;
}) {
  const [draft, setDraft] = useState(choice.name);
  const latest = useRef({ draft, choice, onChoose });
  latest.current = { draft, choice, onChoose };
  useEffect(
    () => () => {
      const { draft: typed, choice: now, onChoose: send } = latest.current;
      if (send !== undefined && named(typed) !== named(now.name)) send(now.owner, named(typed));
    },
    [],
  );
  return (
    <div className="armada-home-picker">
      <RadioGroup label="Owner">
        {all.map((one) => (
          <Radio
            key={one}
            name={`home-owner-${login}`}
            checked={choice.owner === one}
            onChange={() => (onChoose === undefined ? set?.({ owner: one }) : onChoose(one, named(draft)))}
          >
            {one}
          </Radio>
        ))}
      </RadioGroup>
      <Input
        label="Name"
        mono
        autoFocus
        onFocus={(event) => event.target.select()}
        value={onChoose === undefined ? choice.name : draft}
        placeholder={DEFAULT_HOME_NAME}
        onChange={(event) => (onChoose === undefined ? set?.({ name: clean(event.target.value) }) : setDraft(clean(event.target.value)))}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            close();
          }
        }}
      />
    </div>
  );
}
