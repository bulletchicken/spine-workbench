import type { CurveRegion, Level, NotMeasured } from '@/shapes';
export const READINGS_WORDS = {
  title: 'Measurements', loading: 'Reading the spine…', failed: 'The measurements could not be read.',
  mainRange: "The main curve's range holds the angle that exact marks would give about three times in four, for the spread the marks were given. A range does not cover which vertebrae are the ends.",
  smallerRanges: "The smaller curves' ranges were not checked.",
  sideRanges: 'For kyphosis the range holds it about three times in four, for lordosis about two times in three.',
  twistRange: "The twist's range has not been checked this way.",
  noisyCurves: 'On noisy marks a curve can read a few degrees high, and noise can split one curve into shorter ones.',
  rangeFigures: 'These figures were measured on made-up spines.',
  kyphosis: 'Thoracic kyphosis', lordosis: 'Lumbar lordosis', twist: 'Rotation at the apex',
  kyphosisAbout: 'T5 to T12, seen from the side',
  lordosisAbout: (sacrum: boolean) => sacrum ? 'L1 to the sacrum, seen from the side' : 'L1 to L5, seen from the side',
};
export const REGION_NAMES: Record<CurveRegion, string> = {
  'proximal-thoracic': 'Upper thoracic curve', 'main-thoracic': 'Main thoracic curve',
  'thoracolumbar-lumbar': 'Thoracolumbar and lumbar curve',
};
export const REASON_WORDS: Record<NotMeasured, string | null> = {
  'no-side-film': 'Side curves need a side film.',
  'not-on-side-film': 'The side film does not show these vertebrae.',
  'twist-not-measured': 'Twist is not measured from these films.',
  'no-sacrum': 'The lower curve is read to L5; the sacrum is not marked.',
  'no-curve': 'No side-to-side curve was found.', 'no-bending-films': null, 'not-fitted': null,
};
export function pictureDirection(left: boolean): string { return left ? 'toward the left of the picture' : 'toward the right of the picture'; }
export function curveAbout(upper: Level, lower: Level, direction: string): string { return `Cobb angle ${upper} to ${lower}, bulging ${direction}`; }
export function twistAbout(apex: Level, direction: string): string { return `Apex at ${apex}${direction === '' ? '' : `, turned ${direction}`}`; }
