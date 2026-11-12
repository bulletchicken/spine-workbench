// The standing promise and the note that makes it checkable.
export const ON_THIS_DEVICE_WORDS = {
  badge: 'On this device only',
  lead: 'Your films never leave this computer.',
  boneCredit: 'Bone shapes: BodyParts3D, © The Database Center for Life Science licensed under CC Attribution 4.0 International.',
  blocked: (attempts: number): string =>
    attempts === 1
      ? 'The browser blocked 1 attempt to reach outside the app. Nothing was sent.'
      : `The browser blocked ${String(attempts)} attempts to reach outside the app. Nothing was sent.`,
} as const;
