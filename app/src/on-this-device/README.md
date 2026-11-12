# on-this-device

The badge says "On this device only" at every moment. Its note shows
"Your films never leave this computer.", the live request count and the
credit for the bone shapes. The badge has no count on it.

| File | Holds |
|---|---|
| `PromiseBadge.tsx` | the badge and its lock; accepts the bar's shared anchor ref |
| `PromisePopover.tsx` | the note's lead line, count, refused-attempt line and credit |
| `requests-sent.ts` | what the page asked of another origin, and what the policy refused |
| `popover-open.ts` | whether the note is open, also used by a development scene |
| `on-this-device.scenes.ts` | the `on-this-device` scene, with the note open |

The bar mounts the badge and note as siblings. `Popover` keeps the note
in the page, inert and hidden from assistive technology while closed.
Opening moves focus onto it. Escape returns focus to the badge; a
second badge press, a press outside or focus moving on also closes it.
The note is dense glass, below the badge, with its right edge on the
bar's. The system supplies its width, stacked lines, refused-attempt line,
opening and closing transitions.

`useRequestsSent()` gives `{ sent, blocked }` and follows both.

- **blocked** counts attempts the content policy refused outside the
  app. The browser reports each attempt once for each policy, giving its
  text as `originalPolicy`. The counter counts outside reports for each
  text and takes the largest count. With one policy, every report is an
  attempt; with our tag and header, whose texts differ by the header's
  extra `frame-ancestors 'self'` rule, each policy counts every attempt
  once. Report order and timing do not matter. If a host instead repeats
  the tag's exact text in its header, both copies go into one count:
  that is an upper bound and can count one attempt twice.
- **sent** is every resource the page asked for from another origin that
  the policy did not refuse, read from the browser's own record of what
  it asked for, from the record's start. The browser lists a refused
  request there too, so the addresses the policy refused are taken out.
  In the built app the policy refuses every such request before it
  leaves, so this stays at zero; in development, where there is no
  policy, it counts what really went.

A refusal inside a worker is raised there and is not counted by this
page's tally, though the policy still refuses it.
