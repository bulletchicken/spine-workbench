// The words of the 3D view: what a screen reader calls it, and the one
// thing that must be said wherever its bones are explained.

/** The canvas's spoken name. */
export const SPINE_3D_LABEL = 'The spine in 3D';

/** The bones drawn are one adult's, shaped to each fitted vertebra. Say so wherever the shapes are explained. */
export const GENERIC_BONES_NOTE = "Each bone is a generic shape fitted to this spine's measurements, not the patient's own bone.";

/** What a front film can show, and what a one-film fit takes from a typical spine. */
export const ONE_FILM_WORDS = {
  aligned: 'One front film, no perspective',
  opening: 'One front film, the opening view',
  measured: 'Measured on the film: left-right and up-down place, tilt from the corners, width, height and front wedge.',
  typical: 'Forward lean, front-to-back place, side wedge and body depth: typical, not measured.',
  twist: "Twist: estimated from the curve's size using a typical relation, not read from the film.",
  typicalSize: 'Size: typical, not measured. No length in millimetres is shown.',
  measuredSize: "Size: measured from the film's stated scale.",
  lengths: 'Lengths',
  curve: 'Curve used for the typical profile',
  level: 'Level',
  cornerTilt: 'Tilt read from the corners',
  cornerBand: 'Corner bands come from the input corner spread. Model bands come from the fit. Each is one standard deviation.',
  modelTilt: 'Model side tilt',
  lean: 'Forward lean (typical)',
  twistAngle: 'Twist (estimated)',
  depthFlag: 'Depth inferred',
  widthFlag: 'Width inferred',
  twistFlag: 'Twist inferred',
  yes: 'yes',
  no: 'no',
  absent: 'not reported',
};
