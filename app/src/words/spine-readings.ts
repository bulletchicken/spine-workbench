import type { CurveRegion, Level, NotMeasured } from '@/shapes';
export const READINGS_WORDS = {
  title: 'Measurements', loading: 'Reading the spine…', failed: 'The measurements could not be read.',
  meaning: 'Each band covers eight in ten of the spines that fit these marks.',
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
