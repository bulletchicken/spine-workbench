# Where the app differs from the mock

The mock is the picture the app is held to. These are the places the app
departs from it on purpose, each with its reason. Anything else that
looks or moves differently is a mistake to fix, or a line to add here in
the pull request that makes it.

## Look and type

| The mock | The app | Why |
|---|---|---|
| Runs on system fonts. | Ships Instrument Serif, Geist and Geist Mono from its own origin. | The mock may fetch nothing; the app bundles its type. Line breaks shift a little. |
| The view figures are closed shapes whose spine runs down through the legs. | The figures stand on open legs, the spine ends at the pelvis, the side view has a nose, and the two bends are the front figure bent as one arc. | Four views that read as one person in four poses, and a side view that cannot be taken for a front. |
| A row that warns and a row that is wrong share the warning triangle, told apart by colour. | A wrong row wears a cross in a ring; the triangle is kept for warnings. | Nothing rests on hue alone. |
| A busy row spins the dotted ring of a waiting row. | Busy is an arc on a faint ring; waiting is a ring of dots. | The two are told apart when nothing moves, as under reduced motion. |
| The waiting sign is drawn in `--ink-4`. | It is drawn in `--ink-3`. | `--ink-4` is 2.9:1 on a card, under the 3:1 a sign needs. |
| The brand mark is tinted pale blue. | It is drawn in ink. | Lumen is kept for the next action; the chrome is neutral. |
| A confidence glyph on a film has no casing. | It has the same dark casing as the outline and the name beside it: a line round a solid glyph, a whole disc under the two with a hollow. | It must read on bright bone, and no bone may show through a ring. |
| Draws the dots of a needs-you outline `1.5 5.5`, though its token `--dash-needs` says `1.5 5`. | `confidence.css` writes `1.5 5.5`, as the mock draws it and the design's table says. The token is kept as the design has it, and nothing reads it. | The tokens are not edited, and at 3.2 px wide the round dots need the half pixel to stay apart. |
| Writes `←`, `→` and `⌘` in key hints as characters, in whatever face the system has. | A key hint draws an arrow key, and Command, Shift and Option, as icons. | The app's typefaces have no left or right arrow and none of those signs, so as characters they would change size and weight from one system to the next. |
| The focus ring is a shadow. | The ring is the token's shadow and an outline in the same place. | A control with a shadow of its own keeps its ring, and forced colours still draw one. |
| The boundary wand's gap line touches the two bodies; the wand's sparks are uneven. | The drawn icons are redrawn on one 20 px grid with one 1.6 px line. | One family at one weight. |
| Draws its everyday icons (tick, close, undo, play and the rest) by hand in its sprite. | Takes them from Lucide, as the design language asks. The three solid playback signs are drawn at four fifths. | The mock may load nothing; the app installs the icon package. A solid shape weighs more than a line drawing of the same size. |

## Motion

| The mock | The app | Why |
|---|---|---|
| Three keyframes breathe: the lit button's, the choice pair's and the ring's. | One, `breathe`, moves a number from 0 to 1 and each lit part turns it into its own light. | One loop, stopped in one place under reduced motion. |
| Under reduced motion the ring rests at full strength and a settling line still rises. | Every lit part rests at half a breath, and a settling line only brightens. | Light still arrives; nothing loops and nothing travels. |
| A cascade's delay is set on each element by script. | Script writes each item's place, `--i`, and CSS works out the delay from a token. | Timing is never set from script, so reduced motion closes every gap. |
| The film is moved by a CSS transform with a transition. | The camera is moved by script on the one loop, with the same token duration and curve. | A real film is about 3000 by 7000 pixels; the app draws it on a canvas the size of its viewport. |

## Set by the contracts

| The mock | The app | Why |
|---|---|---|
| Saves `spine-case-2026-11-10.zip`. | Saves `spine-case-YYYY-MM-DD.spinecase`. | The case file contract names the file. |
| Draws C7 as a landmark above 17 outlines. | Holds a C7 body above T1, drawn as not counted and left out of every count. | The engine's chain starts at C7, and the landmark is worked out from that body. |
| "Can't read this file. It isn't a PNG, a JPEG or a DICOM image." | The title and explanation of the fault in `app/src/words/faults.ts`, then "Nothing else changed." | The app tells a wrong file from an unsupported compression, and says so. |
| The status line reads "Not a medical device. A research preview. It shows previews with error bands, never predictions." | The status line reads the approved notice in `app/src/words/preview-notice.ts`. | That notice changes only with the owner's approval. |

## The sample case

In development the app opens in the mock's story: the same made-up
spine, painted again from the mock's numbers, in `app/src/sample-case/`.
Gallery pages, scenes and tests read it, so a screen can be put beside a
screenshot of the mock in the same state. Where the repository's frozen
rules and the mock disagree the rules win, and these are the differences
that shows. The sample case's own readme has each in full.

### The films

| The mock | The sample case | Why |
|---|---|---|
| Paints at 0.6 mm a pixel and prints 0.144 mm/px; bending right measures 0.151. | Every film is 0.45 mm a pixel: the size chips read 0.45, and bending right measures 0.45 across its ball. | At 0.45 the bodies come out at an adult's size, and the 25 mm ball spans 55.6 pixels, enough for a two-click ruler. The ball is a little larger than the mock's. |
| Simulates its files; none has a kind. | Front, side and bending left are DICOM files; bending right is a PNG. | A size "read from the file" needs tags only a DICOM file has, so the PNG is the one film with no size. |
| Sets the lead letter in a typeface. | Draws the letter as a shape. | The film is painted onto plain arrays, with no typeface to call on. Its edges differ by a pixel or two. |
| Draws one circle for both hips on the side film. | Paints two heads, 8.5 mm apart, and marks them as two unpaired hip centres. | The mark names tell two heads seen apart from one merged midpoint, and the story shows the harder of the two. |

### The marks

| The mock | The sample case | Why |
|---|---|---|
| 17 outlines on Front and Side. | 18 bodies: the 17 and the C7 body above them, drawn as not counted. | The chain search solves C7 as its first body. |
| Marks the same 17 bodies on the bending films, the tall one included. | The bending films' chains are right, C7 to L5, with no body taken as two. | No edit can change a bending film's count, so a merged outline there could never be fixed. |
| A vertebra has one confidence on every film. | Confidence is per film: T3 needs you on Front only, and is clear on Side; T1 is worth a look on both. | The pass judges each film by what that film shows. |
| Draws a pedicle ring only where its painted bone has one, found or not visible. | A pedicle the pass cannot find is placed where it usually sits and marked inferred: on the C7 body, on the outline on the sacrum, and on the lower piece of a split. | Nothing the app filled in is drawn as found. |

### The names, stage by stage

| The mock | The sample case | Why |
|---|---|---|
| Once L5 is confirmed the outline below it is "S1", grey and dashed. | That outline is named L6 and flagged as a count that is not the usual one. It is still drawn as not counted. | The frozen naming rules have no S1: levels run C7 to L6. |
| Confirming L5 shifts the 17 names; nothing stands above T1. | Every name moves one down, the C7 body's too: until the split it is named T1 and the real T1 is T2, as in the mock, and no body is C7, so the C7 landmark is empty. | The `confirm-l5` rule is followed to the letter. |
| Counts names up from L5, so a split shifts the names above it. | The same, by the frozen rules: the names on the side away from the confirmed L5 shift, L5 keeps its name and its confirmation, and C7 comes back. | Not a difference from the mock, but one inside the repository: the frozen rules shift one side of an edit and leave open which, and the engine's boundary wand shifts the names below. The sample case and the wand are to be brought to one rule before a screen relies on either. |

### The pass and the answer key

| The mock | The sample case | Why |
|---|---|---|
| Plays the whole pass, with evidence maps, strips and barcodes computed from its drawn films. | The pass is the frames the contract never drops, made from the marks, until its frames are recorded from the films. | An honest short pass now, the full one once it is recorded. |
| Has no answer key. | Carries the truth its films were painted from as a made-up spine's answer key. Its recipe does not remake the case, and its twist follows the painted pedicles. | The films are painted from fixed numbers, not drawn from a recipe; the key names a recipe only because a key must. |
