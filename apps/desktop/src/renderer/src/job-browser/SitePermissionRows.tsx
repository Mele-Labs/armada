// What a site may use, drawn once for the two places that set it: the lock before a Job browser's address, and System › Browser.
// A row is a glyph, a label and Ask, Allow or Block. Allow on Local network is asked about first, in the Dialog drawn here.

import type { KeyboardEvent } from "react";
import {
  Bell,
  ClipboardList,
  Download,
  ExternalLink,
  Image,
  MapPin,
  Monitor,
  Network,
  Radio,
  type LucideIcon,
} from "lucide-react";
import { Dialog } from "@armada/components";

import {
  CHOICES,
  SITE_PERMISSIONS,
  type Choice,
  type SitePermission,
} from "../../../shared/site-permissions";
import "./job-browser.css";

/** The glyph each permission is known by. Camera and Microphone borrow Image and Radio, and Fullscreen borrows Monitor: the registry has none of them. */
const PERMISSION_GLYPH: Record<SitePermission, LucideIcon> = {
  local_network: Network,
  camera: Image,
  microphone: Radio,
  notifications: Bell,
  clipboard: ClipboardList,
  location: MapPin,
  downloads: Download,
  popups: ExternalLink,
  fullscreen: Monitor,
};

/** A row of segments of which one is chosen: arrows move the choice, and Tab stops once. */
export function Segments<T extends string>({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: T;
  options: readonly {
    value: T;
    label: string;
  }[];
  onChange: (value: T) => void;
}) {
  const move = (event: KeyboardEvent<HTMLDivElement>) => {
    const step =
      event.key === "ArrowRight" || event.key === "ArrowDown"
        ? 1
        : event.key === "ArrowLeft" || event.key === "ArrowUp"
          ? -1
          : 0;
    if (step === 0) return;
    event.preventDefault();
    const at = options.findIndex((one) => one.value === value);
    const next = options[(at + step + options.length) % options.length]!;
    onChange(next.value);
    event.currentTarget.querySelector<HTMLElement>(`[data-value="${next.value}"]`)?.focus();
  };
  return (
    <div className="job-browser__segments" role="radiogroup" aria-label={label} onKeyDown={move}>
      {options.map((one) => (
        <button
          key={one.value}
          type="button"
          role="radio"
          className="job-browser__segment"
          data-value={one.value}
          aria-checked={one.value === value}
          tabIndex={one.value === value ? 0 : -1}
          onClick={() => onChange(one.value)}
        >
          {one.label}
        </button>
      ))}
    </div>
  );
}

const CHOICE_OPTIONS = CHOICES.map((one) => ({
  value: one.choice,
  label: one.label,
}));

/** The nine permissions, one row each. A press on Allow for Local network that would change it goes to `onAskLocalNetwork` and not to `onChoose`. */
export function PermissionRows({
  valueOf,
  onChoose,
  onAskLocalNetwork,
}: {
  valueOf: (permission: SitePermission) => Choice;
  onChoose: (permission: SitePermission, choice: Choice) => void;
  onAskLocalNetwork: () => void;
}) {
  return (
    <ul className="job-browser__permissions">
      {SITE_PERMISSIONS.map(({ permission, label }) => {
        const Glyph = PERMISSION_GLYPH[permission];
        const value = valueOf(permission);
        return (
          <li key={permission} className="job-browser__permission">
            <Glyph size={16} strokeWidth={2} aria-hidden />
            <span className="job-browser__permission-label">{label}</span>
            <Segments<Choice>
              label={label}
              value={value}
              options={CHOICE_OPTIONS}
              onChange={(choice) => {
                if (permission === "local_network" && choice === "allow" && value !== "allow")
                  onAskLocalNetwork();
                else onChoose(permission, choice);
              }}
            />
          </li>
        );
      })}
    </ul>
  );
}

/** What a Local network confirmation is about: one site, or every site. */
export type LocalNetworkAsk =
  | {
      scope: "site";
      host: string;
    }
  | {
      scope: "all";
    };

const said = (asking: LocalNetworkAsk) =>
  asking.scope === "site"
    ? {
        title: `Allow local network for ${asking.host}?`,
        body: "This site can reach apps on this Mac and devices on your network. That includes pages the agent opens on it.",
      }
    : {
        title: "Allow local network on all sites?",
        body: "Every site can reach apps on this Mac and devices on your network. That includes pages the agent opens.",
      };

/** The confirmation Allow on Local network asks first: Esc cancels, Enter confirms. Closed where `asking` is null. */
export function LocalNetworkConfirm({
  asking,
  onConfirm,
  onCancel,
}: {
  asking: LocalNetworkAsk | null;
  onConfirm: (asked: LocalNetworkAsk) => void;
  onCancel: () => void;
}) {
  const words = asking === null ? null : said(asking);
  return (
    <Dialog
      open={asking !== null}
      title={words?.title ?? ""}
      tone="neutral"
      initialFocus="confirm"
      confirmLabel="Allow"
      onCancel={onCancel}
      onConfirm={() => {
        if (asking !== null) onConfirm(asking);
      }}
    >
      {words?.body}
    </Dialog>
  );
}
