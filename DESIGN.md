# Spine Workbench: design system

## 1. What this is

- This is the one document that says how Spine Workbench looks, moves and speaks, and why.
- Two files go with it: [`app/src/look-and-feel/tokens.css`](app/src/look-and-feel/tokens.css) (shared values, by name) and [`app/src/look-and-feel/components.css`](app/src/look-and-feel/components.css) (shared parts, as plain classes). From here on they are called `tokens.css` and `components.css`.
- To design a new screen: read the principles, build it from the parts in section 6, and use only the names in `tokens.css`.
- At the time of writing, 14 November 2026, Films, Find and 3D, saving a case and opening it again are built in the app. The values here describe those moments. Further changes follow section 11.

| To find | Look in |
|---|---|
| A colour, a type size, a spacing step, a corner, a shadow, a glass fill, a duration | 4 Foundations |
| Where a part sits and how big it is, at each window size | 5 Layout |
| What a part is for, its states and its class names | 6 Components |
| What happens, in order, when a moment changes | 7 Motion, scene by scene |
| A word on screen, spelled exactly | 8 Words |
| A contrast ratio, the keyboard, what is not yet solved | 9 Accessibility |
| What not to do | 3 Principles, 10 What would make it look cheap |
| How to change something | 11 Keeping this current |
| Why something is the way it is | 12 Decisions log |

| Status word | Meaning |
|---|---|
| **Settled** | Built and agreed. |
| **Decided, being built** | Agreed. Not yet in the app. Written here from the decision; sizes and timings are filled in when it is built. |
| **Open** | Not yet decided. Do not build on it until it is. |

Anything with no status word is Settled. All sizes are in CSS pixels.

## 2. The product

Spine Workbench is a browser app that runs entirely on the user's own computer. A clinician or researcher can give it up to four spine X-rays of one child: standing from the front, standing from the side, bending left and bending right. It finds vertebrae on the films, then builds a 3D spine from the fitted T1 to L5 bones that can be turned and measured. Missing bones are not drawn. Every number is shown with its error band. It should feel like a quiet white room with one object in it: clean, calm, thoughtful and effortless. The identity is called Clear.

| Moment | What happens | The one sentence | The one button |
|---|---|---|---|
| **1 Films** | The four films are added. Each lands in its own place. | "Add the four films." then "All four films are in." | Find the spine |
| **2 Find** | The app finds the spine while the user watches. Outlines appear on the films, vertebra by vertebra, each with its name. | "Finding the spine…" then "17 vertebrae found. 2 need a look." | Build the 3D spine |
| **3 3D** | The films move to a rail at the side. The spine can be turned, seen from three sides, and read as a short list of measurements. | "The spine in 3D." | None. The view control takes its place. |

Settled: Films, Find and 3D follow one another, with fewer than four films allowed.

- The Front film alone is enough to find and build the spine. The other films can be left out.
- A Side film can be searched alone after its facing is chosen and its finder is available. Building a 3D spine still needs Front.
- Find shows "Build the 3D spine" disabled while searching, and ready when Front has been found. A failed build keeps the films and offers "Try again".

## 3. Principles

1. **One sentence and one button per moment**: the dock says what is happening and holds the one next step.
   Rules out: a second primary button, toolbars, tooltips, helper paragraphs, panels of tools.
2. **The films and the spine are the only dark, rich things on screen**: everything around them is light, soft and out of the way.
   Rules out: dark bars, coloured panels, pictures or illustration in the interface, coloured gradients.
3. **Glass is for controls that float, never for content**: the bar, the dock, popovers and chips are glass; films, outlines and the spine are not.
   Rules out: glass on a film, glass on glass, a glass card in the middle of the stage. One exception is recorded under Depth.
4. **Things travel, they do not swap**: the same four films carry through every moment, and a scene change only moves and scales them.
   Rules out: cross-fading whole screens, anything that vanishes in one place and reappears in another.
5. **Every number carries its band**: a measurement is always written `40° ± 4°`.
   Rules out: a bare number, a band hidden behind a pointer, colour that grades a value as good or bad.
6. **Colour is a signal**: blue means focus, the thing pointed at, the place a drop will land, or progress; amber means "needs a look"; everything else is black, white or grey.
   Rules out: a third colour, a coloured button, colour as the only sign of a state.
7. **The same things stay in the same place**: the bar, the dock and the notice never move, share one width and start their words at the same inset.
   Rules out: controls that jump between moments, a dock wider than the bar, hiding the notice or the trust badge.
8. **The film stays as it is**: a film is never dimmed, tinted or otherwise altered to make a mark on it readable. A mark carries its own casing instead: the same shape drawn again beneath it, a little wider and dark.
   Rules out: a dark layer over a film, a change to a film's brightness or contrast, a pill, a box or a drop shadow behind a mark.

## 4. Foundations

### Type

| Token | Value | Use |
|---|---|---|
| `--font-sans` | Geist, then the system's own sans-serif face | Everything |
| `--font-mono` | Geist Mono, then the system's own mono face | Numbers and vertebra names only, with tabular figures (`.num`) |
| `--weight-regular` / `--weight-medium` / `--weight-semibold` | 400 / 500 / 600 | Nothing lighter, nothing heavier |

Both faces are local files, served with the app (variable weight, latin).

| Role | Size / line | Weight | Tracking | Token | Where |
|---|---|---|---|---|---|
| Display | 44 / 48 | 600 | -0.025em | `--type-display`, `--track-display` | The start sentence. Nowhere else in the product |
| Heading | 18 / 24 | 600 | -0.01em | `--type-heading`, `--track-heading` | The dock's sentence in a window wider than 900, the panel's title |
| Body | 15 / 22 | 400 | 0 | `--type-body` | Default text |
| Body, medium | 15 / 22 | 500 | 0 | `--type-body-medium` | Primary button, product name, first line of the trust note; the dock's sentence in a window 900 wide or less |
| Small | 13 / 18 | 400 | 0 | `--type-small` | The dock's helper line, the trust note's count |
| Small, medium | 13 / 18 | 500 | 0 | `--type-small-medium` | Quiet buttons, step and view options, trust badge |
| Row | 13 / 20 | 500 | 0 | `--type-row` | A measurement's name |
| Caption | 12 / 16 | 500 | 0.01em | `--type-caption`, `--track-caption` | Film names, the notice, a measurement's note, small print |
| Number | 13 / 18 | 500 mono | 0 | `.num` on small text | Counts in the dock, a measurement's band |
| Value | 16 / 20 | 500 mono | 0 | `--size-value`, `--line-value`, with `.num` | A measurement's value |
| Chip | 12 / 24 | 500 mono | 0 | `--size-caption`, `--line-chip`, with `.num` | The name chip |
| Mark | 11 | 500 mono | 0 | `--size-mark` | A vertebra's name on film. The smallest text |

Sentence case everywhere. No all-capitals labels. No size outside this table. Classes: `.t-display`, `.t-heading`, `.t-body`, `.t-small`, `.t-caption`, `.num`. A number takes the size of the text it sits in; `.num` gives it the mono face, weight 500 and tabular figures.

The dock's sentence reads its own pair of tokens, `--type-say` and `--track-say`. They are the Heading role. In a window 900 wide or less `tokens.css` sets them to Body, medium, with tracking 0. Settled.

### Colour

Contrast is the colour as text on white, or on black for film colours. For a surface it is the named text colour on that surface. Pass means 4.5 to 1 or more, the level needed for body text (WCAG AA).

| Token | Value | Use | Contrast | Body text |
|---|---|---|---|---|
| `--ink` | `#0d0d0d` | Main text, the primary button | 19.4 on white | Pass |
| `--ink-2` | `#5d5d5d` | Second text: helpers, names, quiet controls | 6.6 on white, 6.1 on `--page-soft`, 5.7 on `--glow` | Pass |
| `--ink-3` | `#8f8f8f` | Words on a disabled primary button | 3.2 on white, 3.0 on `--fill` | Fail. Disabled only |
| `--ink-faint` | `rgba(13,13,13,0.3)` | A step not yet reached; the drawing in an empty place | 2.0 on its ground | Fail. Disabled and drawing only |
| `--ink-faint-hover` | `rgba(13,13,13,0.44)` | That drawing, pointed at, and while its film is being read | 2.9 on `--soft-2` | Fail. Drawing only |
| `--ink-hover` | `#2e2e2e` | Primary button, pointed at | White on it: 13.6 | Pass |
| `--ink-pressed` | `#000` | Primary button, pressed | White on it: 21.0 | Pass |
| `--on-ink` | `#fff` | Words on the primary button | 19.4 on `--ink` | Pass |
| `--line` | `rgba(13,13,13,0.08)` | Hairlines; pressed fill of quiet controls | `--ink` on it: 16.4 | Pass |
| `--line-strong` | `rgba(13,13,13,0.4)` | Edge of glass under the More contrast setting | 2.7 against white | Not text |
| `--fill` | `rgba(13,13,13,0.04)` | Hover fill of quiet controls; a disabled primary button | `--ink` on it: 17.9 | Pass |
| `--track` | `rgba(13,13,13,0.05)` | Track of the step and view controls | `--ink-2` on it: 5.9 | Pass |
| `--thumb` | `#fff` | The thumb in that track | `--ink` on it: 19.4 | Pass |
| `--underline` | `rgba(13,13,13,0.28)` | Under a text link at rest | 1.9 against white | Not text |
| `--page` | `#fff` | The page | `--ink-2` on it: 6.6 | Pass |
| `--page-soft` | `#f6f7f9` | The foot of the page | `--ink-2` on it: 6.1 | Pass |
| `--page-wash-1`, `--page-wash-2` | `#fcfcfd`, `#f9fafb` | Two steps between them. `--page-wash` is the whole wash | `--ink-2` on them: 6.4, 6.3 | Pass |
| `--glow` | `#e9f0f9` | The glow behind the spine | `--ink-2` on it: 5.7 | Pass |
| `--glow-quiet` | `rgba(233,240,249,0.8)` | The same glow behind the films | `--ink-2` on it: 5.9 | Pass |
| `--soft-1` / `--soft-2` / `--soft-3` | `#f3f4f7` / `#edeff3` / `#e8eaef` | An empty film place: rest; pointed at, or while its film is being read; pressed | The drawing on them: 2.0 to 2.9 | Drawing only |
| `--accent` | `#0a7cff` | Focus ring, the vertebra pointed at, the progress bar, a place files are held over, the ring of the one place or film they are over | 3.9 on white, 5.3 on black | Fail on white: rings, marks and drawings only, never words |
| `--accent-soft` | `rgba(10,124,255,0.12)` | A wash for a selected thing. Open: named, used by nothing yet | `--ink` on it: 16.7 | Pass |
| `--accent-fill` | `rgba(10,124,255,0.2)` | Inside the outline pointed at | | Not text |
| `--accent-edge` | `rgba(10,124,255,0.4)` | Ring of a place while files are held over the window | | Not text |
| `--accent-wash` | `#e7f1fe` | Fill of that place | `--accent` on it: 3.4 | Drawing only |
| `--look` | `#f5a524` | "Needs a look" | 2.0 on white, 10.3 on black | Never words: a dot beside words, or a line on film over its casing |
| `--look-fill` / `--look-pulse` | `rgba(245,165,36,0.14)` / `0.5` | Inside an outline that needs a look; the top of its pulse | | Not text |
| `--film` | `#000` | Behind every film | | |
| `--film-edge` / `--film-edge-hover` | `rgba(255,255,255,0.1)` / `0.6` | The lit edge inside a tile; the same on a rail film pointed at | 7.4 on black when pointed at | Not text |
| `--mark` | `rgba(255,255,255,0.9)` | Outline of a vertebra | 16.7 on black | A line, not text. On film it lies over its casing |
| `--mark-casing` | `rgba(0,0,0,0.45)` | The casing under a white outline. Settled | The white line against it: 3.0 over white bone, the brightest a film can be | Not text |
| `--mark-casing-look` | `rgba(0,0,0,0.65)` | The casing under an amber outline. Settled | The amber line against it: 3.4 over white bone | Not text |
| `--mark-casing-on` | `rgba(0,0,0,0.8)` | The casing under a blue outline. Settled | The blue line against it: 3.2 over white bone | Not text |
| `--mark-fill` | `rgba(255,255,255,0.06)` | Inside it | | Not text |
| `--mark-name` | `rgba(255,255,255,0.92)` | A vertebra's name on film. Before the casing it was white at 80% | 17.6 on black. Against its casing: 6.7 or more on the sample films, 4.3 over white film | Pass where the film behind is no lighter than `#f8f8f8`. Section 9 has the numbers |
| `--mark-name-casing` | `rgba(0,0,0,0.55)` | The casing painted under a name's letters. Settled | | Not text |
| `--bone` | `#ece7df` | Bone in 3D | | |
| `--spine-shadow` | `rgba(20,24,32,0.55)` | Centre of the shadow under the spine | | |

`--find-light` and `--find-light-mask` are the gradient and the side fade of the light that travels during the find. The light is drawn as an SVG shape, and an SVG paint cannot read a CSS gradient. So the same stops are written out again where the shape is drawn. Keep them equal to the two tokens.

### Space

| Token | Value | Typical use |
|---|---|---|
| `--space-4` | 4 | A name's drop on film; the product mark's pull to the left |
| `--space-8` | 8 | Amber dot to words; a name's distance from its outline; above the trust note's line for refused attempts |
| `--space-12` | 12 | Between a measurement's name and value; above and below the hairline in the trust note |
| `--space-16` | 16 | Top and bottom of the trust note |
| `--space-20` | 20 | Above the hairline in the panel |
| `--space-24` | 24 | Between related objects: tiles. Kept clear round a spine when its film is pushed in |
| `--space-28` | 28 | The room a film's name takes under its tile |
| `--space-40` | 40 | An available spacing step. The built frame keeps 48 or 16 at its edges |
| `--space-48` | 48 | Between groups: under the start sentence |
| `--space-60` | 60 | Kept clear above the start sentence and below the film names |
| `--space-2`, `--space-6`, `--space-10`, `--space-14` | 2, 6, 10, 14 | Half steps, for the inside of a part: padding in a control, an icon beside its words, a count beside its bar, the gaps in the dock and the panel |
| `--primary-pad` | 22 | Named: each side of the primary button's words |
| `--panel-list-top` | 18 | Named: under the panel's title |
| `--brand-gap` | 7 | Named: from the product mark to the product's name |

- A distance that places a part on the page is a step of 4. Two are half steps instead: the notice sits 14 from the bottom, and the name chip 14 to the right of the pointer.
- Inside a part, half steps are allowed.
- Three distances are on neither scale and have their own names: 22, 18 and 7.
- The size of a small part is not spacing and need not be on the scale: the amber dot is 7 across, the progress bar 3 tall, the name chip 26 tall.
- A gap between groups is at least twice a gap inside a group. Group with empty space before reaching for a line.
- Distances that place a part on the page have their own names in section 5.

### Shape

| Token | Value | Use |
|---|---|---|
| `--r-hair` | 2 | The progress bar |
| `--r-text` | 4 | The focus ring's corner on a text link |
| `--r-small` | 10 | A film in the rail |
| `--r-medium` | 14 | The two bending films while tucked beside the others |
| `--r-large` | 20 | A film tile, an empty place |
| `--r-xlarge` | 28 | The measurements panel, the trust note, sheets |
| `--r-pill` | 999 | Everything a person presses; the bar; the dock |

A film's corner shrinks with the film: 20, 14, 10. No other corner sizes.

Lines: a hairline is 1 (`--hairline`). An outline on film, and the ring of a place under held files, are 1.5 (`--mark-stroke`). The focus ring is 2 (`--ring-width`), and so is the ring of the one place or film that held files are over. The casing under an outline on film is 3.5 (`--mark-casing-stroke`), and the casing under a name's letters is 3 (`--mark-name-casing-stroke`).

### Depth

| Layer | What is on it | Surface | Shadow |
|---|---|---|---|
| **1 Page** | The page, the glow, empty places, words set on the page | `--page-wash`: white, easing to `--page-soft` over the lower half | None |
| **2 Objects** | Film tiles, the spine | `--film`; `--bone` | `--shadow-film`: `0 1px 2px` black 6%, `0 18px 40px -14px` black 26%. Inside every tile a 1 lit edge, `--film-edge`. Under the spine one soft round shadow |
| **3 Glass** | Bar, dock, trust note, name chip, measurements panel | See Glass | `--glass-shadow` |

`--shadow-thumb` (`0 1px 3px -1px` black 16%, and a 0.5 edge of ink 6%) is for the thumb inside a step or view control. It is not a layer.

| Token | Value | What |
|---|---|---|
| `--z-spine` | 1 | The 3D canvas |
| `--z-stage` | 2 | Films, places, the panel |
| `--z-chip` | 5 | The name chip |
| `--z-bar` | 30 | The bar and the dock |
| `--z-popover` | 40 | The trust note |

Recorded exception to principle 3: the measurements panel holds numbers and is glass. It floats over the page glow at the edge of the stage, never over a film, and uses the thicker fill so its text keeps full contrast.

### Glass

| Part of the recipe | Value | Token |
|---|---|---|
| Fill | White at 66%, 72%, 80% or 88% | `--glass-thin`, `--glass-regular`, `--glass-thick`, `--glass-dense` |
| Blur | `blur(24px) saturate(180%)` | `--glass-blur` |
| Edge | 1 solid, white 70% | `--glass-border` |
| Shadow | Outer hairline ink 5%; top highlight inside, white 90%; lift `0 12px 32px` black 8% | `--glass-shadow` |

| Thickness | Used by | `--ink-2` text on it, over white | Over a black film | It passes 4.5 when what is behind is no darker than |
|---|---|---|---|---|
| Thin, 66% | Bar, dock | 6.6 | 2.8. Fail | `#858585` |
| Regular, 72% | Measurements panel | 6.6 | 3.3. Fail | `#6b6b6b` |
| Thick, 80% | Name chip | 6.6 | 4.1. Fail (`--ink` text, as the chip uses: 12.1) | `#2f2f2f` |
| Dense, 88% | Trust note | 6.6 | 5.0. Pass | Anything |

| | Over the page | Over the spine | Over a film |
|---|---|---|---|
| Thin, regular, thick | Yes | Yes. Bone is light: the darkest bone drawn is about `#686868`, and the blur mixes it with the lighter bone around it | No |
| Dense | Yes | Yes | Yes |

Never glass on glass. If text on glass is hard to read, make the glass thicker. Never outline text on glass. Only a name on film has a casing.

| Setting | Glass becomes |
|---|---|
| Reduced transparency | `--glass-solid` `#fbfbfc`, no blur. Edge and shadow stay. `--ink-2` on it: 6.4 |
| More contrast | White, no blur, a 1 edge of `--line-strong` |

Class: `.glass`. A part chooses its thickness by setting its own `background`.

Open: dark glass. The identity includes a second, dark glass for a control that sits on a film: fill `rgba(24, 24, 26, 0.5)`, `blur(28px) saturate(160%)`, a 1 edge of white 14%, text white 92%. No part uses it, so it is not in `tokens.css` or `components.css`. Its text would read 16.5 to 1 over black film and 7.6 over mid-grey film, but 3.1 over white bone, which fails. Before it is added, either keep it off bright bone or raise the fill to 64% (4.8 over white).

### Motion values

| Speed | Time | Curve | Token | Use |
|---|---|---|---|---|
| Quick | 140 | `cubic-bezier(0.2, 0, 0, 1)` | `--quick` (`--dur-quick`, `--ease-quick`) | Hover, press, anything leaving |
| Appear | 280 | `cubic-bezier(0.22, 1, 0.36, 1)` | `--appear` (`--dur-appear`, `--ease-settle`) | Anything arriving; the thumb sliding |
| Scene | 640 | the same | `--scene` (`--dur-scene`, `--ease-settle`) | A film travelling between moments |

Other curves: `--ease-fade` (`ease`, plain fades), `--ease-meter` (`ease-out`, the progress bar), `--ease-pulse` (`ease-in-out`, the amber pulse).

| Token | ms | What it times |
|---|---|---|
| `--dur-say-out` | 120 | The dock's sentence leaving |
| `--dur-fade` | 160 | Films fading out and in when movement is turned off |
| `--dur-say-in` | 240 | The dock's sentence arriving; everything fading on "Start over" |
| `--dur-fill` | 320 | A step of the progress bar; an outline's fill |
| `--dur-rise` | 420 | The stage on first paint; the spine fading up |
| `--dur-land` | 520 | A film landing; the primary button becoming ready |
| `--dur-view` | 600 | The camera moving to another view |
| `--dur-draw` | 620 | An outline drawing itself |
| `--dur-push` | 800 | A film pushing in or easing back inside its tile |
| `--dur-slow` | 900 | The glow changing; a landing film easing back to size |
| `--dur-pulse` | 1400 | The pulse of outlines that need a look. It peaks 35% of the way through |
| `--dur-light-lead`, `--dur-light` | 700, 4400 | The wait before the light; its journey |
| `--dur-hold` | 560 | After the last vertebra; after "Looks right" |
| `--dur-saved` | 1800 | Kept in the token file; unused by the three built moments |
| `--stagger`, `--stagger-land` | 40, 60 | Between things that travel together; between films as they start to land |
| `--delay-land`, `--hold-land` | 80, 360 | Unused group-landing delay and hold. The hold starts 60 after the last film starts, not after it lands. The built intake uses neither |
| `--hold-leave` | 260 | "Start over": how long the scene fades before it is reset |
| `--delay-in`, `--delay-turn` | 100, 120 | Ordinary dock and spine entry delay; the earlier turn delay. Building and the selected formation use neither |
| `--delay-display`, `--delay-caption` | 160, 200 | The start sentence returning; a film's name returning, and the wait before outline widths follow a scene change |
| `--delay-name`, `--delay-late`, `--delay-marks` | 220, 260, 320 | A vertebra's name, and the panel; an outline's fill, and the first row; the outlines returning |

| Token | Value | What |
|---|---|---|
| `--rise`, `--rise-small`, `--drop-note` | 8, 4, 6 | How far things rise in; the dock's sentence and a lifting place; how far the trust note drops in |
| `--press`, `--press-seg`, `--press-badge` | 0.97, 0.95, 0.98 | Scale while pressed: a button, a step or view option, the trust badge |
| `--from-ready`, `--from-note`, `--from-film` | 0.94, 0.985, 1.06 | Scale things start from: a button becoming ready, the trust note, a landing film |
| `--zoom-find`, `--zoom-find-small`, `--zoom-review` | 1.3, 1.15, 2 | How far a film is pushed in, as a multiple of the film shown whole. The first two are the most: Front and Side in Find, the bending films. The third is exact: during review |
| `--dim` | 0.3 | The other outlines during review |
| `--light-in`, `--light-out` | 0.05, 0.93 | Shares of the light's journey: it has faded in by the first and starts to fade out at the second |

Rules. Things arrive with a fade and a rise of 8, and leave faster than they arrive. Ordinary grouped arrivals are 40 apart; a scene timeline gives its own stagger. Nothing bounces, loops, or moves while the user is idle. Two named work states may loop while the person waits: the meter's waiting light during a running search, and the ring on a busy Save case button. Under reduced motion the meter has a still wash and the button ring stands still.

Reduced motion: fades only. `tokens.css` sets every rise, drop and press scale to nothing. `components.css` stops the rest: films fade out (160), jump and fade in; outlines fade in; the light is hidden; the thumb does not slide. The camera jumps to its view.

The app has its own setting for less motion, `data-motion="reduced"` on its root. It gets the same answers. `tokens.css` and `components.css` each repeat their reduced block for it. Keep each pair equal.

`--stagger-land`, `--delay-land` and `--hold-land` time four films that arrive together. Where each film lands when it has been read, as in the app, they are not used.

### Icons

| Icon | Size | Drawn as | Where |
|---|---|---|---|
| Product mark | 20 (`--icon-medium`) | Three rounded bars, 9 by 3.5 with corners of 1.25, the middle one set 1.5 to the left | The bar |
| Lock | 18 (`--icon-small`) | A body 10.5 by 7 with corners of 2, and a shackle | The trust badge |
| Place drawing, one for each film | 52 by 95 (`--place-drawing-w`, `--place-drawing-h`) | Seven blocks along the curve that film will show | Empty places |

Line 1.5 (`--icon-stroke`), round ends and joins, no fill, the colour of the words beside it. Drawn inline as SVG. No icon in a tinted circle. A new icon needs a reason.

## 5. Layout

One column runs down the middle: bar, stage, dock, notice. The bar and the dock share one width, so their edges line up.

| Part | Size | Place | Tokens |
|---|---|---|---|
| Frame. **Settled** | No wider than 1280. No taller than 1076, so that no film is shown taller than 820 | Centred in the window, with at least 48 clear at each side, or 16 in a window 900 wide or less. Extra window is margin | `--frame-max-w`, `--frame-margin`, `--tile-max-h`, `--frame-w`, `--frame-h` |
| Capsule bar | 720 by 48, or the frame's width if less | 16 from the top | `--col-max`, `--bar-h`, `--bar-top` |
| Trust note. **Settled** | A popover 312 wide | 72 from the top: 8 under the bar. Its right edge on the bar's right edge | `--note-w`, `--note-top` |
| Stage | The frame less 88 at the top and 140 at the bottom | | `--stage-top`, `--stage-bottom` |
| Dock | 720 by 72 | 44 from the bottom | `--dock-h`, `--dock-bottom` |
| Notice | One line | 14 from the bottom | `--notice-bottom` |
| Film rail | Present films, each up to 66 by 132, 12 apart | Left edge of the stage, centred top to bottom | `--rail-h`, `--rail-gap` |
| Measurements panel | 320 wide | Right edge of the stage, centred top to bottom | `--panel-w` |
| 3D canvas | Spans the stage width; from 48 below the frame top to 112 above its bottom | Behind the stage at rest; above it during formation, with no pointer events | `--spine-top`, `--spine-trim` |
| Words in bar and dock | Start 24 from the left edge | | `--inset` |

The frame is built. The canvas keeps its full width during formation and at rest. Resting named views reserve the same room on both sides: half `--rail-h` plus `--tile-gap` in a window 900 wide or less, `--panel-w` plus `--tile-gap` when wider. The spine stays on the frame's centre line. `app/src/bench/three/ThreeStage.tsx` sets this framing; `app/src/bench/three/ThreeStage.css` places its host.

How a film tile gets its size. A tile is always twice as tall as it is wide (`--tile-aspect`). The film inside is shown whole: see Film tile in section 6.

| Moment | Tile height |
|---|---|
| Films | The least of: 62% of the stage's height (`--tile-share`); 420 (`--tile-h-films`); what leaves 60 clear above the sentence and below the names (`--clear`); what four tiles and three gaps of 24 fit across. Sentence, 48, tiles and names are centred in the stage |
| Find | Front and Side: the stage's height less 28 for the name (`--caption-room`). The two bending films share that height in a column: 12 between one's name and the next (`--tuck-gap`). The row is also fitted to the stage's width, and holds only the films that are in. See Find's row, below |
| 3D | The lesser of 132 and what the present films share after their gaps of 12. Only films that are in take a rail place |

Find's row. **Settled.**

- The row is the films that are in, in film order: the large ones first (Front, then Side), then one column for the bending films. It is centred in the stage as one group.
- A film that is not there leaves no hole and no placeholder. Its caption fades out.
- The large films' height is the least of three: the stage's height less 28; 820 (`--tile-max-h`); and what lets the row fit across the stage. It is rounded down to a multiple of 4, so half and quarter sizes are whole.
- What fits across is `(width - 24 × (n - 1 + c) + 10 × c) / (n / 2 + c / 4)`. `width` is the stage's, `n` is the number of large films, and `c` is 1 if there is a column of small films and 0 if not. 24 is `--tile-gap`. 10 is a quarter of 28 and 12 together: the part of a small tile's width that does not shrink with the large one.
- A small film's height is half of what is left of that height after 28 and 12.
- At 1440 by 900 the width is not the least of the three, and nothing moves.

| Films in | What is on the stage |
|---|---|
| Front alone | One tall tile in the middle of the stage, as tall as a film gets there, its name under it |
| Front and Side | The two large tiles as a centred pair, 24 apart |
| Front and one or two bending films | Front, then the column at its right |
| Front, Side and one bending film | The row of four without its lower small tile. The one small tile takes the top of the column, whichever of the two it is |

### The three moments at 1440 by 900

```
1 FILMS                                              window 1440 x 900
+----------------------------------------------------------------------+
|             bar 720 x 48           x 360..1080   y 16..64            |
| stage 1280 x 672                   x 80..1360    y 88..760           |
|                                                                      |
|      Turn four X-rays into a spine you can measure.   y 154..202     |
|                           (48)                                       |
|         +-------+  +-------+  +-------+  +-------+                   |
|         |  208  |24|  208  |24|  208  |24|  208  |    y 250..666     |
|         | x 416 |  | x 416 |  | x 416 |  | x 416 |    x 268..1172    |
|         +-------+  +-------+  +-------+  +-------+                   |
|           Front      Side     Bending    Bending      y 676..692     |
|                                 left      right                      |
|             dock 720 x 72          x 360..1080   y 784..856          |
|                      notice                      y 870..886          |
+----------------------------------------------------------------------+

2 FIND
|          +-----------+  +-----------+  +-----+                       |
|          |   Front   |24|   Side    |24| 151 |  Bending left         |
|          | 322 x 644 |  | 322 x 644 |  |x 302|  y 88..390            |
|          |           |  |           |  +-----+  its name, then 12    |
|          | x 298..620|  | x 644..966|  +-----+                       |
|          | y 88..732 |  |           |  | 151 |  Bending right        |
|          |           |  |           |  |x 302|  y 430..732           |
|          +-----------+  +-----------+  +-----+  x 990..1142          |
|              Front          Side       Bending right  y 742..758     |

3 3D
| +----+                                             +--------------+  |
| | 66 |  y 142..274                                 | Measurements |  |
| |x132|                      spine                  | 320, height  |  |
| +----+  12 between                                 | by content   |  |
| +----+             canvas 1280 x 740               | x 1040..1360 |  |
| +----+             x 80..1360 y 48..788            |              |  |
| +----+  y 574..706                                 +--------------+  |
|  x 80..146                                                           |
```

The stage at this size is 1280 wide, from x 80 to 1360. The rail starts at 80 and the panel ends at 1360. The panel is as tall as its content, up to the stage height, and scrolls inside when needed. The frame reaches its full width in a window 1376 wide; under that it keeps 48 at each side. In a window 1920 by 1080 the frame is 1280 by 1076 and a film is at most 820 tall. In a window 900 wide or less the frame keeps 16 at each side: a window 901 wide has a frame of 805, and one 900 wide a frame of 868. The trust note, 312 wide, is at x 768 to 1080 in a window 1440 wide.

### Other sizes

| Window | What changes |
|---|---|
| 1240 wide or less, or 800 tall or less | The panel tightens: 312 wide, padding 20, rows 12 apart. Tokens are overridden in `tokens.css` |
| 1100 by 760 | Stage 1004 by 532. Find: Front and Side 252 by 504, bending films 116 by 232. With four films in 3D: rail films 62 by 124, filling the stage's height; panel at x 740 to 1052. Bar and dock still 720 |
| Under about 1000 wide | The start sentence takes two lines |
| Under 800 wide | Bar and dock are 720 wide down to a window 752 wide, and narrow below that, keeping 16 at each side |
| 900 wide or less. **Settled** | Nothing overlaps and nothing runs off. The frame's margin is 16. The dock's sentence is set in Body, medium: 15 / 22, 500, tracking 0; the helper, the buttons and the dock's size stay. Find's row is fitted to the stage's width. A line ends in an ellipsis before it can touch a button, including the long Front-only 3D helper |
| 700 by 900. **Settled** | Stage 668 by 672. Films: tiles 149 by 298 under a start sentence of two lines. Find, all four: Front and Side 252 by 504, bending films 116 by 232; the row is 668 wide, edge to edge of the stage. Find, Front alone: 322 by 644. Bar and dock 668 |
| 640 by 800. **Settled** | The smallest window designed for. Stage 608 by 572. Films: tiles 134 by 268. Find, all four: 228 by 456 and 104 by 208. Find, Front alone: 272 by 544. Bar and dock 608. The trust note is at x 312 to 624 |
| 600 wide or less. **Settled** | The bar shows the product mark without the product's name, and the step control sits in the middle of what is left. Under about 573 wide the bar's three parts no longer fit |
| 900 wide or less, in 3D. **Settled** | The measurements wait behind a quiet "Measurements" button in the dock and open as a sheet at the bottom of the stage, above the dock. The rail is hidden and the spine is inert while the sheet is open |

The widths at which things change are named in `tokens.css`: `--break-compact-w` (1240), `--break-compact-h` (800), `--break-narrow-w` (900) and `--break-small-w` (600). CSS cannot read a token in a media query, so the numbers are written out there, and only there. `components.css` holds no window width: a change at a width is made by giving tokens new values in `tokens.css`.

## 6. Components

Shared parts are in `components.css`; the tables name the files that add behavior or a local rule. A part looks the same wherever it is put; the rules that begin `.frame >`, `.stage >` and `body >` place it. State classes `.is-hover`, `.is-pressed`, `.is-focus` and `.is-drag` exist only so a state can be shown without a pointer, in a picture or a test. They are never used in the product.

### Page and glow

| | |
|---|---|
| For | The quiet room everything sits in. The glow gives the glass something to pick up |
| Page | `--page-wash`: white for the top 52%, then easing through `--page-wash-1` at 72% and `--page-wash-2` at 86% to `--page-soft` at the foot. It never scrolls |
| Glow | Two blurred ovals in the same place, centred 46% down the window (`--glow-y`). Quiet, behind the films: `--glow-quiet`, 40% of the window wide and 42% tall, blurred 80. Wide, behind the spine: `--glow`, 50% wide and 56% tall, blurred 100 |
| States | Films and Find show the quiet glow. 3D shows the wide one. They cross-fade in 900 |
| Classes | `body.app`, `.glow` |
| Do | Keep it so soft that no edge can be found |
| Do not | Add a second glow, give it more colour, or draw it as a gradient |

### Start sentence

| | |
|---|---|
| For | Saying what the product does, once, before any film is in |
| Size | Display type, centred, its lines balanced. 48 above the films (`--head-gap`). The sentence, the gap, the films and their names are centred in the stage together |
| States | Shown on the Films moment only. Leaving: fades out in 140 and lifts 8. Returning: after 160, fades in 280 and settles over 640 |
| Classes | `.display` |
| Do | Keep it to one sentence |
| Do not | Use display type anywhere else |

### Capsule bar

| | |
|---|---|
| For | The product's name, where you are, saving the case and the promise that nothing leaves the computer |
| Parts | `.brand` (the mark and `.brand-name`), `.seg.steps`, then `.bar-actions` holding Save case before `.badge`. The three columns remain left, centre, right (`--bar-cols`) |
| Size | 720 by 48, pill, thin glass. Padding 23 at the left; 7 at the right, so the badge is as far from the end as from the top and bottom. 7 from the product mark to the name (`--brand-gap`); the mark is pulled 4 to the left |
| States | Save case is shown once a film that is still present has found vertebrae. While Save is present, at every width, the brand shows its mark without its name and the columns are `auto 1fr auto`. Without Save, the same compact pattern begins at 600 wide or less through `--bar-cols` and `--brand-name-display` |
| Classes | `.bar.glass`, `.brand`, `.brand-name` in `components.css`. `.bar-actions` and `.bar-save` in `app/src/app-shell/TopBar.tsx` and `app/src/app-shell/TopBar.css` group the right-side controls |
| Do | Keep it the same width as the dock |
| Do not | Add another action beyond Save case and the trust badge |

#### Save and reopen. Settled

| | |
|---|---|
| For | Keeping the current films and marks in one file that can be opened again |
| Save | Save case is a quiet button in the bar, before the trust badge. It uses the existing 36-high size and padding. Saving does not change the dock or the current moment |
| Busy | The same button keeps its size and focus, with `aria-busy` and `aria-disabled`, without native `disabled`. Its words become transparent and a 16-wide ring turns in their place, once per 1100 (`--t-reading`). It takes no second press. Reduced motion leaves the ring still |
| Destination | A native save dialog when available; otherwise a download. Both produce one case file. Cancelling the dialog is quiet. If the case changes before its chosen destination is ready, no file is written and the toast asks for another save |
| Confirmation | A native write says "Case saved." only after writing and closing succeed. A download handoff says "Case sent to your downloads." It does not claim the browser finished saving the file. A write failure uses the two sentences in section 8 |
| Open | A case can be dropped or chosen through the ordinary picker. It is checked before the current films are cleared. A valid case replaces them and opens Find when it has found vertebrae; otherwise it stays on Films. A case refused during checking leaves the current case and moment intact. If restoring a checked case fails, the incomplete case is cleared |
| Mixed batch | The first case is taken; every other file in that batch is counted as left out. The toast joins the open or refusal line with that count. A refused case also writes a dock notice only while on Films |
| Kept | The films, their geometry, current marks and names are in the file |
| Not kept | The built 3D spine, measurements, current view, review looks and which finder made the marks are not kept. Reopening clears the review looks and needs a new 3D build. It does not restore the visit as it was |
| Reopened readings | Reopened Front and Side marks use the trained-finder convention because their source is not saved. Front then uses the trained finder's wider error spread; Side keeps the existing spread. Rebuilt bands can differ from the earlier visit |
| Classes and files | `.bar-save.button.quiet`, `.button-content`, `.button-working` in `app/src/building-blocks/Button.tsx` and `app/src/building-blocks/Button.css`. `app/src/bench/save-case.ts` and `app/src/open-and-save/save-file.ts` save; `app/src/bench/open-saved-case.ts` and `app/src/bench/film-intake.ts` open. `app/src/bench/reopened-sources.ts` assigns the reopened sources |
| Do | Distinguish a completed write from a handoff to downloads |
| Do not | Say that review looks or the earlier finder source have been restored |

### Step control

| | |
|---|---|
| For | Where you are in the flow, and the way back |
| Parts | A track (`--track`, 2 padding), a white thumb, three options: Films, Find, 3D |
| Size | Each option 60 by 28. Words 13 / 18, 500. It is as wide as its options only inside a column sized to its content, which the bar gives it. Shown alone, give it such a column |
| States | Rest: `--ink-2`. Hover: `--ink`. Pressed: scale 0.95. Current: `--ink` over the thumb, `aria-current="true"`. Disabled: `--ink-faint`. All three steps are drawn. Earlier steps can be pressed. A later step opens only when it and the steps before it are complete; 3D needs a current fit. Reading films blocks later steps. Focus: the ring |
| Motion | The thumb slides in 280 |
| Classes | `.seg.steps`; `--at` says which option the thumb is under |
| Do | Let a step already reached be a way back |
| Do not | Number the steps or colour them |

### Trust badge and its note. Settled

| | |
|---|---|
| For | Saying, at every moment, that the films stay on this computer; and showing the proof |
| Parts | Badge: lock and "On this device only". Note: the first line, a sentence about the saved file, the request count, then the credit under a hairline. Refused attempts sit under the count only when there are any |
| Size | Badge: 32 tall, padding 10 left and 14 right, 6 between lock and words. Note: a compact popover under its badge, 312 wide (`--note-w`), its right edge on the bar's right edge, 72 from the top. Corner 28, dense glass, padding 16 top and bottom and 24 at the sides, 12 above the hairline and 12 below it. Its height follows the words, including the saved-file sentence and any refused-attempt line |
| Badge states | Rest: `--ink-2`, no fill. Hover, and while the note is open: `--fill`, `--ink`. Pressed: `--line`, scale 0.98. Focus: the ring. `aria-expanded` follows the note |
| Note states | Closed: hidden, 6 higher, scale 0.985. Open: `.open`. Opens in 280, closes in 140, from its top right corner: the corner under the badge. Refused attempts: "The browser blocked 1 attempt to reach outside the app. Nothing was sent." in Caption, `--ink-2`, 8 below the count, shown only when the browser has refused an attempt to reach outside |
| Focus | Focus moves onto the note when it opens, so that its name is read aloud, and back to the badge when it closes. The note is not a control and wears no ring |
| Closes on | A second press, Escape (focus returns to the badge), a press anywhere else, focus leaving it, "Start over" |
| Saved-file sentence | Caption type, `--ink-2`, 8 below the first line. It says what the saved file holds and that the person can move it |
| Classes | `.badge`; `.note.glass`, `.note-lead`, `.note-count`, `.note-blocked`, `.note-credit` in `components.css`. `.note-saved` lives in `app/src/on-this-device/PromisePopover.css`; `app/src/on-this-device/PromisePopover.tsx` supplies the words |
| Do | Keep the count true: it counts what the page has loaded from anywhere but this computer |
| Do not | Put controls in the note. Make it as wide as the bar: a sheet that wide covers the start sentence for three short lines |

### Dock

| | |
|---|---|
| For | The one sentence and the one button of the moment |
| Parts | One `.scene` for each state, all in the same place; only `.scene.on` shows. In a scene: `.say` (the sentence and a helper line), at most one quiet button, then the primary button or the view control |
| Size | 720 by 72, pill, thin glass. Padding 23 at the left; 13 at the right, so the button is as far from the end as from the top and bottom. 6 between things. Sentence 18 / 24, 600; 15 / 22, 500 in a window 900 wide or less (`--type-say`). Helper 13 / 18, `--ink-2`. The amber dot is 7 across, 8 before the words |
| States | Twelve scenes: add, reading, ready, finding, found, review, failed, engine, building, build-failed, three, compare. All stay in the page and one is on. Inactive scenes are inert and hidden from a screen reader. Many states share a scene and change their words in place |
| Motion | The old scene fades out in 120. The new one fades in over 240 after 100, and its sentence rises 4. Words that change inside a scene fade in over 280. Building and three hand over before paint with no delay, fade gap or word animation; the outgoing scene leaves at the same time |
| Read aloud | `.spoken-only`, with `role="status"`, holds the whole sentence and helper of the scene that is on. The count and bar are left out. Words cut short on screen remain whole here. The line lives in `app/src/guide/Dock.tsx`; its clip is in `app/src/look-and-feel/base.css` |
| Classes | `.dock.glass`, `.scene`, `.say`, `.dot`, `.meter` in `components.css`. `[data-fresh]` marks changed words. `.meter.waiting` and the building handoff rules live in `app/src/guide/Dock.css`; scenes live in `app/src/guide/Dock.tsx` |
| Do | One sentence, one helper line, each on one line |
| Do not | Let a line wrap or touch a button: a line too long ends in an ellipsis (Settled). Show two primary buttons. Draw a button for a step that does not exist |

#### Reading. Settled

| | |
|---|---|
| For | Saying that films are being read, which takes a real film a moment |
| Dock | One film: "Reading the film…" over "It goes in the Front place." Several: "Reading the films…" over a count of films read and the bar, "2 of 4". The made-up sample case: "Getting the sample case ready…" over "Four made-up films, drawn by the app." |
| Buttons | No quiet button. "Find the spine", disabled |
| States | It ends when the last film has been read. The ready scene follows |
| Do | Name the place a single film goes to |
| Do not | Show a spinner |

#### Fewer than four films. Settled

| | |
|---|---|
| For | Letting a person go on with the films they have. A film often comes alone |
| Dock | Front alone: "The Front film is in." Two or three, Front among them: "Two films are in.", "Three films are in." Helper: "You can find the spine now, or add the others." Side without Front: "The Side film is in." A searchable Side uses the ordinary can-find helper. Otherwise the helper names its exclusion. With only bending films: "The Front film is still to add." over "The search starts from the Front film." |
| Buttons | "Start over", then "Find the spine". Find is available with a searchable Front or Side film. Side first needs its facing and finder; a 3D build still needs Front |
| States | Scene: ready. "All four films are in." still follows the fourth film |
| Do | Let the search start from the Front film alone |
| Do not | Ask for the other three |

#### A sample film. Settled

| | |
|---|---|
| For | Trying the app on one film where the computer holds sample films of its own |
| Dock | "Sample film 2 of 6 is in." over "Drop your own films at any time." With one sample film: "The sample film is in." |
| Buttons | The quiet button brings the next thing: "Use a sample film", then "Next sample film", then "Use the sample case" after the last. Then "Find the spine" |
| States | Scene: ready. A sample film goes in the Front place. Sample films leave the moment a film of the person's own arrives. "Use the sample case" still brings the made-up case of four |
| Do | Keep it to one control |
| Do not | Show a name from a file. Show anything but made-up films in a picture of the product |

#### A file that cannot be taken. Settled

| | |
|---|---|
| For | Saying, calmly, that a file was not taken, and why |
| Dock | Cannot be read: "One file could not be read." (the number in words to nine) over the fault's own title. A saved case follows Save and reopen above; an invalid case uses its fault title. Several, some read: "The others are in their places." More files than places: "One file was left out." over "Every place is full. Drop on a film to replace it." A film arrived while the search ran: "The search was stopped." over "A film changed. Find the spine again when ready." |
| Buttons | Those of the state the stage is in |
| States | A notice: only the sentence and the helper change. On the stage the place stays empty, or keeps the film it had. The notice goes when films are next taken in, at "Start over", and when "Find the spine" is pressed |
| Do | Quote the fault's own title |
| Do not | Use alarm, or blame the person |

#### The wait before the find's answer. Settled

| | |
|---|---|
| For | Showing that the search is alive before there is anything to count |
| Dock | "Finding the spine…" over the bar, then "Searching the Front film.", "Searching the Side film." or "Searching 3 films." No count. The made-up case's hand-built search adds "About 15 seconds." when searching several films; the trained search does not |
| States | Scene: finding. The bar follows the search's own progress. A wait unchanged for 1500 gains the waiting form described under Progress count. When the answer lands a new, empty bar takes over with the count, so a bar never runs backwards |
| Do | Let the films travel to their Find places at once, shown whole |
| Do not | Show the light, an outline or a count before the answer has landed |

#### Found. Settled

| | |
|---|---|
| For | Ending Find with what was found and the next step |
| Dock | "17 vertebrae found. 2 need a look." over the marked levels, or the estimated-names note. Counts and grammar follow what was found. A trained Side adds its five-outline check sentence; Side alone uses that sentence over "A 3D spine needs a front film." |
| Buttons | "Review the 2" when needed, then "Build the 3D spine". Building is disabled without found Front vertebrae |
| States | Scene: found. Building changes to building at once, without a spinner or meter. After 300, the stage has `aria-busy="true"`. A failed or unfinished fit changes to build-failed with "Try again"; the films stay |
| Classes | `.scene[data-scene="found"]`, `.scene[data-scene="building"]`, `.scene[data-scene="build-failed"]` in `app/src/guide/Dock.tsx`. The build runs in `app/src/bench/three/build-spine.ts` |
| Do | Keep a failed build separate from a failed search |
| Do not | Draw an unfinished spine |

#### The search failed. Settled

| | |
|---|---|
| For | Saying what went wrong with a search, and the way on |
| Dock | Nothing found, only Front searched: "No spine was found on the Front film." over "Check it is a spine X-ray and the right way up." Nothing found on one of several: "One film showed no spine." over "Search Front alone, or go back." A film could not be searched: "The Front film could not be searched." or "One film could not be searched." over the fault's own title. Stopped part way: "The search stopped before it finished." over the fault's own title |
| Buttons | Only Front searched: "Back to the films" as the one button. Several films: "Back to the films" quiet, then "Search Front alone". Stopped part way: "Back to the films" quiet, then "Try again" |
| States | Scene: failed. The films are plain, in their Find places. Anything drawn is cleared |
| Do | Offer the Front film alone when another film fails |
| Do not | Offer "Try again" where the same film would give the same answer |

#### The engine did not start. Settled

| | |
|---|---|
| For | Saying that nothing can be read or searched |
| Dock | The failing engine fault's title plus a full stop, such as "The engine could not start.", over "Reload the page to try again." |
| Buttons | None |
| States | Scene: engine. At any moment. The four places stay as they are; a file given now cannot be read |
| Do | Keep the sentence until the page is reloaded |
| Do not | Add a button that cannot help |

### Primary button

| | |
|---|---|
| For | The one next step |
| Size | 44 tall, pill, 22 each side. Words 15 / 22, 500 |
| States | Rest: `--ink`, white words. Hover: `--ink-hover`. Pressed: `--ink-pressed`, scale 0.97. Focus: the ring. Disabled: `--fill`, words `--ink-3`; it is shown before its time so the next step can be read. Becoming ready: fades in and grows from 0.94 over 520, after 100 |
| Availability | "Build the 3D spine" is drawn during Find because its destination is built. It is disabled until Find is found and Front has vertebrae. No button stands in for an unbuilt step |
| Keyboard | Enter presses it from anywhere that is not another control, when it is drawn and not disabled |
| Classes | `.primary` |
| Do | Name the result: "Build the 3D spine" |
| Do not | Colour it. Show two |

### Secondary button. Settled

| | |
|---|---|
| For | A way out that is not the next step. "Done" in the comparison and "Looks right" in the Side film review |
| Size | The same as the primary button |
| States | Rest: white, a 1 inner edge of `--line`. Hover: `--page-soft`. Pressed: `--soft-2`, scale 0.97. Focus: the ring. Disabled: as the primary button |
| Smaller size | Settled. 36 tall, 14 each side, Small, medium: `data-size="regular"`. For parts that are not on the page; no part on the page uses it |
| Classes | `.secondary`, `.secondary[data-size="regular"]` |
| Do | Use it alone |
| Do not | Put it beside a primary button as an equal choice |

### Quiet button

| | |
|---|---|
| For | Everything a person can do that is not the next step |
| Size | 36 tall, pill, 14 each side. Words 13 / 18, 500, `--ink-2` |
| States | Rest: no fill. Hover: `--fill`, `--ink`. Pressed: `--line`, scale 0.97. Focus: the ring. If it cannot be used it is not shown. One disabled all the same keeps its rest look. "Measurements" keeps the hover look while its sheet is open (`aria-expanded="true"`) |
| Classes | `.quiet` |
| Do | Start with a verb, apart from the named button-wording exceptions in section 8 |
| Do not | Put more than one in the dock |

### Toast. Settled

| | |
|---|---|
| For | Saying that a file was saved, sent to downloads or opened, or that an operation was refused |
| Place | One at the foot of the frame, centred, 18 above its bottom, at layer 14. A new line replaces the previous one. It does not move focus |
| Look | A white slip with a dark hairline, corner 3, no shadow. At least 40 tall, with padding 5 above and below, 12 left and 16 right. Words 13 / 18, weight 400. This look predates the current system and still reads the earlier tokens in `app/src/look-and-feel/earlier-tokens.css`; it has not been restyled as glass |
| Mark | Check for a completed action; Info for a note. Both 16 across, with `--icon-stroke`. The completed mark uses the earlier `--sure`; a note uses `--ink-3`. The words carry the meaning too |
| States | Hidden. Shown with `data-shown`. Replaced by a newer line. Fading out. Save and reopen toasts have no action. The common part can hold one action and returns focus if that action disappears while focused |
| Motion | Fades and rises 16 over 240 (`--t-base`, `--ease-out`). The same transition hides it. Reduced motion shortens that transition to 1; the fixed rise remains. It stays for 4600 (`--t-toast-stay`), paused while hovered or focused inside; reduced motion does not shorten the stay |
| Read aloud | `.toasts-spoken`, with `role="status"`, speaks the current line. The drawn line and its mark are hidden from a screen reader |
| Classes | `.toasts`, `.toasts-spoken`, `.toast`, `.toast-mark`, `.toast-line`, `.toast-clock`, `.toast-action` in `app/src/building-blocks/toast/Toast.css`. `app/src/building-blocks/toast/Toasts.tsx` keeps the current line; `app/src/building-blocks/toast/Toast.tsx` draws it. `app/src/app-shell/AppFrame.tsx` mounts it once |
| Do | Use "Case saved." only for a finished write |
| Do not | Use a download handoff to claim the file was written |

### Text link

| | |
|---|---|
| For | A second way to do the same thing, inside the helper line |
| Size | The size of the words around it, 500, `--ink`, underlined 3 below in `--underline` |
| States | Rest. Hover: the underline turns full ink. Focus: the ring, corner 4 |
| Classes | `.link`, on a `<button>` when it does something and an `<a>` when it goes somewhere |
| Do | Keep it to two or three words |
| Do not | Use it for the main action |

### Empty film place. Settled

| | |
|---|---|
| For | Showing where each film will sit, and taking the films: a press to choose, or a drop |
| Parts | One zone, `.dropzone`, a plain box, holding four `button.place`: one button for each film, each with a line drawing of the spine as that film will show it. A name (`.caption`) under each. As first drawn the zone was one button round four places |
| Size | Each place is the size of a film tile on the Films moment: 208 by 416 at 1440 by 900. Corner 20. Drawing 52 by 95 |
| Press and drop | A press on a place opens the chooser for that place: the first file goes there and the rest fill the empty places in film order. A drop on a place does the same. A drop anywhere else, and the link "choose them", fill the empty places in the order Front, Side, Bending left, Bending right |
| States | The four still change together on hover, on press and under held files: the zone is one object. Rest: `--soft-1`, drawing `--ink-faint`. Hover: `--soft-2`, drawing `--ink-faint-hover`. Pressed: `--soft-3`. Focus: each place has its own ring, the standard one. Files held over the window: `--accent-wash`, drawing `--accent`, a 1.5 inner ring of `--accent-edge`, lifted 4, one after another 40 apart. Filled (`.filled`): fades out as its film lands, and stays under the film |
| Read aloud | "Choose the Front film", and so on for each place |
| Classes | `.dropzone`, `.place`. On `<html>`, `data-films` says what the places are doing. `empty`: some place is empty and nothing is being read. `landing`: something is being read, and the places take no press. `ready`: all four films are in |
| Do | Take a drop anywhere in the window |
| Do not | Add "drag here" words or a dashed edge. Light only the place under the pointer |

#### A place being read. Settled

| | |
|---|---|
| For | Saying that a place is taken while its film is read, which takes a real film a moment |
| Look | The pointed-at look, held still: `--soft-2`, with its drawing in `--ink-faint-hover` |
| States | It takes no press. While any film is being read, no place does. The film lands the moment it has been read |
| Read aloud | "Reading the Front film" |
| Classes | `.place[data-reading]`, with `disabled` and `aria-busy="true"` |
| Do | Hold it still |
| Do not | Loop anything: no pulse, no spinner |

#### The place or film the held files are over. Settled

| | |
|---|---|
| For | Showing where a drop will land, now that each film has a place of its own |
| Look | A firmer ring: an inner ring of 2 (`--ring-width`) in `--accent`. On a place it replaces the 1.5 ring of `--accent-edge`. On a film it is drawn on the tile's lit edge |
| States | Only while files are held over the window, and only on the one place or film they are over. The other places keep the look of held files |
| Classes | `.place[data-drop-target]`, `.tile[data-drop-target]`, under `[data-drag]` |
| Do | Use blue: it already means the place a drop will land |
| Do not | Ring more than one |

#### A place under its film. Settled

| | |
|---|---|
| For | Putting a wrong film right without starting over |
| Look | Unseen: the place has faded out under its film (`.filled`) |
| States | It still takes the press, because a tile takes no pointer on the Films moment. The press opens the chooser, and the film chosen replaces the one there. A drop on the film does the same. The zone stays in reach when all four films are in. Its focus ring shows round the film. On Films, outline polygons let the pointer through too, so a press over a mark reaches this place |
| Read aloud | "Replace the Front film" |
| Classes | `.place.filled`; the zone stays under `[data-films="ready"]`. `app/src/bench/FilmTile.css` makes `.mark polygon` ignore the pointer on Films |
| Do | Keep the same press for choosing and for replacing |
| Do not | Put a button on the film |

### Film tile and its name

| | |
|---|---|
| For | A film. The same four tiles carry through the whole flow |
| Parts | `.tile` holding `.film`, which holds the picture, the light (`.light` in the film viewer's SVG layer) and the outlines (`.marks` in the film viewer's SVG layer). A name, `.caption`, 10 under it |
| Size | Films: 208 by 416, corner 20. Find, Front and Side: 322 by 644, corner 20, pushed in up to 1.3. Find, bending: 151 by 302, corner 14, pushed in up to 1.15. Rail: 66 by 132, corner 10, no name. All at 1440 by 900 |
| Look | Black, `--shadow-film`, a 1 lit inner edge. Name: caption type, `--ink-2`, centred |
| The film in its tile | A film is always shown whole: the largest size that fits its tile, centred on the tile's black. It is never cropped or stretched to fill. A film is not always 1 by 2, and cutting part of it off would hide what was searched. As first drawn the film filled its tile |
| Pushed in | Settled. In Find a film is pushed in from the whole film until the box round its outlines has 24 clear in its tile, with room for the names at its right where names show (`--name-room`). It goes no further than `--zoom-find` for Front and Side, or `--zoom-find-small` for a bending film. The push-in comes when the search's answer lands, because where the spine is, is not known before. On the made-up films of the sample case this gives 1.3 and a nudge of 24, as drawn |
| Film pixels | `--name-room`, `--light-h` and `--light-edge` are written for a film 1300 tall (`--film-h`), as the made-up films of the sample case are. On a film of another height each is scaled by that film's own height over `--film-h`. Settled |
| At rest | A tile at rest is at its real size: `--bw` and `--bh` are its own width and height, and `--s` is 1. For a change of moment, script writes where the tile was into `--x`, `--y` and `--s`, then where it rests, and the tile's own transition carries it. Why: a film viewer measures itself by its box, and a box scaled at rest would put every outline and every press in the wrong place. As first drawn every tile was one size, scaled down |
| States | Not landed: hidden. Landed: `.in`. On the Films moment a tile takes no pointer: a press or a drop on a film reaches the place under it. In the rail: a hand cursor; hover lights the edge to 1.5 of `--film-edge-hover`; a press opens Comparison view, with Enter and Space as well as a pointer |
| Classes | `.tiles`, `.tile`, `.film`, `.caption` in `components.css`. `app/src/bench/FilmTile.tsx` mounts them; `app/src/bench/FilmTile.css` fits the viewer inside. Script sets the tile's numbers. `.caption[data-away]` hides the name of a film absent from Find |
| Do | Move and scale the same tile |
| Do not | Crop a film, or stretch it to fill its tile. Give a tile a border, or put a title bar on a film. Dim, tint or otherwise change the picture to help what is drawn on it: principle 8 |

#### Side film controls. Settled

| | |
|---|---|
| For | Choosing facing before a trained Side search, then checking its names without covering its outlines |
| Parts | A section attached inside the Side tile. The film and controls take separate grid rows; the film shrinks within the same tile. The film is turned upright without mirroring |
| Look | White page fill, ink words, a hairline in `--line`, corner `--r-large`, padding 12 and gaps 8. Words Small; legend Small, medium. Native radios use `--ink` |
| Facing | A native fieldset with two radio choices and no default. Legend: "Which way does the patient face?" The full labels name the left and right of the picture. A choice confirms facing and clears the old search and fit |
| Other states | An attached line says that the finder is waiting, absent or unavailable for this opening. The film stays. Review replaces the facing question when trained Side outlines have been found |
| Classes | `.side-controls` in `app/src/bench/SideFacing.css`, mounted by `app/src/bench/SideFacing.tsx` and `app/src/bench/Bench.tsx`. Side review uses the same section; see Review card |
| Do | Keep controls beside the film in its own box |
| Do not | Choose a facing for the person, or cover a mark with a control |

#### One, two or three films on the Films moment. Settled

| | |
|---|---|
| For | Showing which films are in and which places are still empty |
| Look | Tiles and places side by side, in film order. Each film is in its own place; an empty place keeps its faint look and its name |
| States | A film lands in its place when it has been read. The search can start as soon as the Front film is in. The dock's words are under Dock, Fewer than four films |
| Classes | `.tile.in` over `.place.filled`; `data-films="empty"` on `<html>` while any place is empty |
| Do | Keep all four names in sight |
| Do not | Close up the row round a missing film on this moment |

#### Front alone in Find, and other rows of fewer than four. Settled

| | |
|---|---|
| For | Searching the films that are in. A film often comes alone, with no Side film |
| Look | Front alone is one tall tile in the middle of the stage, its name under it: one object in the middle of the room. Front and Side are the centred pair. A lone bending film takes the top of the column. Section 5, Find's row, has the sizes |
| States | A film that is not there leaves no hole and no placeholder, and its name fades out. Names show on Front and Side only, as with four |
| Classes | `.caption[data-away]` on the name of a film that is not there |
| Do | Centre what is there as one group |
| Do not | Draw an empty tile for a film that was not given |

#### Nothing found on a film. Settled

| | |
|---|---|
| For | A film the search found no spine on, or could not search |
| Look | The film is plain and shown whole in its Find place. No outline, no light, no push-in |
| States | The dock says what happened and offers the way on: see Dock, The search failed |
| Do | Leave the film in sight |
| Do not | Mark the film itself as failed |

### Vertebra outline on film. Settled

| | |
|---|---|
| For | Showing what was found, exactly where |
| Parts | Every outline is drawn twice. First its casing: the same path, wider and dark. Then the line. The faint fill inside lies under both |
| Size | A 1.5 line over a 3.5 casing, whatever size the film is shown at. 1 of casing shows at each side of the line. `--k` is the screen pixels one film pixel covers; script writes it on the tile. Where a film viewer of the app's own draws the film, the viewer supplies it |
| States | Not found: nothing. Found (`.found`): line `--mark`, fill `--mark-fill`; the line draws itself in 620 and the fill follows. Needs a look (`.look`): line `--look`, fill `--look-fill`; pulses once when the find ends; turns white when checked. Selected (`.on`): line `--accent`, fill `--accent-fill`, on every film at once. In review: the one looked at (`.focus`) stays full, the rest drop to 30%. The casing draws, fades and dims with its line. The ordinary layer hides after leaving Find; formation, linked rail marks and comparison have their own visible states. A hidden layer is out of reach |
| Read aloud | A film's outlines: "Outlines on the Front film". One outline: "T3", or "T3, needs a look". An outline not yet found, and every casing, is left out. Amber alone must not carry "needs a look" |
| Casing | Black at 45% under the white line (`--mark-casing`), 65% under the amber line (`--mark-casing-look`), 80% under the blue line (`--mark-casing-on`). The darker the line, the darker its casing has to be. Each value keeps its line 3 to 1 or more against the casing beside it, even over white bone. Section 9 has the measured numbers |
| Paint order | On each film: every fill and casing first, then every line and name. No casing may darken a neighbour's line, and no fill may lighten a casing |
| Classes | Inside `.marks` in the film viewer's SVG layer: first one `g.mark.under` for each vertebra, holding a `polygon` that is the fill and the casing. Then one `g.mark` for each vertebra, holding a `polygon` that is the line, and the name. The two groups of a vertebra take the same state classes |
| Do | Keep it thin and let the bone show through. If a line is ever hard to see, make its casing darker before making it wider |
| Do not | Add drop shadows, heavy boxes or handles. Dim or tint the film: the casing is the only help a line gets |

### Vertebra name on film. Settled

| | |
|---|---|
| For | Naming each vertebra: T1 to T12, then L1 to L5 |
| Size | Mono 500, 11 on screen whatever size the film is shown at. 8 right of the outline's right edge, 4 below its centre |
| Look | Letters in `--mark-name`, white at 92%. Under the letters lies their casing: a stroke 3 wide in `--mark-name-casing`, black at 55%, with round joins, painted before the letters (`paint-order: stroke`). Half of it, 1.5, shows around each letter. The same in every state. Before the casing: white at 80%, nothing under the letters |
| States | Hidden until its vertebra is found; then fades in 280 after 220. Shown on Front and Side during Find (`data-names="on"`), and on the compared film in 3D |
| Classes | The `text` inside each `g.mark` |
| Do | Leave room for it: the view is nudged right by 24 film pixels (`--name-room`, on a film 1300 tall) |
| Do not | Put it in a pill or a box, or give it a drop shadow. Dim or tint the film behind it: the casing is the only help a name gets |

#### Names that are estimates. Settled

| | |
|---|---|
| For | Saying that the names on a film were placed by estimate. The commonest fault is the whole stack one level out |
| Look | The names are drawn as any others |
| Dock | "Names are estimates. Check where T1 is." takes the found helper's place whenever the Front film's names are estimates. The Front naming flags decide this; file names and origins do not. The ordinary Front helper has the amber dot when checks remain. A trained Side adds its own check sentence |
| States | It stays after a review, because it is still true. It keeps the found helper's place even after every "Looks right" |
| Do | Say it every time it is true |
| Do not | Colour or mark the names themselves |

### The travelling light. Settled

| | |
|---|---|
| For | Showing that the app is looking, and where |
| Look | A soft band of light 200 film pixels tall, 22% white at its middle, fading to nothing above, below and at the sides. The 200, and the 60 below, are for a film 1300 tall, and are scaled to the film (`--film-h`) |
| Drawn as | Settled. One shape in the film's own layer: an SVG rectangle as wide as the film, with the stops of `--find-light` down it and of `--find-light-mask` across it, and no blend. So it follows the film through a push-in, and on a film shown whole it never lights the tile's black beside the film. White at a given strength laid over a film adds the same light as the blend first drawn |
| States | Off. Travelling: from 60 film pixels above the highest mark to 60 below the lowest, scaled to the film, on each searched film with outlines. Front sets the shared level times; Side does when Front has no outlines. It starts after the search answers and the films settle |
| Classes | `rect.travelling-light` in `.light`. Script moves it and fades it by writing its `y` and `opacity`. The first band over the tile is gone |
| Do | One pass |
| Do not | Loop it, colour it, or draw a hard scan line |

### Progress count

| | |
|---|---|
| For | How far the find has come |
| Parts | A count, "9 of 17", then a thin bar |
| Size | Count: mono 13, `--ink`. 10, then the bar: 72 by 3, corner 2, track `--line`, fill `--accent` |
| States | "0 of 17", empty. Part way. "17 of 17", full. Each step eases in 320. A bar may stand before words, with no count. In the wait before the find's answer there is nothing to count: the bar, 10, then "Searching the Front film." A new, empty bar takes over when the count starts, so a bar never runs backwards. The same count and bar say how many films have been read: "2 of 4" |
| Waiting | Only a wait, never a count: after 1500 without progress, `.meter.waiting` keeps its fill at 40% opacity. A light 40% of the track wide crosses every 1400 (`--dur-pulse`, `--ease-pulse`). Progress removes it. Reduced motion hides the moving light and uses a still `--accent-fill` track |
| Classes | `.num`, `.meter` (`--p` from 0 to 1) in `components.css`; `.meter.waiting` in `app/src/guide/Dock.css`, timed by `app/src/guide/Dock.tsx` |
| Do | Count whole vertebrae |
| Do not | Show a percentage or a spinner |

### Review card

| | |
|---|---|
| For | Checking a vertebra the app is unsure of |
| Parts | It is not a separate card. It is the dock's review scene and the films pushed in |
| Dock | "T1 needs a look." Under it "1 of 2" and "Check its outline on both films." One button: "Looks right". There are two other helpers. Where the level is on one large film only: "Check its outline on the film." Where the search did not see the vertebra and placed it from the ones beside it: "Filled in from its neighbours. Check it on the film." |
| Films | Large films that have the level push in to 2 times, centred on it. Every other outline drops to 30%, except the separately reviewed trained Side, which keeps all outlines visible |
| States | One for each doubtful vertebra, in order from the top. After "Looks right" the outline turns white, and further presses are ignored for 560 while the next one comes. Escape leaves at any point |
| What a look changes | A look lasts for the visit, including Films and back to Find. "Looks right" turns that level white on every film and takes it out of the found sentence's count. A review left part way leaves the rest amber and counted. Nothing in the case changes, and the next search clears it |
| Classes | `.tiles.reviewing`, `.mark.focus` |
| Do | One vertebra at a time |
| Do not | Add editing tools here. Open: what a user does when the outline does not look right. A pointer has no way out of a review but to finish it or to go back to Films. A quiet "Skip it" beside "Looks right" has been suggested; it is not decided |

#### A Side film's review. Settled

| | |
|---|---|
| For | Checking all five trained Side outlines and their names as one set |
| Parts | The attached `.side-controls` section, named "Review the side film". It says "Names are estimates. Check where L1 is." |
| Buttons | "Looks right" is secondary. "The names are one too high", "The names are one too low" and "Leave the side film out" are quiet buttons |
| States | Waiting for a look. Checked for this marks revision. Renamed. Left out. Renaming starts from the original answer and changes names, not outline positions. Leaving out removes the Side film and clears the fit; without Front marks it returns to Films |
| Front review | All Side outlines remain visible while Front's review walks its levels. The trained Side is reviewed separately |
| Classes | `.side-controls`, `.secondary`, `.quiet`. `app/src/review/SideReview.tsx` mounts the section; `app/src/review/side-review.ts` changes names. Its look lives in `app/src/bench/SideFacing.css` |
| Do | Keep the whole Side answer in view |
| Do not | Move an outline to its neighbour when only its name changes |

### Film rail

| | |
|---|---|
| For | Keeping the films that are in beside the spine |
| Size | Only the films that are in, in the order Front, Side, Bending left, Bending right. Tiles up to 66 by 132, 12 apart, corner 10. The group is centred vertically; a short stage reduces their height. No names |
| States | Rest. Hover: lit edge. Pointer press, Enter or Space opens the comparison. Pointing at a bone shows its blue outline on rail films. While a film is beside the spine, its rail box stays as a faint empty place: `--soft-1`, `--r-small`, no glyph. Pressing it returns the film; the rail does not reflow |
| Classes | `.tile` under `[data-moment="three"]` in `components.css`; `.compare-slot` in `app/src/bench/three/ThreeStage.css`. `app/src/bench/Stage.tsx` keeps the empty rail box; `app/src/bench/FilmTile.tsx` gives rail films their button role, focus and spoken state |
| Do | Keep the order |
| Do not | Let it drift to the edge of a wide window. That is what the frame is for |

### 3D stage

| | |
|---|---|
| For | The spine, alone |
| Bone | `--bone`, matte (roughness 0.72, no metal). The vertebra pointed at is tinted 70% of the way to `--accent`. A soft shade is baked into the bones' hollows, on top of the matte bone: deepest share 0.5, warm tint `#cfc6b8`. |
| Light | Soft light from above, white over warm grey `#d8d2c8`, strength 0.9. Key: white, 1.9, from the upper front left. Fill: cool white `#f2f5ff`, 0.55, from the right. Rim: white, 1.1, from behind. The lights ride with the camera, so the spine is lit the same from every side |
| Shadow | One soft round shadow under the spine: `--spine-shadow` at its centre fading to nothing, shown at 55%, 1.1 times as wide as the spine's footprint. It sits 8 mm under the lowest bone, turns with the camera and tips toward a level viewer. Strength 0.55 at the centre, 0.22 at 45% of its radius, nothing at the edge. |
| Page glow | The wide glow. See Page and glow |
| Line version. Settled | A see-through cage with cells of about 8 mm. Formation adds plain dots at its points and lines 0.75 wide. Lines are `--ink-2` at 55% over the page, white at 60% over film; the ground is chosen pixel by pixel. Dots are ink at 78% or white at 90%, 2.6 across at full scale, with first points 1.3 times larger. Dots shrink with the view's scale over 1.2, clamped from 0.7 to 1. Threads share the 0.75 width, at 45% over the page or white at 50% over film. The far side is at 32% strength |
| Fill. Settled | In the chosen move, each bone fills from its first points outward and from its lines inward, in the fixed scattered order in section 7. Outward growth takes 60% of the fill; closing faces takes the rest, with softness 0.3 of half a cell and reach 1.35. The older height fill remains available in code with edge 0.14 of a bone's height |
| Reveal | The reusable bone reveal fades in and rises 8 mm in the spine's own space. The selected formation sets that rise to zero and grows points, lines and faces instead |
| Views | Front, Side, Top, and the three-quarter view it opens in (34° round, 14° up). A narrow lens of 18°, so it reads almost flat. Front, Side and Top have no perspective at all, so a bone sits exactly on its outline when a film is seen along its own axis; perspective eases in over the first 24° away from the nearest of the three. Top is zoomed 1.7 and puts the front of the body toward the bottom of the picture; the opening view is zoomed 1.02. |
| Pointer | Drag to turn; it glides a little after release. Wheel or pinch to zoom, from 0.7 to 2.2 times. Pointing at or tapping a vertebra tints it and shows the name chip. Reduced motion stops the release glide |
| Keyboard | Front, Side and Top are buttons in the dock. The canvas itself takes no keyboard focus, turning, zooming or bone selection |
| States | Building. Forming, with the host above the stage and pointer events off. Settled. A vertebra pointed at or tapped. Being dragged. Comparing, with hand turning off. Sheet open, with the stage inert. Only fitted T1 to L5 bones are drawn; C7 is not counted as missing |
| Classes | `.spine` is the frame host; `.three-stage` in `app/src/bench/three/ThreeStage.css` fills it. `.spine-3d` and `canvas.spine-3d-canvas` live in `app/src/spine-3d/view/Spine3D.css`. `.glow` lives in `components.css`. `app/src/bench/three/move/move.css` raises the host during formation. Bone, light, shadow and formation values live in `app/src/spine-3d/scene/look.ts`; `--bone` and `--spine-shadow` copy them for CSS |
| Do | Let it float on the page |
| Do not | Add a grid, axes, a floor or a box. Do not turn it on its own |

### View control

| | |
|---|---|
| For | Seeing the spine as a film sees it |
| Size | The step control, larger: each option 64 by 36. It sits where the primary button would |
| States | None chosen (`data-none`, thumb hidden): on arrival, and after a hand turn or zoom. One chosen. Hover, pressed and focus as the step control. Hidden during formation and comparison. It has no disabled look; with a sheet open its choices do nothing |
| Motion | The camera moves in 600 |
| Classes | `.seg.views` in `components.css`, mounted by `app/src/guide/Dock.tsx`. `app/src/steps/3-three/ThreeScreen.tsx` supplies Front, Side and Top; `app/src/bench/three/ThreeStage.tsx` moves the view |
| Do | Front, Side, Top |
| Do not | Add the three-quarter view as a fourth option |

### Measurements panel and row

| | |
|---|---|
| For | A short list of measurements, each with its band |
| Parts | Title. Measured rows. One quiet line for each reason something is not measured. A hairline, the band's meaning and small print. One quiet "Start over" button |
| Row | Name at the left (13 / 20, 500). Value at the right (mono 16 / 20, `--ink`) with its band 6 after it (mono 13, `--ink-2`). Under both, the levels, view and direction (caption, `--ink-2`). Front curves come in the engine's order, largest first, then T5 to T12 kyphosis, lordosis, then twist at the largest curve's apex when measured. The screen applies no curve threshold |
| Size | 320 wide, corner 28, regular glass. Padding 24 at the top and sides, 10 at the bottom. Title to first row 18. Rows 16 apart. Then 20, the hairline, 14, the small print, 6, the buttons. The buttons' words line up with the rows |
| States | Loading: "Reading the spine…". Failed: "The measurements could not be read." with "Try again". Ready: measured rows and distinct reasons. Hidden during comparison. The formation clock brings in the panel and rows as section 7 says; otherwise the panel arrives in 280 after 220 and rows follow 40 apart. Pointer or keyboard focus on a row tints its fitted levels; leaving clears the tint. A focused row has a 2 blue ring, 4 out, with corner 4 |
| Not measured | Never a number, including a typical value. The reason lines in section 8 appear once each. No bending-film line is shown. A not-fitted result fails the readings rather than becoming a row |
| Under the numbers | "Each band covers eight in ten of the spines that fit these marks." appears when there are measured rows. The bone-shape note always follows. Assumed facing adds its note. Estimated Front names add "Names are estimates. Check where T1 is." The narrow sheet has the same notes |
| Classes | `.panel.glass.measurements`, `.measures`, `.m-name`, `.m-value`, `.m-band`, `.m-about`, `.panel-note`, `.panel-acts` in `components.css` and `app/src/bench/three/measurements.css`. `app/src/bench/three/Measurements.tsx` mounts them; `app/src/bench/three/readings-view.ts` supplies rows and reasons |
| Do | Name, value with band, and what it was measured across |
| Do not | Show a number without its band |

### Notice

| | |
|---|---|
| For | Saying what this is not |
| Size | Caption type, `--ink-2`, centred, 14 from the bottom, under the dock |
| States | None. It is on screen at every moment and every window size |
| Classes | `.notice` |
| Do | Keep it word for word |
| Do not | Shorten it, hide it behind a button, or cover it with a sheet |

### Focus ring

| | |
|---|---|
| For | Showing where the keyboard is |
| Size | 2 of `--accent`, 2 out from the part, following its shape. Each empty place has its own; on a place under its film it shows round the film. As first drawn one ring went round all four places, 6 out |
| States | Shown for keyboard focus only (`:focus-visible`) |
| Do | Leave it as it is |
| Do not | Remove it or restyle it for one control. The trust note takes focus only so that its name is read aloud. It is not a control, and it wears no ring |

### Name chip. Settled

| | |
|---|---|
| For | Naming the vertebra pointed at or tapped on the spine |
| Size | 26 tall, pill, thick glass, mono 12 in `--ink`, 10 each side. It sits 14 right of the pointer and 16 below (`--chip-dx`, `--chip-dy`), clamped inside the window |
| States | Hidden. Shown: fades in 140 while a vertebra is pointed at or tapped. Hidden during formation and while the sheet is open. The chip itself is hidden from a screen reader |
| Links | The same level lights on the films in the rail and on the compared film. A focused measurement tints its levels on the spine |
| Classes | `.chip.glass.num`, `.show` in `components.css`; `app/src/bench/three/name-chip.css` keeps one line. `app/src/bench/three/NameChip.tsx` places it on the page. `[data-links]` shows the linked film marks |
| Do | One name, nothing else |
| Do not | Turn it into a tooltip with sentences. Open: selecting a bone and reading its name by keyboard |

### Comparison view. Settled

| | |
|---|---|
| For | Seeing how a film and the standing spine line up |
| Opens | A pointer press, Enter or Space on a rail film |
| Layout | The same film travels to the left half; the spine moves to the right half. Front and Side use the displayed film placement and fitted camera for the same height and scale. Bending films use the standing Front view. The panel is hidden |
| Rail | The empty place keeps the film's box. In a narrow window the other rail films are hidden |
| Links | Pointing at a vertebra on the spine or the compared Front or Side film lights the same level on both. Hand turning is off |
| Leaves | "Done", Escape, the empty rail place, or a second press on the compared film. Done and Escape return focus to that film in the rail |
| Hairline. Decided, being built | `.hair` and `.hair.on-film` remain styles in `components.css`; the app mounts no height hairline |
| Classes | `.tiles[data-compared]`, `.three-stage[data-compared]`, `.compare-slot` in `app/src/bench/three/ThreeStage.css`; `.secondary` for Done. `app/src/bench/Stage.tsx` places the film; `app/src/bench/three/comparison-layout.ts` divides the room and `app/src/bench/three/comparison.ts` aligns the camera |
| Do | Keep the same film and standing spine in sight |
| Do not | Present a bending film as a separate fitted 3D pose |

### Measurements sheet. Settled

| | |
|---|---|
| For | Reading measurements in a window 900 wide or less |
| Parts | A quiet "Measurements" button in the dock, with `aria-expanded` and `aria-controls`. The same panel opens as a sheet |
| Look | Dense glass, corner 28, centred at the bottom of the stage above the dock. Width is the smaller of the dock's width and the stage's full width. Height follows content up to the stage height, with scrolling inside |
| States | Closed and inert. Open: the heading takes focus, the rail hides, and the spine is inert. Escape, an outside press, opening the trust note, comparison, or leaving 3D closes it. The button toggles it. A wider window closes it too |
| Focus | Keys below the sheet's rank are stopped. If focus is still inside when it closes, it returns to the Measurements button. The sheet does not trap Tab |
| Classes | `.panel.glass.measurements.sheet`, `.quiet`, `[data-hidden]`, `.tiles[data-sheet]`. `components.css` places the sheet; `app/src/bench/three/measurements.css` limits its height and sets its layer. `app/src/bench/three/Measurements.tsx` handles focus and dismissal |
| Do | Keep the notice in sight and use the panel's own rows and notes |
| Do not | Add another set of measurements for the narrow view |

## 7. Motion, scene by scene

Times are milliseconds. A search and a fit take their own time; the tables say when each clock starts. All are Settled. A part's hover, press, open and close timings are in section 6.

### Films landing

| When | What happens |
|---|---|
| Files are held over the window | The places turn blue and lift 4, one after another, 40 apart. The place or film under the files has the firmer ring |
| Reading starts | Places stop taking presses. The dock uses its reading scene, with the place name for one film or a count for several |
| Each film has been read | Its tile fades in and rises 8 over 520. The film inside eases back from 1.06 over 900. Its place fades out in 280 |
| The last film has been read | The dock changes to the ready sentence for the films that are in. Find becomes available when a searchable Front or Side film is present |

The old group-landing delay, stagger and hold tokens are not used. Each film lands when it is ready.

### Start of the find

| At | What happens |
|---|---|
| 0 | The step thumb moves to Find in 280. The start sentence fades out in 140 and lifts 8 over 280. The dock says "Finding the spine…", with a progress bar before its helper. "Build the 3D spine" is disabled |
| 0, 40, 80, 120 by film place | Each present film starts travelling to its Find place, 640 each. The delay is 40 per place in film order: Front 0, Side 40, Bending left 80, Bending right 120, whichever films are present. They are shown whole, with no push-in, light or outline before the answer |
| The answer has landed and the films have settled | The reveal clock starts. The films push in over 800. A new, empty bar and count replace the wait's bar |

A wait whose progress has not changed for 1500 takes the meter's waiting form. It ends when progress changes. Only the made-up case's hand-built search adds "About 15 seconds." to its helper, when it searches several films.

### The find itself

This clock starts after the answer and the film travel. The answer sets the levels and their heights; times for individual vertebrae vary with it.

| At | What happens |
|---|---|
| 700 | The light starts on each searched film with outlines. It travels from above the highest mark to below the lowest over 4400 |
| When the light reaches a level on Front, or Side if Front has no outlines | That level's outline draws in 620 on the films that have it; its fill and name follow. The count advances |
| Through the pass | Outlines that need a look are amber. A name appears on a large film after 220, over 280 |
| 5100 | The light's journey has ended |

### End of the find

| When | What happens |
|---|---|
| All outlines have been revealed | Outlines that need a look pulse once over 1400, peaking at 35% |
| 560 after the last outline | The dock says what was found and how many need a look. "Build the 3D spine" becomes ready when Front has vertebrae |

Pointing at an outline then lights the same level on the films. Estimated names keep their note after review.

### Review

| When | What happens |
|---|---|
| Review opens | The dock names the first level. Large films that have it push in over 800 to 2 times. Other outlines drop to 30% over 280. A trained Side keeps its whole set visible |
| "Looks right" | That level turns from amber to white over 280 |
| 560 later | The words fade in over 280 and the films glide to the next level over 800 |
| 560 after the last | The films ease back over 800. The dock returns to found. The estimated-names note stays when true; otherwise, with no checks left, the helper is "All of them look right." |

The trained Side's attached review changes names or removes that film in place. It does not walk Front's review queue.

### Films to 3D. Settled

"Build the 3D spine" first changes the dock to "Building the 3D spine…". There is no spinner or meter. A fit still running after 300 marks the stage busy. Only a usable finished fit begins the move.

The selected points timeline lasts 4020. Its constants are in `app/src/bench/three/move/points-timeline.ts`; `app/src/bench/three/move/points-move.ts` sets the film room and `app/src/bench/three/move/film-flight.ts` carries the walls to the rail. `app/src/bench/three/move/move.ts` runs the clock.

Before that clock starts, the move is held at zero through its first painted frame. Find's real film, crop, outlines and names remain in place during that paint. The following frame hands the picture to the canvas; the vertebra names leave with the page's films at this hand-over frame. The walls carry outlines, without names. Only then may time advance. The canvas is fully visible above the stage, with pointer events off. The building and 3D dock words change before paint, without an empty fade or a word-arrival delay.

| At | What happens |
|---|---|
| 0 | The thumb moves to 3D. The dock says "The spine in 3D." with the helper for the films actually used. The view control waits until the move ends |
| 0 to 220 | Front and Side captions fade. The vertebra names have left at the hand-over frame. Outlines remain on the film walls and travel with them |
| 0 to 640 | Side swings on its inner edge to stand at a right angle to Front. The wall corners are 1.5 mm |
| 0 to 760 | The view gains depth: 42° towards the readable Side wall, elevation 12°, scale 0.95 and perspective 0.6. The fold stays in place, with a drop of 4 |
| 0 to 640; 30 to 670 | Bending left, then Bending right, travel to their rail places |
| 440 to 3600 | The view drifts 6° towards Front |
| 660 + 66 × i | Vertebra i begins in the fixed order below. Its outlines lift together and threads start from the projections of its first points on each film |
| A vertebra's own 0 to 200 | Threads grow square to the films towards the two first points on the fitted bone's surface |
| Its own 185 | The two first points appear. Each point fades in over 110 and settles from 1.7 times size over 260 |
| Its own 230 to 520 | Remaining points spread along the bone from the nearer first point, with fixed timing variation up to 26 either way |
| Its own 300 to 520 | Threads fade away |
| Its own 520 to 840 | Lines draw outward from point to point. Points become the joints of the lines over 180 |
| 2520 + 24 × i | Each bone begins its faces in the same order, taking 440. Faces grow out from the first points and close inward from the lines |
| A bone's fill start + 100 to + 360 | Its points and lines fade as the faces arrive |
| 2900 to 4000 | The page glow widens |
| 3140 to 4000 | The view settles to the resting three-quarter view |
| 3200 and 3260 | Front, then Side, travel to their rail boxes over 720 each, sinking 380 mm back behind the spine. Their crops ease to the rail's whole-film crop. The outline lift eases away from 3200 to 3520 |
| 3440 to 3960 | The shadow arrives last of the spine's effects |
| 3580 to 3860, then 60 apart | The panel, heading and first row start together. Remaining rows, notes and action are scheduled 60 apart. Each is scheduled for 280, but later arrivals are cut short when the move ends at 4020 |
| 3870 to 4010 | The normal rail viewers take over from the travelling walls, in place |
| 4020 | The move ends. Temporary walls and formation drawing clear. Any unfinished panel arrivals are cut short: their inline fade and rise clear, and the ordinary panel styles take over. The view control appears and the spine can be turned |

The order, with i starting at zero: T9, T3, L2, T6, T12, T1, L4, T8, T4, L1, T10, L5, T2, T7, L3, T5, T11. A missing fitted bone is not invented. Points and lines change from white over a film to ink over the page, pixel by pixel.

Front alone uses the same clock, without a Side wall. The wall stands 30 mm behind the fitted bounds. The view turns 36°, with its pivot 0.42 of the way from the wall towards the spine; elevation, scale, depth and drift stay the same. Threads end at the bone's first points without a second film to cross them.

An earlier mesh-first timeline remains selectable through `MOVE_TIMELINE` in `app/src/bench/three/move/move.ts`.

Enter that no control owns, or a pointer press away from the step control, finishes the move at once. The pointer press is consumed. Step controls keep their ordinary action. A resize or a changed motion preference finishes the move too.

Reduced motion uses two `--dur-fade` halves of 160 each. The films fade out during the first half; the spine stays hidden. At the half-way point the films jump to their rail boxes and the spine is whole in its resting view. The films, spine and panel fade in over the second half. There are no standing walls, threads, growing points, lines or faces.

### Back to Find, and back to Films

| When | What happens |
|---|---|
| Leaving 3D | The spine and panel fade out in 140. The films fade out in 160, take their destination boxes, then fade in over 160. The glow narrows |
| Films and Find change between one another | Present films travel in 640, delayed 40 per place in film order: Front 0, Side 40, Bending left 80, Bending right 120. Missing films keep their place in that stagger. Find restores the found framing over 800 |
| Back to Find | Names return after 200 and outlines after 320. The dock returns to found |
| Back to Films | The outlines fade out in 140 and the start sentence returns after 160. A running find stops at once; its drawing clears after 140, once the layer has faded |

### Start over

| At | What happens |
|---|---|
| 0 | The trust note closes. Films, spine and panel fade out over 240 |
| 260 | The case resets without movement. The empty places return. The dock says "Add the four films." |

### The comparison. Settled

| When | What happens |
|---|---|
| A rail film opens | Its rail box remains as a faint empty place. The panel hides. The same film travels to the left half in 640, with its film-order stagger of 40 per place |
| At the same time | The spine moves to the right half over 600. Front and Side use the fitted film camera and displayed placement for matching scale and height. Bending films use standing Front |
| Done, Escape, the empty place or the compared film is pressed | The film returns over 640 with the same stagger; the spine returns to the resting framing over 600. Done and Escape return focus to the film |
| Reduced motion | The camera jumps. The film fades out over 160, changes its box, then fades in over 160 |

There is no moving height hairline in the built comparison.

### Small moves

| What | Timing |
|---|---|
| First paint | The stage fades in over 420 once the fonts and first layout are ready |
| The spine is turned or zoomed by hand | The view control's thumb fades out over 140 |
| Words change inside a scene of the dock | The new words fade in over 280, except the immediate building and three handoff |
| The trust note opens | It drops 6 and grows from 0.985 over 280, from its top right corner. It closes over 140 |
| Save case is busy | Its ring turns once per 1100 until saving ends; reduced motion leaves it still |
| A toast shows or hides | Fades and moves 16 over 240; reduced motion makes the transition 1. It stays for 4600, paused under the pointer or focus |

## 8. Words

| Voice | |
|---|---|
| Plain | Everyday words. Use the same verbs as the buttons |
| Calm | Say what is true. No praise, no alarm |
| Short | One sentence and one helper line. A button usually has two to four words and starts with a verb. Named exceptions: "Done", "Measurements" and "Back to the films" name a destination; "Looks right" confirms a check. "The names are one too high", "The names are one too low" and "Leave the side film out" spell out the Side review choice, even when longer |
| Sentence case | Capitals only where a sentence or a name needs one |
| Never | Exclamation marks, emoji, marketing lines, "please", "oops" |

### Every fixed string

The tables cover Films, Find and 3D as built. Each String cell is a whole string or an example filled by its word function, without editorial words inside it. Counts change; count words run to nine, then digits. The empty helper joins its three word parts around the link. Fault helpers take their title from `app/src/words/faults.ts` and add a full stop. The engine-failed sentence uses the failing engine fault's title plus a full stop too; the title listed below is one example. A write-failure toast joins its two listed sentences with a space. A mixed case batch joins the open or refusal line with the existing left-out count.

**The shell**

| Where | String |
|---|---|
| Browser tab and bar | Spine Workbench |
| Steps | Films |
| Steps | Find |
| Steps | 3D |
| Trust badge and note name | On this device only |
| Trust note | Your films never leave this computer. |
| Trust note, under the first line | A saved case holds your films. It stays on this device unless you move it. |
| Trust note, with the live count; shown here at zero | Requests sent: 0 |
| Trust note credit | Bone shapes: BodyParts3D, © The Database Center for Life Science licensed under CC Attribution 4.0 International. |
| Trust note, one blocked attempt | The browser blocked 1 attempt to reach outside the app. Nothing was sent. |
| Trust note, several blocked attempts | The browser blocked 2 attempts to reach outside the app. Nothing was sent. |
| Notice at every moment | A preview with error bands, not a prediction. Not a medical device. |
| Engine failed, helper | Reload the page to try again. |
| Engine failed, in the dock: the failing fault's title with a full stop; one example | The engine could not start. |
| Bar, quiet action | Save case |
| Toast, completed native write | Case saved. |
| Toast, download handoff | Case sent to your downloads. |
| Toast, write failure, first sentence | The case could not be written there. |
| Toast, write failure, second sentence | Choose another place and try again. |
| Toast, case changed before saving | The case changed. Save it again. |
| Toast, case opened | Opened a saved case. |

**Films**

| Moment | Where | String |
|---|---|---|
| Films | Start sentence | Turn four X-rays into a spine you can measure. |
| Films | Film name | Front |
| Films | Film name | Side |
| Films | Film name | Bending left |
| Films | Film name | Bending right |
| Films | Empty, dock sentence | Add the four films. |
| Films | Quiet action | Use the sample case |
| Films | Quiet action | Use a sample film |
| Films | Quiet action | Next sample film |
| Films | Primary action | Find the spine |
| Films | Quiet action | Start over |
| Films | Reading one, dock sentence | Reading the film… |
| Films | Reading several, dock sentence | Reading the films… |
| Films | Sample case, dock sentence | Getting the sample case ready… |
| Films | Sample case, helper | Four made-up films, drawn by the app. |
| Films | One Front film, dock sentence | The Front film is in. |
| Films | All films, dock sentence | All four films are in. |
| Films | All films, helper | Each one is in its place. |
| Films | Fewer films, helper | You can find the spine now, or add the others. |
| Films | Front absent, dock sentence | The Front film is still to add. |
| Films | Front absent, helper | The search starts from the Front film. |
| Films | One sample, dock sentence | The sample film is in. |
| Films | Sample, helper | Drop your own films at any time. |
| Films | Film-only refusal fallback, helper | This is a saved case, not a film. |
| Films | Some read, helper | The others are in their places. |
| Films | Full places, helper | Every place is full. Drop on a film to replace it. |
| Films | Changed film, dock sentence | The search was stopped. |
| Films | Changed film, helper | A film changed. Find the spine again when ready. |
| Films | Empty helper, link in the middle | Drop them anywhere, or choose them. |
| Films | Reading one, place named | It goes in the Front place. |
| Films | Reading one, place named | It goes in the Side place. |
| Films | Reading one, place named | It goes in the Bending left place. |
| Films | Reading one, place named | It goes in the Bending right place. |
| Films | Reading count, example | 2 of 4 |
| Films | Films in, number filled in | Two films are in. |
| Films | Films in, number filled in | Three films are in. |
| Films | Sample count, example | Sample film 2 of 6 is in. |
| Films | Files unread, number filled in | One file could not be read. |
| Films | Files left out, number filled in | One file was left out. |
| Films | Files unread, number filled in | Two files could not be read. |
| Films | Files left out, number filled in | Two files were left out. |

**Find and Front review**

| Moment | Where | String |
|---|---|---|
| Find | Step | Find |
| Find | Running, dock sentence | Finding the spine… |
| Find | Primary action | Build the 3D spine |
| Find | Found, helper | All of them look right. |
| Find | Review, helper | Check its outline on both films. |
| Find | Review, one-film helper | Check its outline on the film. |
| Find | Review, filled-in helper | Filled in from its neighbours. Check it on the film. |
| Find | Review, action | Looks right |
| Find | Hand-built sample search only, helper | About 15 seconds. |
| Find | Estimated names, helper | Names are estimates. Check where T1 is. |
| Find | Failure, action | Back to the films |
| Find | Failure, action | Try again |
| Find | Failure, action | Search Front alone |
| Find | Failure, dock sentence | No spine was found on the Front film. |
| Find | Failure, helper | Check it is a spine X-ray and the right way up. |
| Find | Failure, dock sentence | One film showed no spine. |
| Find | Failure, helper | Search Front alone, or go back. |
| Find | Failure, dock sentence | The Front film could not be searched. |
| Find | Failure, dock sentence | One film could not be searched. |
| Find | Failure, dock sentence | The search stopped before it finished. |
| Find | Found count, number filled in | 1 vertebra found. |
| Find | Found count, number filled in | 17 vertebrae found. |
| Find | Needs a look, appended to found sentence | 1 needs a look. |
| Find | Review action, number filled in | Review it |
| Find | Needs a look, appended to found sentence | 2 need a look. |
| Find | Review action, number filled in | Review the 2 |
| Find | Found, combined sentence example | 17 vertebrae found. 2 need a look. |
| Find | Marked helper, names or count filled in | L5 is marked on the films. |
| Find | Marked helper, names or count filled in | T1 and L5 are marked on the films. |
| Find | Marked helper, names or count filled in | T1 and L5 are marked on the film. |
| Find | Marked helper, names or count filled in | T1, T5 and L5 are marked on the films. |
| Find | Marked helper, names or count filled in | 5 are marked on the films. |
| Find | Search wait, film count filled in | Searching the Front film. |
| Find | Search wait, film count filled in | Searching 3 films. |
| Find | Search wait, film count filled in | Searching 4 films. |
| Find | Review sentence, level filled in | T1 needs a look. |
| Find | Find count, example | 0 of 17 |
| Find | Find count, example | 9 of 17 |
| Find | Find count, example | 17 of 17 |
| Find | Review count, example | 1 of 2 |

**Side film**

| Moment | Where | String |
|---|---|---|
| Films and Find | Facing legend | Which way does the patient face? |
| Films and Find | Facing radio label | The patient faces the left of the picture |
| Films and Find | Facing radio label | The patient faces the right of the picture |
| Films and Find | Facing absent, helper | Choose which way the patient faces. |
| Films and Find | Finder absent, helper | The trained side finder is not in this build. |
| Films and Find | Capability waiting, helper | Checking whether the side finder is available… |
| Films and Find | Capability failed, helper | The side film is kept, but it cannot be searched here. |
| Films and Find | Opening unavailable, helper | Add this film again before searching it. |
| Films and Find | Found, check sentence | Five outlines on the side film. Check all five and their names. |
| Films and Find | Review, names note | Names are estimates. Check where L1 is. |
| Films and Find | Side alone, helper | A 3D spine needs a front film. |
| Films and Find | Side in, dock sentence | The Side film is in. |
| Films and Find | Side search wait, helper | Searching the Side film. |
| Films and Find | Search failed, sentence | No spine was found on the Side film. |
| Films and Find | Search failed, sentence | The Side film could not be searched. |
| Films and Find | Review, secondary action | Looks right |
| Films and Find | Review, quiet action | The names are one too high |
| Films and Find | Review, quiet action | The names are one too low |
| Films and Find | Review, quiet action | Leave the side film out |
| Films and Find | Review section, spoken name | Review the side film |

**3D and comparison**

| Moment | Where | String |
|---|---|---|
| 3D | Comparison, secondary action | Done |
| 3D | Empty rail place, spoken action | Put the film back in the rail |
| 3D | Front or Side comparison, helper | Point at a vertebra to see it on both. |
| 3D | Bending comparison, helper | Shown beside the standing spine. |
| 3D | Step | 3D |
| 3D | Building, dock sentence | Building the 3D spine… |
| 3D | Build failed, dock sentence | The 3D spine did not finish building. |
| 3D | Build or readings failed, action | Try again |
| 3D | Settled, dock sentence | The spine in 3D. |
| 3D | Paired fit, helper | Built from the front and side films. Drag to turn it. |
| 3D | Front-only fit, helper | Built from the front film. Its depth and side curve are typical values, not measured. |
| 3D | Unused Side, appended helper | The side film could not be used. |
| 3D | Partial fit, appended helper | Some vertebrae were not found, so they are not shown. |
| 3D | View control, spoken name | View |
| 3D | View option | Front |
| 3D | View option | Side |
| 3D | View option | Top |
| 3D | Drawing ready, hidden words | 3D drawing ready. |
| 3D | Comparison, film name filled in | Front film beside the spine. |
| 3D | Rail film, spoken state | Front film. Show it beside the spine. |
| 3D | Rail film, spoken state | Front film. Shown beside the spine. |
| 3D | Comparison, film name filled in | Side film beside the spine. |
| 3D | Rail film, spoken state | Side film. Show it beside the spine. |
| 3D | Rail film, spoken state | Side film. Shown beside the spine. |
| 3D | Comparison, film name filled in | Bending left film beside the spine. |
| 3D | Rail film, spoken state | Bending left film. Show it beside the spine. |
| 3D | Rail film, spoken state | Bending left film. Shown beside the spine. |
| 3D | Comparison, film name filled in | Bending right film beside the spine. |
| 3D | Rail film, spoken state | Bending right film. Show it beside the spine. |
| 3D | Rail film, spoken state | Bending right film. Shown beside the spine. |

**Measurements**

| Moment | Where | String |
|---|---|---|
| 3D | Panel title and narrow dock action | Measurements |
| 3D | Loading | Reading the spine… |
| 3D | Readings failed | The measurements could not be read. |
| 3D | Under the measured rows | Each band covers eight in ten of the spines that fit these marks. |
| 3D | Row name | Thoracic kyphosis |
| 3D | Row name | Lumbar lordosis |
| 3D | Row name | Rotation at the apex |
| 3D | Kyphosis span | T5 to T12, seen from the side |
| 3D | Front curve row name | Upper thoracic curve |
| 3D | Front curve row name | Main thoracic curve |
| 3D | Front curve row name | Thoracolumbar and lumbar curve |
| 3D | Not measured: no-side-film | Side curves need a side film. |
| 3D | Not measured: not-on-side-film | The side film does not show these vertebrae. |
| 3D | Not measured: twist-not-measured | Twist is not measured from these films. |
| 3D | Not measured: no-sacrum | The lower curve is read to L5; the sacrum is not marked. |
| 3D | Not measured: no-curve | No side-to-side curve was found. |
| 3D | Lordosis span | L1 to L5, seen from the side |
| 3D | Lordosis span | L1 to the sacrum, seen from the side |
| 3D | Direction phrase | toward the left of the picture |
| 3D | Curve span and direction, example | Cobb angle T6 to T12, bulging toward the left of the picture |
| 3D | Twist span and direction, example | Apex at T9, turned toward the left of the picture |
| 3D | Direction phrase | toward the right of the picture |
| 3D | Curve span and direction, example | Cobb angle T6 to T12, bulging toward the right of the picture |
| 3D | Twist span and direction, example | Apex at T9, turned toward the right of the picture |
| 3D | Exactly zero unrounded twist, span example | Apex at T9 |
| 3D | Small print, bone shapes | Each bone is a generic shape fitted to this spine's measurements, not the patient's own bone. |
| 3D | Small print, assumed facing | Left and right assume the film is shown the usual way round. |
| 3D | Small print, estimated Front names | Names are estimates. Check where T1 is. |

**Read aloud**

| Read aloud by a screen reader | String |
|---|---|
| Step control | Steps |
| Stage | The films |
| Dock | What to do now |
| Empty place | Choose the Front film |
| Place under film | Replace the Front film |
| Place being read | Reading the Front film |
| Film viewer | Front film |
| Film outlines | Outlines on the Front film |
| Empty place | Choose the Side film |
| Place under film | Replace the Side film |
| Place being read | Reading the Side film |
| Film viewer | Side film |
| Film outlines | Outlines on the Side film |
| Empty place | Choose the Bending left film |
| Place under film | Replace the Bending left film |
| Place being read | Reading the Bending left film |
| Film viewer | Bending left film |
| Film outlines | Outlines on the Bending left film |
| Empty place | Choose the Bending right film |
| Place under film | Replace the Bending right film |
| Place being read | Reading the Bending right film |
| Film viewer | Bending right film |
| Film outlines | Outlines on the Bending right film |
| One outline | T3 |
| One outline needing a look | T3, needs a look |
| 3D canvas | The spine in 3D |


### Names

| Thing | Rule |
|---|---|
| Moments and steps | The three moments and the three steps share their names |
| Films | Always in the order Front, Side, Bending left, Bending right |
| Vertebrae | T1 to T12, then L1 to L5. Always in the mono face |
| States of a vertebra | found, needs a look |

### How a measurement is written

`40° ± 4°`: value and unit, then the band and its unit. The gap before `±` is a margin of 6, not a typed space. Type and colour are in section 6, Measurements panel and row.

| Thing | Rule |
|---|---|
| Value | The reading on the spine as fitted, rounded to a whole degree. Halves round away from zero; negative zero is 0. It is not the median of the versions |
| Band | Take the larger distance from the unrounded reading to the engine band's lower or upper end, then round up to a whole degree. Do not measure from the rounded value shown on screen. The band uses the 10th and 90th percentiles of 32 versions |
| Front curves and twist | A size, with its band, and "toward the left of the picture" or "toward the right of the picture". The effective film facing maps the direction. The direction phrase is dropped only when the unrounded twist is exactly zero. A twist that rounds to 0° can still carry a direction |
| Thoracic kyphosis and lumbar lordosis | Keep the sign. A negative value uses the true minus sign, −, not a hyphen. Forward lean is not shown on screen |
| Thoracic kyphosis | T5 to T12, seen from the side |
| Lumbar lordosis | L1 to the sacrum when measured; otherwise L1 to L5, with the no-sacrum line when that is the reason |
| Missing measurement | No number, including any typical value. Show each reason once. No bending-film line is shown; a not-fitted result fails the readings |
| Repeated reading | 32 versions, seed 7, so the same inputs give the same reading |

While any film's facing is assumed, no screen names the patient's left or right. Directions are sides of the picture. The panel and sheet add the assumed-facing note in the words table. `app/src/bench/three/degree-band.ts` writes the value and band; `app/src/bench/three/readings-view.ts` writes rows, directions and missing reasons.

## 9. Accessibility

| Subject | Rule | Where it stands |
|---|---|---|
| Contrast | Reading text is 4.5 to 1 or more. Rings, lines and marks that carry meaning are 3 to 1 or more | Text on the page and on glass passes: the weakest is `--ink-2` on the view control's track with the glow behind the dock, 5.7. The focus ring is 3.9 on white and 3.4 on the glow. Without a casing, names and outlines on film fail on the sample films. With their built casings, they pass on the measured sample films: see Marks on film, below |
| Colour alone | A state is never shown by colour alone | Needs a look: amber, and the dock names the vertebrae. Each such outline also says "needs a look" when read aloud. Pointed at: blue, with its name beside it on the large films and in the chip on the spine. Current step: the thumb, the darker word, and `aria-current`. Disabled: faint, and the `disabled` attribute |
| Keyboard order | Follow the mounted parts | Steps that can be pressed, Save case when shown, trust badge, then the stage and dock. Films has four place buttons in film order. Side radios and review buttons take native focus. In 3D the measurement rows and Start over precede rail-film buttons and the dock views. The canvas takes no focus |
| Saving focus | Keep the button while its press is carried out | Busy uses `aria-busy` and `aria-disabled`, without native `disabled`, so Save case keeps its focus and size and ignores further presses. Its ring stands still under reduced motion |
| Toast announcements | Say what changed without taking focus | A separate status line speaks the current toast. Save and reopen confirmations have no focusable action |
| Enter | Presses the dock's primary button | From anywhere that is not itself a control |
| Escape | Closes what is open, innermost first | Closes the trust note and returns focus to its badge. Otherwise leaves the sheet, comparison or review at its own key rank |
| Focus | Visible for keyboard focus | The default ring is 2 of `--accent`, 2 out. Measurement rows use a 4 offset and corner 4. Keyboard focus only. The trust note takes focus when it opens, so that its name is read aloud; it is not a control and wears no ring |
| Read aloud. Settled | Each state is spoken once | A line in the dock that is not shown holds the sentence and the helper of the scene that is on. The count and the bar are left out of it: a count that changes seventeen times is not read seventeen times. A line cut short on screen by an ellipsis is still read whole |
| Out of sight, out of reach. Settled | What cannot be seen does not answer the pointer or a screen reader | Outside Find the ordinary outline layer hides after its fade; formation, linked rail marks and comparison make their own visible marks available. An outline not yet found, and every casing, is left out |
| Reduced motion | Fades only | The find keeps its reveal clock, without travelling light or drawing strokes. Formation uses two 160 fades and the resting spine. Named views jump and a hand release never glides. A waiting meter uses a still wash and the busy Save ring stands still. The earlier toast transition shortens to 1 while its 4600 stay remains. System and app preferences get the same answers |
| 3D keyboard | The built controls work without a pointer | Rail films answer Enter and Space. Measurements rows take focus and tint their levels; Front, Side and Top are buttons. Escape closes comparison and the narrow sheet. The canvas itself has no keyboard turning, zooming or bone selection |
| Sheet focus | The heading takes focus on open | The spine is inert and the rail hidden behind it. Escape and an outside press close it. Focus returns to Measurements if it was still in the sheet. Tab is not trapped |
| Reduced transparency | Glass becomes solid | `--glass-solid`, no blur |
| More contrast | Glass becomes white with a clear edge | White, no blur, edge `--line-strong` |
| Smallest text | 12 in the interface, 11 on film | Vertebra names on film are 11 |
| Pointer targets | 24 by 24 or more, except a link inside a sentence | Primary 44 tall. Quiet 36. View option 64 by 36. Trust badge 32 tall. Step option 60 by 28, the smallest control. The text link is 18 tall, the height of its line. An empty film place is the size of a film |

| Marks on film | Before: no casing, names at 80% | With the casing. Settled |
|---|---|---|
| White outlines. 3 to 1 needed | 18 of 34 under: all 17 on the Front film, 1 on the Side film. Worst 1.2 | None under. Worst 3.7, at T8 on the Front film. Weakest stretch 3.06 |
| Amber outlines, with every vertebra tried in amber | 25 of 34 under: 17 on Front, 8 on Side. Worst 1.1 | None under. Worst 4.1. Weakest stretch 3.5 |
| Blue outlines, with every vertebra tried in blue | 29 of 34 under: 17 on Front, 12 on Side. Worst 1.0 | None under. Worst 3.6. Weakest stretch 3.3 |
| Names. 4.5 to 1 needed | 23 of 34 under: 16 on Front, 7 on Side. Worst 1.6 | None under. Worst 6.7, at T7 on the Front film. Weakest single letter 6.2 |

- How these were measured: the sample case's Front and Side films, with their outlines and names on them, were photographed at the size they have in Find in a window 1440 by 900 (322 by 644, pushed in 1.3), at two picture pixels to one screen pixel. Every figure is read from the picture's own pixels, for each of the 17 vertebrae on each film.
- An outline: the pixels wholly covered by its line, against the pixels wholly covered by the casing beside it, on both sides of the line.
- A name: the pixels wholly inside its letters, against the pixels wholly covered by the casing around them.
- "Worst" is the lowest of the 34. "Weakest stretch" is the lowest figure along any 6 pixels of any outline.
- The "Before" column reads the same pixels with the casing left out and the names at 80%.
- At one picture pixel to one screen pixel the answer is the same wherever a pixel is wholly covered: the worst white outline is 3.5.
- Worked out again by arithmetic, not in a browser, with each casing laid over bright bone `#d0d0d0`: name 5.8, white outline 4.2, amber 4.4, blue 3.7.
- The pictures were taken when a film filled its tile. Shown whole, as built, the same film is about a tenth smaller in its tile; lines, casings and names keep their sizes on screen. They have not been measured again.
- With one casing of 45% under all three lines, as first decided, the white lines and the names passed as above. The amber and blue lines did not: 13 of 34 amber outlines and 24 of 34 blue ones stayed under 3 to 1, worst 2.1 and 1.2. Amber and blue are darker than white, so they need a darker casing: 65% and 80%.

| Open | |
|---|---|
| Marks on real films | Only the made-up sample films have been measured. By arithmetic the three outline casings hold 3 to 1 on any film: 3.0, 3.4 and 3.2 over pure white. The name's casing holds 4.5 where the film behind is no lighter than `#f8f8f8`. Over pure white it gives 4.3, and would need 57%. Measure again when real films are in |
| One casing or three | The rule as first decided had one casing for all three lines. Measuring split it by line colour. The split was confirmed on 13 November 2026 after looking at it on the sample films. The other answer that passes, 80% under every line, reads as a heavy black edge on the white lines. As chosen, the casing cannot be told from the line's own edge under white, shows a little under amber, and is a plain dark edge under blue, on the one vertebra pointed at |
| Faint shapes | An empty film place is 1.1 to 1 against the page and its drawing 2.0. The thumb of a step or view control is 1.1 against its track. Words carry the meaning in each case: the films' names and the dock's sentence; the darker word of the current step |
| The glass edge with more contrast | `--line-strong` is 2.7 to 1 against white, under the 3 needed for an edge |
| The amber dot | 2.0 to 1 on white. The words beside it carry the meaning |
| Keyboard and the spine | The spine cannot be turned or zoomed by keyboard; the view control gives three views. A vertebra cannot be pointed at by keyboard, so the name chip has no keyboard selection |
| Keyboard and the outlines | An outline on film can be pointed at, but not reached by keyboard |
| Leaving a review by pointer | Escape leaves a review. A pointer can only finish it or go back to Films. See Review card in section 6 |
| Touch | Targets under 44 have not been tried on a touch screen |

## 10. What would make it look cheap

- A second black pill on screen, or any coloured button.
- A disabled button standing in for a step that does not exist.
- Glass on a film, glass on glass, or a glass card in the middle of the stage.
- A glow whose edge can be seen, or any gradient with colour in it.
- A third colour. Blue or amber used because it looks nice.
- Lines on film thicker than 1.5, with drop shadows, or as filled boxes. Names in pills or boxes. A casing darker or wider than its line needs.
- A film dimmed, tinted or otherwise changed so that the marks on it read better.
- A film cropped or stretched to fill its tile. Light on the black beside a film.
- A number without its band.
- A film or a spine that appears from nowhere. A scene that cross-fades where it could travel.
- Bounce or overshoot. A spinner or idle movement outside the named work states in section 4.
- The bar and the dock at different widths, or their words starting at different places.
- A corner that is not in the Shape table. A sharp rectangle beside a pill.
- Weight 700. All-capitals labels. Text under 11.
- A shadow on anything but a film, glass, the thumb, or the ground under the spine. A shadow under glass darker than 8%.
- A grid, axes, a floor or a box around the spine.
- Icons in tinted circles, or an icon where a word would do.
- Two lines of helper text, or a sentence that wraps in the dock.

## 11. Keeping this current

The rule: a change to a value, a part, a motion or a string on screen is made in one pull request. That pull request changes `app/src/look-and-feel/tokens.css`, `app/src/look-and-feel/components.css`, this document and the screens the change affects, and adds a new line to the decisions log. A change that is in only some of them is not finished.

When two of them disagree: the app is right about what is on screen, `tokens.css` is right about a value, and this document is right about why. Fix the others to match.

| Step | Do this | It is done when |
|---|---|---|
| 1 | Change the value in `tokens.css`, or add a named one beside its neighbours, with a comment that says what it is for | The number is written nowhere else |
| 2 | Change the part in `components.css`, using names only | The search in step 7 prints nothing |
| 3 | Change the screens that use the part, in the same pull request | Every state in the part's table in section 6 can be seen in the app |
| 4 | Search this document for the old value and for the token's name, and change each place. A value is quoted in its table in section 4, in the part's table in section 6, and often in a size in section 5 or a timeline in section 7. A string is quoted in section 8 | The search finds only the new value |
| 5 | Set the status word. When something Decided is built into the app: replace its sketch in `components.css` with the styles the app uses, rewrite its tables from what the app does, and drop the words "Decided, being built" | Nothing built is still marked Decided. Nothing unbuilt is unmarked |
| 6 | Add a dated line at the top of the decisions log: what changed and why. Where the app differs from what was written down before, say so | A reader can tell why from that line alone |
| 7 | Check, with the command below | The search prints nothing, and the screens match sections 5 and 6 |

Run this from the root of the repository.

```
grep -nE '#[0-9a-fA-F]{3,8}\b|rgba?\(|[0-9.]+(px|ms|em|rem|vw|vh)\b' app/src/look-and-feel/components.css | grep -vE '\* 1px|100v[wh]'
```

- The search looks for a colour, a length or a time written as a number in `components.css`. It should print nothing.
- Open the app as well. Go through the three moments at 1440 by 900, 1100 by 760, 700 by 900 and 640 by 800, the four window sizes section 5 gives sizes for, with this document open beside it.

## 12. Decisions log

Newest first. Each line gives what was decided and why.

- **14 November 2026 — Save case writes one file.** The films, geometry, marks and names travel together, so the case can be opened without gathering its films again.
- **14 November 2026 — The two save confirmations say different things.** Case saved follows a completed write and close; Case sent to your downloads follows only the browser handoff, because those are different results.
- **14 November 2026 — Save case sits before the trust badge.** The right-side group keeps the existing quiet-button size; while Save is present the brand name gives way to its mark so the steps and trust badge keep their room.
- **14 November 2026 — A busy Save button keeps focus.** The same button stays in the tab order with busy and unavailable states, ignores repeat presses and shows the existing working ring so the press does not lose its place.
- **14 November 2026 — An outdated save is cancelled.** A destination chosen for one case must not receive a different case after the case has changed.
- **14 November 2026 — A case is checked before replacing the films.** A refusal during checking should leave both the current case and current moment intact; only Films receives the extra dock notice.
- **14 November 2026 — Other files in a case batch are counted as left out.** The first case replaces the current one, so the other files need an explicit account in the toast.
- **14 November 2026 — Review looks are not kept on reopen.** They belong to the visit and the file has no place for them; the rebuilt 3D spine, readings and view are not restored either.
- **14 November 2026 — The finder that made the marks is not kept.** Reopened Front and Side marks use the trained-source convention because the file cannot recover their source; Front uses the trained finder’s wider error spread while Side keeps its existing spread.
- **14 November 2026 — The trust note explains the saved file.** Its added sentence says that the file holds the films and stays on the device unless the person moves it; the note grows with its contents.
- **14 November 2026 — Save and open messages use the existing toast.** Its earlier white-slip look is kept as built rather than given new values; the separate status line says what changed without moving focus.
- **14 November 2026 — Marks let replacement presses through on Films.** The place under a film must receive a press even when the pointer is over an outline.

- **13 November 2026 — Films, Find and 3D are Settled.** The registered steps, frame, intake, finding, review, casing and spoken states are built, so their old implementation statuses no longer describe the app.
- **13 November 2026 — Find leads to a real build.** The build button waits for Front findings, the building sentence appears at once and a failed fit keeps the films with Try again, so there is a visible way on.
- **13 November 2026 — A busy fit marks the stage after 300.** A sentence is enough before then and no spinner or meter is added.
- **13 November 2026 — The canvas keeps the full stage width.** Named resting views reserve the rail symmetrically when narrow and the panel symmetrically when wide, so formation is not clipped and the spine stays centred.
- **13 November 2026 — A Side film has attached controls.** Separate film and control rows keep the question and review off the outlines.
- **13 November 2026 — Facing is a two-choice radio group with no default.** The trained Side search needs the person to choose which way the patient faces.
- **13 November 2026 — The trained Side review checks the whole answer.** Its five outlines stay visible while names can move one level or the Side film can be left out.
- **13 November 2026 — The waiting meter has a quiet moving light.** After 1500 without progress it shows work continuing, as a named work-state exception to the rule against loops while idle; reduced motion uses a still wash.
- **13 November 2026 — The dock keeps one whole spoken line.** Counts stay out of it and words cut short on screen remain available to a screen reader.
- **13 November 2026 — Building and 3D hand over before paint.** Removing the scene delay and word animation keeps readable words through the change.
- **13 November 2026 — The points timeline is the default move.** The fixed 4020 clock and scattered order show where the fitted bones come from.
- **13 November 2026 — The first wall stays until it has been painted.** The real film is retained through that frame so the opening picture is never handed off to an unpainted canvas.
- **13 November 2026 — Faces close from the drawing.** Points and lines grow from the first points, then give way to bone in the same order.
- **13 November 2026 — Film walls travel behind the spine to the rail.** Their actual crops and outlines travel with them and the ordinary viewers resume in place.
- **13 November 2026 — The shadow arrives last.** The spine comes to rest after its faces have formed.
- **13 November 2026 — Reduced motion uses two fade halves.** The resting spine and rail replace formation without travel or release glide.
- **13 November 2026 — A compared film keeps an empty rail place.** The rail must not reflow and the film needs the same place to return to.
- **13 November 2026 — Comparison uses the standing spine.** Front and Side align through their fitted cameras while bending films are labelled as beside the standing view; no height hairline is mounted.
- **13 November 2026 — Measurements are the fitted reading with a whole-degree band.** Using the larger raw band distance preserves both ends even when the median differs.
- **13 November 2026 — Missing readings are lines, never numbers.** Typical values would read as measurements beside fitted ones.
- **13 November 2026 — Curve direction is written as a side of the picture.** An assumed facing must never name the patient’s left or right; kyphosis and lordosis keep their sign with a true minus sign. Forward lean is not shown on screen.
- **13 November 2026 — Thoracic kyphosis is T5 to T12.** The screen and engine use the same span; lordosis reaches the sacrum only when measured.
- **13 November 2026 — The panel explains bands and bone shapes.** Its small print states what is being read, with quiet notes for assumed facing and estimated names.
- **13 November 2026 — Rows and rail films work by keyboard.** Focus can link measurements and Enter or Space can open comparison; free turning and bone naming by keyboard remain open.
- **13 November 2026 — Measurement rows use their built focus ring.** The ring is 2 blue, 4 out, with corner 4, so the document records the row's visible exception to the shared offset.
- **13 November 2026 — Side can be searched alone.** Its facing and available finder let its outlines be checked before Front is added; a 3D build still needs Front.
- **13 November 2026 — Start over is the panel's only action.** The panel offers no saving control; Save case is now in the bar, as recorded on 14 November 2026.
- **13 November 2026 — The list follows the engine's curve order.** Every returned curve is shown, largest first, so the screen adds no second threshold.
- **13 November 2026 — The narrow sheet is the same panel.** Dense glass, internal scrolling, heading focus and dismissal keep the numbers available without covering the dock or notice.
- **13 November 2026 — Fixed strings follow the word files.** 3D, Side and measurement words now match the app and the earlier inactive saving and four-film helper text were removed from the tables. Save case now returns in the bar, with its file confirmations recorded on 14 November 2026.
- **13 November 2026 — Returning from 3D uses a fade.** The built stage changes its persistent film boxes under that fade; travel still joins Films and Find.

**13 November 2026, from building Films, Find and 3D in the app**

Films, Find and 3D are built on the engine. The entries below record the settled choices that reached the app.

- **The 3D view's own choices are recorded (13 November 2026).** The shade baked into the bones' hollows, the see-through line version of a bone, the soft edge of the fill, the 8 mm rise of a reveal, where the shadow sits and how it turns, and the rule that Front, Side and Top have no perspective. Why: these were settled while the view was built, and the app and this document must agree. The points timeline now fills bones in its fixed scattered order, as section 7 records.
- **The trust note is a compact popover under its badge.** 312 wide (`--note-w`), its right edge on the bar's right edge, the count under the first line, the credit under a hairline. It opens from its top right corner. Focus moves onto it so that its name is read aloud; it is not a control and wears no ring. A line under the count says so when the browser has refused an attempt to reach outside. Why: a sheet as wide as the bar covered the start sentence for three short lines, and 312 holds the first line on one line. This replaces the sheet, and settles the point this document left open.
- **Each empty film place is its own button.** The zone is a plain box holding four buttons, one for each film. A press chooses the film for that place; the rest of the files fill the empty places in film order. Hovering still lights all four, because the zone is one object. Each place has its own focus ring, so the old ring round all four is gone. New with it: a place being read holds the pointed-at look; the one place or film that held files are over has a firmer ring; a film is replaced by pressing it, through the place under it. Why: with real films a person often has one or two, and has to say which place a film is for.
- **One, two or three films are enough.** The search starts from the Front film alone. In Find the row is the films that are in, centred as one group; a film that is not there leaves no hole. Front alone is one tall tile in the middle of the stage. `data-films` now has a meaning for any number of films. Why: a real film often comes alone, and one object in the middle of the room is the product's own picture.
- **A film is always shown whole, and a tile at rest is at its real size.** A film is never cropped or stretched to fill its tile. `--zoom-find` and `--zoom-find-small` are now the most a film is pushed in, and the push-in comes when the search's answer lands. `--film-h` (1300) names the film height that the three film-pixel values are written for. Why: a real film is not always 1 by 2, and cutting part of it off would hide what was searched. A tile scaled at rest put every outline and every press in the wrong place for a film viewer that measures itself by its box.
- **Narrow windows are settled.** In a window 900 wide or less the frame's margin is 16 and the dock's sentence is set in Body, medium. Find's row is fitted to the stage's width. A line in the dock ends in an ellipsis before it can touch a button. At 600 wide or less the bar drops the product's name. Why: with a margin of 48 a window 640 wide left a frame of 544, and the dock's found scene needs 572. At 18 / 24 that scene needs a dock 609 to 620 wide; at 15 / 22 the original found scene fits, with no size outside the type table; the long 3D helper still uses the ellipsis rule. This settles the two points left open: the margin under 900, and the Find row running off in a tall, narrow window.
- **A change at a window width is made in `tokens.css`, not in `components.css`.** The dock's sentence reads `--type-say` and `--track-say`; the bar reads `--bar-cols` and `--brand-name-display`. `tokens.css` gives them new values at 900 and at 600, and names the second width `--break-small-w`. Why: CSS cannot read a token in a media query, and `components.css` holds no number. The search in section 11 still prints nothing.
- **The travelling light is a shape inside the film's own layer.** An SVG rectangle with the stops of `--find-light` and `--find-light-mask`, and no blend. The old band over the tile is gone. Why: it must move with the film through a push-in, and on a film shown whole a band laid over the tile would light the black beside the film.
- **"Build the 3D spine" is drawn only when there is a 3D step to go to.** The step control draws all three steps. Find now shows its build button during the search, disabled until Front is found, because the 3D destination is built. Why: a button that can never be pressed is a dead end, and nothing on screen may promise what is not there. A disabled primary button is still shown before its time, but only for a step that exists.
- **The dock has three more scenes: reading, failed, engine.** Words that change inside a scene fade in over 280. A bar may stand before words, with no count, in the wait before the find's answer. What the dock says is read aloud from one line that is not shown. Why: many states share a scene and their words should not snap; a real search has nothing to count until it answers; and a count that changes seventeen times should be spoken once.
- **Films land when they have been read, and the find has a wait.** Each film lands the moment it has been read, and the dock changes when the last one has. The films travel to Find at once; the push-in, the light and the count start when the answer has landed. Leaving Find while outlines are drawing stops the clock at once and clears the outlines 140 later, when their layer has faded. Why: reading and searching a real film take their own time, and an outline half drawn should fade with the rest.
- **What `--hold-land` times is now said as it happens.** Its comment said the dock changes "after the last film lands". It does not: the wait of 360 starts 60 after the last film starts to land, so the dock changes at 680 and the last film has landed at 780. The token comment records that old group landing. The built intake instead lands each film as it is read, as section 7 says. No timing changed.
- **Words for the new states.** Reading films; one to three films in; a sample film from the computer; a file that cannot be taken; the wait; a search that fails; the engine not starting; more forms of the found and review lines; and what is read aloud for places, outlines, the stage and the dock. They are in section 8. "Back to the films" does not start with a verb. It is kept, as "Done" and "Measurements" are, because it names where the press leads.
- **Names on a film that is not made up are estimates, and the dock says so.** "Names are estimates. Check where T1 is." takes the found helper's place. Why: on such a film the names are placed from where each vertebra sits, and the commonest fault is the whole stack one level out.
- **A look lasts for the visit.** "Looks right" turns that level white on every film and takes it out of the count. Nothing in the case changes, and the next search clears it. Why: what a person does when an outline does not look right is still open, so the answer is kept as light as it can be. A quiet "Skip it" was suggested for that point. It is not taken up; the point stays open.
- **Outlines are read aloud, and hidden outlines are out of reach.** "Outlines on the Front film"; "T3", or "T3, needs a look". Why: amber alone must not carry "needs a look", and an outline that cannot be seen should not answer the pointer. The 3D links and comparison show their marks deliberately.
- **Smaller pieces.** The secondary button has a smaller size, 36 tall, for parts not on the page. A quiet button that is disabled keeps its look at rest; the rule that one is not shown still holds on the page. The app's own setting for less motion, `data-motion="reduced"`, gets the same answers as the system's. Where the app's film viewer draws a film it supplies `--k` itself. The casing tokens in the app are this system's own names and values; worked out over bright bone `#d0d0d0` they give 5.8 for a name, 4.2 white, 4.4 amber and 3.7 blue.
- **Two notes from the build were withdrawn and are not rules.** That a popover opening above its control rises from below: the only popover is the trust note, always below its badge. That the step control is given a width of its own: it takes its width from the bar's middle column, and shown alone it is given a column of its own.

**13 November 2026**

- **Marks on film carry a casing, and the film is never altered. Settled.** Every outline on film is drawn twice: a casing beneath, the same path 3.5 wide in black, then the 1.5 line as before. Every name is white at 92% over a 3 stroke of black at 55%, painted under its letters. No pill, box or drop shadow: the casing is the only help. The film itself is never dimmed, tinted or otherwise changed to make a mark readable; this is now principle 8. Why: a mark must be readable on any film, and the film must stay as it is. This replaces the earlier rule that a name may not be outlined. The app draws its outlines this way.
- **The casing is 45% under a white line, 65% under amber, 80% under blue.** The rule as decided gave one casing of 45% for all three. Measured on the sample films, that brought every white outline to 3 to 1 (worst 3.7) and every name to 4.5 to 1 (worst 6.7). It did not bring the amber or the blue line there: 13 of 34 amber outlines and 24 of 34 blue ones stayed under 3 to 1, worst 2.1 and 1.2, because amber and blue are darker than white. The rule for that case is to strengthen the casing in the smallest step that passes, darker before wider. So only the two casings that failed were darkened, and none was widened. Each value is the lightest, in steps of 5, at which every stretch of every outline measures 3 to 1 or more; each also holds over pure white bone. One casing dark enough for all three lines, 80%, was tried and set aside: under the white lines it reads as a heavy black edge. Lighter values, 60% and 75%, pass on each outline's average but not along its brightest stretch. Confirmed on 13 November 2026 after looking at it on the sample films. Section 9 has the numbers and how they were measured.
- **The faint fill is painted under the casing, not over it.** Painted over, the fill lightened the casing's inner half, and the white line fell to 2.8 against it on the whitest bone. Painted under, the 45% that was decided holds. On each film every fill and casing is painted before any line, so that on the small films no casing darkens the line of the vertebra next to it.
- **Names and outlines on film are recorded as failing contrast on the sample films.** Measured first, before there was an answer. Most vertebra names on the Front film sit on film too light for white text, and the white outlines sit on bright bone. The answer is the casing, in the lines above.
- **Dark glass is out of the token file.** The identity includes it. No part uses it, and its text fails over bright bone. Its recipe is written out under Glass in section 4, marked Open, until a part needs it.
- **The font list stops at the system's own face.** An earlier list went on to name further fallback faces. The generic keywords before them already give the system's own face in every current browser, and Geist is a local file, so nothing on screen changes.
- **Values that were bare numbers now have names:** `--hairline`, `--brand-gap`, `--films`, the four sizes of the glow, `--light-in`, `--light-out`, `--hold-leave`. The peak of the amber pulse, 35% of the way through, stays written in `components.css`, because a keyframe cannot read a token.
- **`--tile-h-films` is the new name of the 420 limit on the Films moment.** Its old name was one swap of words away from `--tile-max-h`, the 820 limit that belongs to the frame.
- **A frame with a maximum width. Settled.** The whole scene sits in one centred frame no wider than 1280; a film is never shown taller than 820; at least 48 clear at each side; extra window is margin. Why: on a wide screen the rail and the panel sat at the far edges, a long way from the spine. The frame gives more margin by default, and its maximum width keeps the films at the side from drifting too far out. The margin under 900 wide has since been settled at 16: see "Narrow windows are settled" in the "13 November 2026, from building Films, Find and 3D in the app" group.
- **The move from films to 3D is found in space, point by point. Settled.** Film walls, crossing threads, points, lines and faces carry the films into the fitted spine. Why: each bone should visibly come from its marks. Section 7 gives the selected timeline.
- **Film beside spine. Settled.** A click on a film in the rail shows that film and the spine side by side, matching view, height and scale for Front and Side. "Done", Escape or a second click returns. It replaces the click that went back to Find. A bending film is shown beside the standing spine in Front view. The height hairline remains unbuilt. Why: to see how a film and the 3D spine line up, with one click to open the view and one to close it.
- **Links. Settled.** Pointing at a bone lights it on the rail films; pointing at a measurement tints the vertebrae it is measured across. Why: the same reason, carried to the numbers: every part of the 3D moment should show what it came from.
- **Narrow windows. Settled.** From 900 wide downward nothing overlaps, and the measurements open as a sheet from a quiet button in the dock. Why: without this the panel covers the spine at that size.
- **One stage chosen over One column and Side by side.** Three layouts were tried. One stage keeps one object in the middle of a quiet page, holds the sentence and the button in the same place at every moment, and carries the same four films through. One column unfolds down the page and folds finished steps away, so the films leave the screen. Side by side shows films and spine together from the start, which leaves no single thing to look at. Chosen because the layout keeps one thing in the middle and one sentence beneath it.
- **The dock's sentence is 18 / 24, not the 28 / 34 first written down.** At 28 the dock competed with the start sentence. The 28 / 34 size is not in the system.
- **The page glow is two blurred ovals, not a radial gradient.** A blurred oval has no edge and none of the fine rings a very shallow gradient is drawn with. The page wash starts 52% of the way down, so it begins without a line.
- **The measurements panel is glass.** In the first draft of the identity, content never sat on glass and cards were white with a hairline. The panel now floats over the glow on the thicker 72% fill. Kept as the one exception. No white card exists in the system. Open: what a card looks like if one is ever needed.
- **The trust note is a shallow sheet under the bar, as wide as the bar.** It is the densest glass because it may open over a film. It carries the credit the bone shapes' licence asks for. Its count is real. Since replaced by a popover: see "The trust note is a compact popover under its badge" in the "13 November 2026, from building Films, Find and 3D in the app" group.
- **Film tiles have a heavier, tighter shadow than first written down, and a faint lit edge.** So a film reads as an object and not as a hole in the page.
- **Every control is a pill.** The 10 corner first meant for small controls is now the corner of a film in the rail.
- **One scene speed, 640.** The first draft of the identity gave a range of 520 to 700.
- **The find has a reveal clock after the search answers.** 700 before the light, 4400 for its travel, then 560 after the last outline. Why: actual search time varies, and the drawings should be watched after the films settle.
- **Review was added.** Stepping through the doubtful vertebrae one at a time, with "Looks right" as the only answer. Words added with it: "T1 needs a look.", "Check its outline on both films.", "Looks right", "All of them look right.", "Review it".
- **Other words added since the first draft of the identity:** "All four films are in.", "Each one is in its place.", "T1 and L5 are marked on the films.", "Measurements". Saving now lives in the bar; see the entries dated 14 November 2026.
- **A disabled primary button is shown before its time.** "Find the spine" before films are in, "Build the 3D spine" during the find. The next step can be read before it can be taken. Since narrowed to a step that exists: see the entry "Build the 3D spine" is drawn only when there is a 3D step to go to, in the "13 November 2026, from building Films, Find and 3D in the app" group.
- **The view control starts with no view chosen, and loses its choice when the spine is turned by hand.** The spine opens in a three-quarter view that is none of the three.
- **The bar and the dock share one width (720) and one inset (24).** The product mark is pulled left by 4 so its ink lines up with the dock's words. A control at the end of either sits as far from the end as from the top and bottom: 7 in the bar, 13 in the dock.
- **Films keep 60 clear above the start sentence and below their names.** In Find every top edge, bottom edge and name lines up.
- **Fallbacks for reduced transparency and for more contrast were added.** The first draft of the identity covered only reduced motion.
- **Named, used by nothing:** `--accent-soft`. Kept, marked Open.
- **This system's own choices.** Token names were made regular. The spacing scale is steps of 4, with half steps for the inside of a part. Three distances on neither are named: 22 (`--primary-pad`), 18 (`--panel-list-top`) and 7 (`--brand-gap`). The last was first drawn as a gap of 9 with the mark pulled back by 2; a gap of 7 draws the same. Timings that belong to one scene change are kept exactly, as named values, not rounded to the three speeds. Each part's look is written apart from its place, so a part can be shown on its own. A tile's corner is given as a radius token. The secondary button's pressed fill uses the nearest existing step, `--soft-2`.

**12 November 2026**

- **The flow is three moments: Films, Find, 3D.** Each has one sentence and one button. Left out for now: magnifier, pins, wands, small tool buttons, tooltips, dense side panels, developer views. Why: to keep the whole thing simple, and to work only on the flow of giving the films, getting the vertebrae, and turning them into a 3D spine.
- **The identity, called Clear.** A quiet white room with one object in it. One type family, almost no colour, three layers, glass only for what floats, motion that eases out and settles. The aim: clean, calm, thoughtful and effortless; clear typography, balanced spacing, soft surfaces, subtle layering, smooth transitions; a restrained hint of translucent glass; understated; the important thing obvious, and care in the small details.
- **Ground rules, set with the identity.** Spacing in steps of 4. One black pill per screen. Weights 400, 500 and 600 only. Glass only on floating controls. Text on glass passes 4.5 to 1. Hairlines at 8% black. Grey text and one blue. The reduced-transparency fallback and the focus ring are both checked.
- **Nothing leaves the computer, and only made-up films are shown.** Fonts, films and the 3D spine are local files. The trust badge says so and the trust note counts it.
