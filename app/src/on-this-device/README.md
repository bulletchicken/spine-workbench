# on-this-device

Nothing leaves this computer, and a person can check: the badge in the
top bar, the popover it opens, and the counter both of them and the
status line read.

| File | Holds |
|---|---|
| `PromiseBadge.tsx` | "On this device only · 0 sent"; a press opens the popover |
| `PromisePopover.tsx` | the three guards in plain words, the count, and a line when the browser refused something |
| `requests-sent.ts` | the counter: what the page asked of another origin, and what the content policy refused |
| `popover-open.ts` | whether the popover is open, so a scene can open it |
| `on-this-device.scenes.ts` | the scene `on-this-device`: the app with the popover open |

## The counter

`useRequestsSent()` gives `{ sent, blocked }` and follows both.

- **blocked** is every time the content policy refused an address
  outside the app, from the event the browser raises when it does. A page
  that holds its policy twice, as a header and as a tag, is told of one
  refusal twice, a moment apart: reports of the same address under the
  same rule within 50 ms are one attempt.
- **sent** is every resource the page asked for from another origin that
  the policy did not refuse, read from the browser's own record of what
  it asked for, from the record's start. The browser lists a refused
  request there too, so the addresses the policy refused are taken out.
  In the built app the policy refuses every such request before it
  leaves, so this stays at zero; in development, where there is no
  policy, it counts what really went.

A request is told by where its address leads, never by what kind of call
made it, so nothing here names a network API. The counter watches the
page: a refusal inside a worker is raised in the worker and is not
counted here, though the policy refuses it all the same.
