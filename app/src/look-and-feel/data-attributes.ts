// The values of the data attributes through which state reaches CSS. Each
// set is a type here, so the code that works a value out and the parts
// that draw it agree without importing each other.

/** `data-step`, on the app root. */
export type StepAttr = 'films' | 'facing' | 'find' | 'adjust';
/** `data-motion`, on the app root. */
export type MotionMode = 'full' | 'reduced';
/** `data-tool`, on the app root: the tool in hand. */
export type ToolAttr = 'pins' | 'region' | 'boundary';
/** `data-lane`, on the app root: what stands between Front and Side. */
export type LaneMode = 'off' | 'pass' | 'ladder';
/** `data-slot`, on a bay. */
export type SlotFace = 'empty' | 'reading' | 'hung' | 'bad';
/** `data-conf`, on a vertebra, a rung and a legend chip. `sacrum` means not counted: C7 above T1, and any outline below L5. */
export type Conf = 'sure' | 'look' | 'needs' | 'byyou' | 'sacrum';
/** `data-phase`, on a vertebra: how far the pass has brought it. */
export type Phase = 'hidden' | 'ghost' | 'locked' | 'named' | 'coloured';
/** `data-state`, on a fact chip, a check row and a card row. */
export type RowState = 'ok' | 'needs' | 'warn' | 'bad' | 'you' | 'wait' | 'busy';
