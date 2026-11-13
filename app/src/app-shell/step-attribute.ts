// The value of `data-step` on the app root for a step. A slug is used as
// it is, unless the design gives that step a shorter name, so the design's
// attribute values and the folders' names can both stay as they are.

const DESIGN_NAMES: Readonly<Record<string, string>> = {
  'size-and-facing': 'facing',
};

/** `data-step` for the step with this slug: `facing` for `size-and-facing`, any other slug unchanged. */
export function stepAttribute(slug: string): string {
  return DESIGN_NAMES[slug] ?? slug;
}
