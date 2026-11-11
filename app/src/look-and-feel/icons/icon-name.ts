// The names an icon is asked for by. Two sets share them: the icons drawn
// for this app, which nothing else has, and the few Lucide icons the app
// uses, named here for what they mean in this app.

/** The icons drawn for this app, on a 20 px grid. */
export const DRAWN_ICON_NAMES = [
  'pin',
  'region-wand',
  'boundary-wand',
  'magnifier',
  'ruler',
  'on-device',
  'case-file',
  'unreadable-file',
] as const;

/**
 * The Lucide icons the app uses. The last seven are keys: the four arrows,
 * and the signs printed on an Apple keyboard's Command, Shift and Option.
 * The app's typefaces lack most of those characters, so a key hint draws
 * them as icons.
 */
export const LUCIDE_ICON_NAMES = [
  'check',
  'plus',
  'minus',
  'close',
  'undo',
  'redo',
  'play',
  'pause',
  'replay',
  'skip',
  'arrow-left',
  'arrow-right',
  'arrow-up',
  'arrow-down',
  'command',
  'shift',
  'option',
] as const;

export type DrawnIconName = (typeof DRAWN_ICON_NAMES)[number];
export type LucideIconName = (typeof LUCIDE_ICON_NAMES)[number];
export type IconName = DrawnIconName | LucideIconName;
