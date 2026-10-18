# Touch

**Part of:** [Design System — UI & Voice](../design-system.md).

**A press that lands can be felt, so the eyes can already be elsewhere.** A
person holds Kill and looks at the next row before Fleet has answered. The
trackpad answers the act that finger made, and nothing else.

| Pattern | Plays when |
|---|---|
| **Alignment** | Fleet accepts an act a person pressed |
| **Level change** | Fleet refuses one |

**The tap follows the answer, not the control.** It plays where Fleet's
answer arrives, so an act whose control is already gone — an accepted
Forget leaves no row, and an event can replace a control mid-act — is still
felt.

**Touch only ever answers the person's own press.** Fleet's events never
play one, and neither does hover or focus. A haptic is only felt while a
finger rests on the trackpad, so anything else would be a signal that
mostly fires into nothing.

**Nothing may carry information by touch alone.** The press bar's colour and
direction already say accepted or refused, and a mouse, or macOS's haptic
feedback turned off, loses nothing. Electron cannot reach the trackpad, so
the pattern is performed from outside the renderer, behind one seam that
names the two patterns and not how they are played.
