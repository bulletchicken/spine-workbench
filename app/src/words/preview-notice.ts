// The approved notice about what the app's numbers are and are not. It is
// shown on every screen; change it only with the owner's approval.

export const PREVIEW_NOTICE = {
  /** The one line shown in the app shell. */
  short: 'A preview with error bands, not a prediction. Not a medical device.',
  /** The full notice, shown where there is room to read it. */
  full:
    'Spine Workbench shows previews with error bands. It does not predict any patient’s outcome. It is not a medical device and is not validated for clinical use.',
} as const;
