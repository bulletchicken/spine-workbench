// The glossary: one word for each idea the user meets, in the design's own
// wording. Every screen says these words and no others for the same idea.
// It never uses segmentation, inference, pipeline, predict, optimal, safe or
// validated; say outline, guess, search and pass instead.

export const GLOSSARY = {
  'front film': 'The film taken from the front, face on.',
  'side film': 'The film taken from the side.',
  'bending film': 'A film taken from the front while the patient bends to one side.',
  level: 'One vertebra’s place in the spine, named by a letter and a number, such as T12 or L5.',
  outline: 'The four-cornered shape drawn round one vertebra.',
  guess: 'What the app placed before you confirm it. A guess can still move.',
  search: 'The app’s step that places the vertebra outlines down the spine, on both films at once.',
  pass: 'The app’s automatic run over the front and side films that finds the spine, places every outline and colours each one by how sure it is.',
  pin: 'A corner you fix in place, so the app keeps it where you put it.',
  'not visible': 'A part the app could not see on the film.',
  inferred: 'Filled in by the app where no bone was seen on the film. Drawn hollow.',
  Sure: 'The app is confident this outline sits on the right bone.',
  'Worth a look': 'The app is less confident this outline sits on the right bone. Check it.',
  'Needs you': 'The app could not settle this one. Your answer is needed.',
  'Set by you': 'You confirmed this outline or answer.',
} as const;

export type GlossaryTerm = keyof typeof GLOSSARY;
