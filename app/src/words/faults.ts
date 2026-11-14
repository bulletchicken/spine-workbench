// What the user reads when an engine call fails: a title, what happened in
// plain words, and what to do next. One entry for every error code; a new
// code in the engine does not compile until it is worded here.

import type { ErrorCode } from '@/shapes';

export interface FaultWords {
  title: string;
  explanation: string;
  whatToDo: string;
}

function words(title: string, explanation: string, whatToDo: string): FaultWords {
  return { title, explanation, whatToDo };
}

export function faultWords(code: ErrorCode): FaultWords {
  switch (code) {
    // Reading a film
    case 'film-unknown-format':
      return words(
        'This file is not a film the app can read',
        'The app reads PNG, JPEG and DICOM images. This file is none of those.',
        'Export the X-ray as a PNG, JPEG or DICOM image and drop it again.',
      );
    case 'film-is-a-directory':
      return words(
        'This is a list of images, not an image',
        'The file is a DICOM directory: it lists other files but holds no picture itself.',
        'Drop the image files from the folder instead.',
      );
    case 'film-no-pixels':
      return words(
        'This DICOM file holds no picture',
        'The file has DICOM information but no image in it.',
        'Choose the image itself, often a larger file in the same folder.',
      );
    case 'film-unsupported-compression':
      return words(
        'This image is compressed in a way the app cannot open',
        'The file uses a compression the app does not decode.',
        'Export the image without compression, or as a PNG, and drop it again.',
      );
    case 'film-unsupported-bits':
      return words(
        'This image stores its grey levels in an unusual way',
        'The number of bits per pixel is one the app cannot read.',
        'Export the image as a 16-bit or 8-bit greyscale PNG and drop it again.',
      );
    case 'film-unsupported-colour':
      return words(
        'This image uses a colour layout the app cannot read',
        'X-rays are greyscale; this file stores colour in a form the app does not convert.',
        'Export the image as a greyscale PNG and drop it again.',
      );
    case 'film-truncated':
      return words(
        'This file ends too soon',
        'Part of the image is missing, as if the file was cut off while it was copied.',
        'Copy or export the file again and drop the new copy.',
      );
    case 'film-corrupt':
      return words(
        'This image is damaged',
        'The image data inside the file could not be decoded.',
        'Copy or export the file again and drop the new copy.',
      );
    case 'film-size-mismatch':
      return words(
        'This file disagrees with itself about its size',
        'The file says the image has one size, but the picture inside has another.',
        'Export the image again, as a PNG if you can, and drop it again.',
      );
    case 'film-too-large':
      return words(
        'This image is too large to open',
        'The file or the picture in it is bigger than the app can hold in memory.',
        'Export a single image at its original size, without extra frames, and drop it again.',
      );

    // Scale and geometry
    case 'scale-too-short':
      return words(
        'The two ruler points are too close together',
        'A ruler this short gives a scale that could be far off.',
        'Click two points farther apart on the ruler, then type the length between them.',
      );
    case 'scale-bad-length':
      return words(
        'That length is out of range',
        'The length typed does not fit what was clicked on the film.',
        'Check the length and its unit (millimetres), then type it again.',
      );
    case 'scale-fit-failed':
      return words(
        'No circle was found at that spot',
        'The app could not find the edge of a round calibration object where you clicked.',
        'Click in the middle of the calibration ball, or use a ruler instead.',
      );
    case 'scale-disagree':
      return words(
        'The scales do not agree',
        'The rulers measured on this film give scales that differ by more than clicking explains.',
        'Check each ruler’s points and length, and remove the one that is off.',
      );
    case 'geometry-no-spine-found':
      return words(
        'No spine was found on this film',
        'The app could not find a spine to judge which way up the film is.',
        'Check this is the right film, then set which way is up by hand.',
      );

    // The auto pass
    case 'side-finder-unavailable':
      return words(
        'The trained side finder is not in this build.',
        'The side film is kept, but it cannot be searched here.',
        'Use a build with the trained side finder to search this film.',
      );
    case 'pass-facing-required':
      return words(
        'Choose which way the patient faces.',
        'Choose on the Side film before searching.',
        'Choose whether the patient faces the left or right of the picture, then search again.',
      );
    case 'pass-no-spine-found':
      return words(
        'No spine was found on this film',
        'The search did not find a spine it could follow from top to bottom.',
        'Check the film is upright and the right way round, then try again, or mark the vertebrae by hand.',
      );
    case 'pass-cancelled':
      return words(
        'The search was stopped',
        'The search was cancelled before it finished. Nothing on the film was changed.',
        'Start the search again when you are ready.',
      );
    case 'pass-film-too-small':
      return words(
        'This film is too small to search',
        'The image has too few pixels for the vertebrae to be told apart.',
        'Use the original full-size image, not a reduced copy.',
      );
    case 'pass-flat-film':
      return words(
        'This film has no contrast',
        'The image is almost one shade of grey, so there is nothing to follow.',
        'Check the right file was chosen and that it opens properly elsewhere.',
      );
    case 'pass-too-little-usable':
      return words(
        'Too little of this film can be used',
        'Shutters, markers and borders cover most of the image.',
        'Use a less cropped copy of the film if there is one, or mark the vertebrae by hand.',
      );
    case 'pass-strip-too-short':
      return words(
        'Too little of the spine is on this film',
        'The spine shown is too short to hold the vertebrae the search looks for.',
        'Use a full-length film, or mark the vertebrae that are shown by hand.',
      );

    // The adjusting tools
    case 'tool-click-outside-film':
      return words(
        'That click was outside the film',
        'The tools work only on the film itself.',
        'Click on the film.',
      );
    case 'tool-refused':
      return words(
        'Nothing to do there',
        'The tool found nothing it can change at that spot.',
        'Move closer to the vertebra or the gap you mean; the preview shows what a click would do.',
      );
    case 'tool-no-contrast':
      return words(
        'Not enough contrast here',
        'The film is too even around that spot for the tool to find an edge.',
        'Click nearer the edge you mean, or adjust brightness and contrast, or move the corners by hand.',
      );
    case 'tool-would-remove-vertebra':
      return words(
        'That would remove the whole vertebra',
        'The change would leave this vertebra with nothing inside its outline.',
        'Click closer to its edge, or merge it with a neighbour using the boundary wand.',
      );
    case 'tool-did-not-converge':
      return words(
        'The tool could not settle on an answer',
        'Its calculation did not settle within its limit. Nothing was changed.',
        'Click again a little closer to the edge you mean.',
      );
    case 'tool-refit-failed':
      return words(
        'No four corners fit that outline',
        'The outline found could not be turned into a vertebra’s four corners. Nothing was changed.',
        'Click again, or place the corners with the pin tool.',
      );
    case 'tool-merge-invalid':
      return words(
        'These two cannot be merged',
        'Merging here would leave the levels out of order or remove a pinned vertebra.',
        'Unpin first if you meant it, or click on the gap between the two you want to join.',
      );
    case 'tool-split-infeasible':
      return words(
        'No split fits here',
        'The vertebrae around this spot cannot be rearranged to fit one more.',
        'Click nearer the middle of the vertebra you want to split, or unpin its neighbours.',
      );
    case 'tool-pin-invalid':
      return words(
        'A pin cannot go there',
        'That spot would put this corner on the wrong side of its neighbours.',
        'Drag the corner to where it sits on the film.',
      );
    case 'tool-constraints-conflict':
      return words(
        'Your pins and edits cannot all hold at once',
        'The pins and gaps you set ask for more than one vertebra can give.',
        'Unpin the corner that is least sure, then try again.',
      );
    case 'tool-unknown-vertebra':
      return words(
        'That vertebra has changed',
        'The edit names a vertebra the film no longer has, usually after an undo or another edit.',
        'Try the edit again.',
      );

    // Case files
    case 'case-not-a-zip':
      return words(
        'This is not a saved case',
        'Saved cases are .spinecase files. This file is not one.',
        'Choose a .spinecase file saved by this app.',
      );
    case 'case-unknown-entry':
      return words(
        'This case file holds something unexpected',
        'It contains a file a saved case never has, so it was not opened.',
        'Open a case saved directly by this app, not one changed by hand or by another tool.',
      );
    case 'case-wrong-format':
      return words(
        'This is not a Spine Workbench case',
        'The file is a zip, but not one this app saved.',
        'Choose a .spinecase file saved by this app.',
      );
    case 'case-newer-version':
      return words(
        'Made by a newer version',
        'This case was saved by a newer version of Spine Workbench than this one.',
        'Open it in the newer version.',
      );
    case 'case-unknown-field':
      return words(
        'This case file has been changed',
        'It contains information a saved case never has, so it was not opened.',
        'Open a case saved directly by this app, not one changed by hand or by another tool.',
      );
    case 'case-missing-film':
      return words(
        'A film is missing from this case',
        'The case names a film whose image is not in the file.',
        'Open an earlier copy of the case if you have one.',
      );
    case 'case-film-size-mismatch':
      return words(
        'A film in this case has the wrong size',
        'A stored image does not match the size the case records for it.',
        'Open an earlier copy of the case if you have one.',
      );
    case 'case-too-large':
      return words(
        'This case file is too large',
        'It is bigger than any case this app saves, so it was not opened.',
        'Check the right file was chosen.',
      );
    case 'case-invalid-value':
      return words(
        'This case file has a value that cannot be right',
        'Something in it is out of range or out of order, so it was not opened or saved.',
        'If this happened while saving, keep the app open and try again; otherwise open an earlier copy.',
      );

    // Made-up spines
    case 'made-up-unsatisfiable':
      return words(
        'That made-up spine could not be built',
        'The recipe and seed asked for a spine that cannot be drawn.',
        'Shuffle the seed or choose another recipe.',
      );

    // The engine itself
    case 'engine-out-of-memory':
      return words(
        'The app ran out of memory',
        'This film needed more memory than the browser would give.',
        'Close other tabs, or use a smaller copy of the film, and try again.',
      );
    case 'engine-crashed':
      return words(
        'The engine stopped and was restarted',
        'Something went wrong inside the engine. Your marks are kept.',
        'Try again. If it happens again at the same step, the film may be one the app cannot handle yet.',
      );
    case 'engine-out-of-sync':
      return words(
        'The app and its engine do not match',
        'Part of the app is from a different build than the rest.',
        'Reload the page.',
      );
    case 'engine-load-failed':
      return words(
        'The engine could not start',
        'The part of the app that does the computing did not load in this browser.',
        'Reload the page. If it still fails, use a recent version of Chrome, Edge, Firefox or Safari.',
      );
  }
}
