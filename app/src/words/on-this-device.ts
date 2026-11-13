// The on-device promise: the badge in the top bar and the popover it
// opens. Plain words for the three guards, and a count that is read from
// the browser, never written here.

export const ON_THIS_DEVICE_WORDS = {
  badge: 'On this device only',
  /** The count beside the badge: "0 sent". */
  sent: (count: number): string => `${String(count)} sent`,

  title: 'Nothing leaves this computer.',
  sub: 'There is no server and no account. Three guards keep it that way.',
  guards: [
    { title: 'The browser blocks it.', line: 'A content policy allows no address outside the app.' },
    { title: 'The code is not allowed to ask.', line: 'A lint rule bans every kind of network call.' },
    { title: 'A test watches the wire.', line: 'The build fails if a single request leaves.' },
  ],
  /** Beside the large number. */
  countLine: (count: number): string => (count === 1 ? 'request sent this session' : 'requests sent this session'),
  /** Shown only when the browser has refused something. */
  blocked: (attempts: number): string =>
    attempts === 1
      ? 'The browser blocked 1 attempt to reach outside the app. Nothing was sent.'
      : `The browser blocked ${String(attempts)} attempts to reach outside the app. Nothing was sent.`,
  fine: 'Films are also stripped on the way in: names, IDs, dates and the file name itself are dropped. Turn the Wi-Fi off and everything still works.',
} as const;
