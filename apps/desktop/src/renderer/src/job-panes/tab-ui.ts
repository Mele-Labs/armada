// The primitives a tab is handed as `host.ui`, named one by one: the vocabulary gate reads an import of `@armada/components` by its names, so a tab's reach is a list here
// and not the whole package. A tab that needs more reads it through `host.armada` or draws its own markup. Add a primitive here when a tab has needed it.

import {
  Alert,
  Badge,
  Button,
  Card,
  Checkbox,
  Input,
  Kbd,
  Popover,
  Prose,
  ScrollArea,
  Select,
  Separator,
  Skeleton,
  Slider,
  Switch,
  Table,
  Tabs,
  Textarea,
  Tooltip,
} from "@armada/components";

export const TAB_UI = {
  Alert,
  Badge,
  Button,
  Card,
  Checkbox,
  Input,
  Kbd,
  Popover,
  Prose,
  ScrollArea,
  Select,
  Separator,
  Skeleton,
  Slider,
  Switch,
  Table,
  Tabs,
  Textarea,
  Tooltip,
};

export type TabUi = typeof TAB_UI;
