# Spine Workbench: design language

The design is called Clinical White: a white, ruled page, with every
film on black. This page and the mock beside it are what the app's look,
wording and motion are measured against.

Where this page and the mock disagree, the mock wins for how something
looks and moves, and this page wins for rules, names and wording. Where
the app differs from both on purpose, the difference is listed in
[where the app differs](where-the-app-differs.md).

| Section | Holds |
|---|---|
| [1. Concept](#1-concept) | the three ideas and the one rule about colour |
| [2. The shell](#2-the-shell) | the frame that never rearranges, its hairlines, and the two grounds: paper and film |
| [3. The flow](#3-the-flow-with-every-state) | every state of every step |
| [4. Screens](#4-screens) | what each screen is for |
| [5. Tokens](#5-tokens) | how the tokens are used; one name, two grounds; confidence in full |
| [6. Typography and icons](#6-typography-and-icons) | three families, two sets of icons |
| [7. Motion](#7-motion) | the named animations and the automatic pass |
| [8. Components](#8-components) | every part, by the folder it lives in |
| [9. Wording](#9-wording) | what the app says and never says |
| [10. Accessibility](#10-accessibility) | keyboard, focus, contrast, speech |
| [11. Growing into Waves 2 and 3](#11-growing-into-waves-2-and-3) | where 3D, measurements and bones attach |
| [12. Build notes](#12-build-notes) | CSS, data attributes, animations, libraries |
| [13. Looking at the mock](#13-looking-at-the-mock) | its states and deep links, and what it fakes |

---

## 1. Concept

**A white, ruled page. Films hang on the bench, each on black, and never leave it. The spine, unrolled, stands between them as a ladder. One sentence says what to do, and one ink button does it.**

Three ideas carry the whole product.

1. **One sentence, one lit button.** The guide on the left always holds one plain sentence about what to do now. Exactly one thing on screen is lit, and it is the next action: the one solid ink button on the page. Everything else that can be pressed is outlined, or plain words. Enter always presses the lit button. When nothing is lit, you are finished.
2. **The bench.** Four films hang in a row where the person put them, each in a black well; everything round the wells is paper. Front and Side stand side by side and travel together, so the same vertebra is always in view on both. The two bending films step aside into a small stack.
3. **The ladder.** The automatic pass lifts the spine off both films, straightens it into a strip, and leaves that strip standing between Front and Side. Each rung is one vertebra, drawn as tall as the vertebra really is. A vertebra that is really two shows up as a rung twice the height of its neighbours before a word is read. The ladder is also the index, the map of where the zoomed films are looking, and the keyboard's way in.

Colour has one rule: **ink means go, colour means data.** The page is white, with near-black ink and one neutral grey scale. The only hues are the four confidence colours, and each is also a line pattern, a glyph and a word.

**Lit** is this document's word, and the code's, for the next action. A lit button is lit the way a sign is: it is the one thing that is on. It is solid ink. It does not glow, and nothing on the page does.

The look is a ruled clinical form or a well-set lab report, not a dashboard. Regions are divided by one-pixel hairlines that run the whole width or height of the window, and by space. There are no tinted panels, no boxes, no shadows and no glass; corners are small and square.

---

## 2. The shell

The frame never rearranges. Learn it once.

```
+----------------------+---------------------------------------------------------+
| mark Spine Workbench | 1 Films - 2 Size & facing - 3 Find - 4 Adjust           |
|      case . saved    | ==========        On this device only | undo redo | Save |
+----------------------+-------+------------+--------+------------+--------------+
| THE GUIDE (368)      | TOOLS | FRONT  ..  | LEVELS | SIDE  ..   | BEND LEFT    |
|  eyebrow             +-------+------------+--------+------------+--------------+
|  One sentence.       | pins  |############| -- T11 |############|##############|
|  One paragraph.      +-------+############| -- T12 |############|##############|
|  [ LIT BUTTON ]      | region|###      ###|        |###      ###+--------------+
|  quiet   quiet       +-------+### film ###| -- L1  |### film ###| BEND RIGHT   |
+----------------------+ bound.|###      ###| -- L2  |###      ###+--------------+
|  CHECKS       2 of 3 +-------+############| -- L3  |############|##############|
|  row                 | magn. |############| -- L4  |############|##############|
+----------------------+-------+############| -- L5  |############|##############|
|  HOW SURE            |       |############|        |############|##############|
+----------------------+-------+------------+--------+------------+--------------+
| Not a medical device. Previews with error bands, never predictions. | mm | 0 sent|
+--------------------------------------------------------------------------------+
        # = a black well        every other line is a one-pixel hairline
```

| Part | Size | Holds |
|---|---|---|
| Top bar | 56 | Over the guide: the mark, the name, and under the name the case name and saved state. Over the bench: the step rail, then the on-device badge, undo and redo, and Save case. |
| Guide | 368 wide (312 under 1200) | Eyebrow, one sentence, one paragraph, one action, at most two quiet actions, then this step's cards, each a ruled section. |
| Bench | the rest | The bays, the lane and ladder, the tool column, playback, toasts, the case sheet. |
| Status line | 28 | The standing claim on the left. Pointer position in millimetres, pixel size and "Requests sent: 0" on the right. |

**The frame is drawn with hairlines.** Each is one pixel of `--rule` and runs the whole width or height of what it divides.

| Hairline | Runs |
|---|---|
| Under the top bar, and above the status line | across the window |
| Between the guide and the bench | from the top of the window down to the status line |
| Under the bays' heads | across the bench: every head is one height, so their rules are one line |
| Between bays, and either side of the ladder and the tool column | down the bench |
| Between the guide's sections | across the guide, edge to edge |
| At the top of every rung | across the ladder, strip and all |
| Above playback, and above the line of small print under the first screen | across the bench |

Nothing is boxed. No region has a fill, a radius or a shadow of its own, and bays touch: there is no gap between them and no margin round the bench.

**A card is a ruled section.** What the guide holds under its sentence keeps the name card in this document and in component names. On screen a card is a hairline across the guide, a small heading in capitals with a count at its right, and its rows, with lighter hairlines between them. The last card is ruled off below.

**Where you are is said in ink.** The current step's name is ink, with a two-pixel ink line under it that sits on the top bar's own rule. The tool in hand has the same line down its edge. A step that is done shows a tick in a ring; one to come is grey.

**Save is an action, not a step.** The rail has four steps in Wave 1 and grows by appending (5 3D spine, 6 Measure, 7 Bones). When it has more than four, finished steps fold down to their tick so it still fits.

**The bench has one layout function.** Given the step and the bench size it returns a rectangle for each bay and for the lane. Bays glide between rectangles; they never jump.

| Step | Bays |
|---|---|
| Films | Four equal columns. An empty column is paper and holds the shape of its film in a dashed line, with a figure and a caption; a column turns black when its film is hung. One line of small print runs across the foot of the bench. |
| Size & facing | Four in a row. The film that needs an answer steps forward (2.5 times as wide); the others become thumbnails with two ticks in their heads. When all are set, four equal. |
| Find | Front and Side large with the bending stack dimmed. At 3.35 s the two films part and the lane opens between them, black. Playback is a ruled strip across the foot of the bench. |
| Adjust | Tool column, Front, ladder, Side, bending stack. Clicking a bending film swaps it in beside Front. |

### Paper and film

Everything on screen is on one of two grounds, and each thing is drawn for the ground it is on.

| Ground | What is on it | How it is drawn |
|---|---|---|
| Paper | The top bar, the guide, the status line, every bay's head, the ladder, the tool column, playback, and everything that floats: the popover, the case sheet, a toast, a tooltip, the word at the pointer, a note or a box laid on a film. | White (`--room-0`), near-black ink, one neutral grey scale. Divided by hairlines and by space. |
| Film | A hung film and the well behind it; every outline, name, ruler, letter and ring drawn on it; the lane while the pass plays; the unrolled strip in the ladder; the magnifier; the small films in a choice card and on the case sheet. | Black (`--well`). Never themed, never inverted, never tinted. Light ink, and the confidence colours over a dark casing. |

- **A film is read on black.** The well behind a hung film is black to the edges of its bay, in every step. A bay that is empty, reading or unreadable has no film, so it is paper.
- **Where the two meet, they meet square.** A well has no radius, no border of its own and no shadow. Black stops at the hairline under the bay's head and at the hairline between bays. Two wells that touch have one hairline between them.
- **Paper laid on a film.** A label on a film (the note that points at the lead marker, the box that asks a length, the count during the search, a toast, the word at the pointer) is white with a one-pixel ink keyline. White on black needs no shadow.
- **Nothing but a mark is laid on a film.** A rung's tint in the ladder stops at the strip and starts again after it. The one exception is the veil under the case sheet, which is paper: the whole bench falls back to a faint print.
- **One token name, two values.** Each ink, line, lumen and confidence token has an on-paper value and an on-film value. A part uses the plain name and is given the value for the ground it stands on (section 5).

---

## 3. The flow, with every state

For each state: what is seen, the one obvious action, and how the person knows it worked. Every state below is in the mock. The name in brackets is its entry in the mock's jump menu.

### Step 1. Films

**1.1 First launch** [First launch]
- Sees: four ruled columns, each holding the shape of its film in a dashed line, with a line figure and a caption (Standing, from the front; Standing, from the side; Leaning to the left; Leaning to the right). Across the foot of the bench: "Films stay on this computer. Drop files anywhere. PNG, JPEG or DICOM". Guide: "Hang your four films." Cards: Films 0 of 4; What happens to a film (Kept, Dropped, Sent anywhere: never).
- Action: the lit button **Choose films** (Enter). Dropping files anywhere, or clicking a slot, does the same. Quiet: Open a saved case; Try a made-up spine.
- Worked: the slots start reading.

**1.2 Films being read** [Films being read]
- Sees: a grey band rising in each waiting slot, with a hairline at its edge, and "Reading…" in its head. As each film lands its column turns black, the film flickers on and the picture develops into focus; the head says "Ready" with a tick; the card counts "1 of 4" and says "Ready. Private details dropped." Guide: "Reading your films…"
- Action: none. Nothing is lit, so there is nothing to do.
- Worked: "Four films are up."

**1.3 A film that cannot be read** [A film can't be read]
- Sees: that one slot turns to a dotted red frame over a faint red wash, with a crossed-file icon: "Can't read this file. It isn't a PNG, a JPEG or a DICOM image. Nothing else changed." It carries its own **Choose another** button. The other three stay up, on black. Guide: "One film couldn't be read." naming the film.
- Action: lit **Choose another file**. Quiet: Start over.
- Worked: the slot reads, its column turns black and the film lamps on; the count is 4 of 4.

**1.4 Four films up** [Four films up]
- Guide: "Four films are up." Lit **Check size and facing**.

### Step 2. Size & facing

Two facts per film: how big a pixel is, and which way it faces. Nothing is resized and nothing is mirrored. These are facts stored beside the film.

**2.1 Left or right, front film** [Left or right]
- Sees: the front film steps forward. A paper label points at the lead marker: "Lead marker found. It reads **L**." A large serif **?** stands outside each edge of the film. Hovering a half of the film tints it. Guide: "Which side is the patient's left?" with two equal cards, ink-lined and neither solid, each showing which half it means on a small black film; one carries the tag "marker". Card: Two facts per film, with where each fact came from in plain words.
- Action: click that half of the film, or a card, or press the left or right arrow. **There is no default and Enter does nothing**, because a mirrored spine still looks normal and file tags are unreliable.
- Worked: the letters lock (**L**, white, "patient's left, confirmed"; **R**, "patient's right"), then glide onto the film's top corners as small badges. The chip in the film's head reads "L on the right" with a person icon. A toast says whether it matched the marker. Choosing against the marker is accepted and said plainly: "Kept as you said. That is the opposite of the lead marker."

**2.2 A film with no size** [No size: two-click ruler]
- Sees: the bending-right film steps forward. Its ruler is amber and reads `px`; its chip reads "size ?". Guide: "Bending right has no size."
- Action: lit **Measure with two clicks**. Quiet: Same as the front film.

**2.3 Measuring** [Ruler: measured]
- Sees: a ring on one end of the marker ball; guide "Click one end." then "Now the other end." After two clicks a white line joins them and a paper box beside it reads "This distance is [25] mm". Guide: "How long is that?" with the pixel length and the size that follows.
- Action: type the length; Enter or lit **Set the size**.
- Worked: the ruler turns from amber `px` to `mm`, the chip reads the measured size with a person icon, toast "Size set from your two clicks: 0.151 mm per pixel."

**2.4 All set** [Films all set]
- Sees: four equal films, each with a millimetre ruler and ticked chips. Guide: "All four films are set." Lit **Find the spine**. Quiet: Change left and right.

Side-film facing and upside-down films are found automatically (the upper back bulges backward; skull and pelvis say which way is up). They arrive ticked, with the reason in the card. Only front-film left and right always needs a person.

### Step 3. Find

**3.1 The automatic pass** [Automatic pass]
- Sees: the six-beat pass (section 7). Guide: "Watch it find the spine." "About twelve seconds, slowed down so you can follow it. Nothing is final until the last step, so an early outline may still move." Card: What it is doing, six plain lines with the live one opened. Card: How sure, counting up as colour arrives. Playback under the films: watch again, play or pause, a six-part bar, speed, skip.
- Action: none needed. Quiet: Skip to the result. Space pauses. The bar scrubs.
- Worked: outlines lock, names roll in, colour arrives, the strip becomes the ladder.

**3.2 The result** [Pass result]
- Sees: 17 outlines on both films in their confidence colours; the ladder, on paper, with one needs-you rung twice as tall as its neighbours. Guide: "Found 17. Four need a look." Cards: Checks (1 of 3 pass); How sure (13, 2, 2, 0).
- Action: lit **Look at the first one**. Quiet: Watch again.

### Step 4. Adjust

The review queue walks to each thing that needs a person. For each item the films travel to it, the right tool is in hand, and a ring breathes exactly where to click. The lit button and the ring are one action: clicking either does it. The button is solid ink and still; the ring, on the film, is the one thing that breathes. Escape leaves the queue; the guide then offers **Go to the next one**.

**4.1 Which vertebra is L5** [Review: which is L5]
- Sees: both films on the two candidates, everything else dimmed, badges **A** and **B** on the films. Check: "Level names may be off by one. Counting from the first rib and from the sacrum disagree." Guide: "Which vertebra is L5?" with the one fact needed ("L5 is the last vertebra above the sacrum") and two equal cards: A, the upper one, counted up from the sacrum; B, the lower one, counted down from the first rib.
- Action: click A or B on the film or the card, or press A or B. No default.
- Worked: names ripple; the one below L5 becomes a grey dashed "S1", not counted; the check becomes "Level names set. L5 confirmed by you"; toast with Undo.

**4.2 A missing vertebra** [Review: split]
- Sees: check "16 of 17 vertebrae found. One missing between T1 and L5." The tall outline is selected, the boundary wand is in hand, a ring breathes on the disc gap inside it, and its rung on the ladder is twice as tall. Guide: "One vertebra is missing."
- Action: lit **Split T12 at the gap** (Enter) or click the ring. Quiet: It really is one; Skip for now.
- Worked: a white line flashes across the gap, two white outlines appear, the rung splits and its neighbours glide apart, every changed name rolls, the check reads "17 of 17 vertebrae found", toast "Split T12 in two. Levels renamed to match." with Undo.

**4.3 A vertebra that needs you** [Review: fix T3]
- Sees: T3 in coral, dotted. The region wand is in hand and a ring sits on the bone the outline missed. Guide: "T3 is hard to see."
- Action: lit **Add the missed bone** or click the ring. Quiet: It looks right; Skip for now.
- Worked: a soft disc spreads from the click, the outline refits, T3 turns white with a tick.

**4.4 A vertebra worth a look** [Review: confirm T1]
- Sees: T1 in yellow, dashed; one pedicle ring drawn hollow and dashed. Guide: "Is T1 right?" "One pedicle could not be seen and is marked **not visible**, not guessed."
- Action: lit **Looks right**. Or drag a corner.
- Worked: T1 turns white with a tick.

**4.5 Free adjusting** [Free adjusting]

| Tool | Key | A click does | The word at the pointer |
|---|---|---|---|
| Pins | V | Drag any corner. It stays where it is put, filled white, and its neighbours re-solve around it. The magnifier opens while dragging. | "Drag to pin this corner" |
| Region wand | R | Click bone the outline missed to add it. Click inside an outline to trim it. The wall that will move lights first. | "+ add to T8" or "− trim T10" |
| Boundary wand | B | Click a disc gap to split one vertebra into two. Click a boundary to merge two into one. A ghost line shows which. | "Split T12 here: gap found", "Merge T9 and T10", "No disc gap found inside T8" |
| Magnifier | M | A round 2.6 times view beside the pointer, never under it, showing film and marks together. | none |

Also: the wheel zooms at the pointer and the partner film follows to the same level; drag pans when zoomed; 0 fits; double-click fits; plus and minus zoom. Clicking a rung travels both films to that level. Hovering a vertebra or a rung lights it on every film and on the ladder. A split or merge on one film applies to all four. Bending films carry endplate lines only.

**4.6 Undo and redo**
- Cmd or Ctrl+Z, the top-bar arrow (its tooltip names the action: "Undo: Split T12"), or the Undo button on the toast. Shift adds redo.
- Worked: outlines and rungs glide back, names roll back, the check and the queue item return, toast "Undid: split T12."

**4.7 All clear** [All clear]
- Guide: "Everything checks out." "17 vertebrae, in order, named T1 to L5. Four were set by you." Lit **Save the case**.
- If things were skipped or the count is off the sentence says so, and Save is still the lit action: "You can save now and finish later. The case file remembers what is still open."

### Saving

**5.1 The case sheet** [Save sheet]
- Sees: a paper veil over the bench only, so the films fall back to a faint print. A sheet of paper with an ink keyline: "Save the case.", the file name, four small films, fanned, and what goes in: four films under fixed names (your file names are not kept); 17 vertebrae with every corner, pedicle mark and pin; sizes, facings and which side is left, as confirmed; every check passes, or how many things are still open. Below: "Written to this computer and nowhere else. Extra DICOM tags are not inside it. Not a medical device."
- Action: lit **Save case file** (Enter). Quiet: Not yet. Cmd+S opens the sheet at any time in Adjust. While the sheet is open its button is the lit one, and the guide's own button stands down to an outline: there is still one ink button on screen.

**5.2 Saved** [Saved]
- Worked: the lines tick one after another, the four films fold into one ink-lined case tile, the title becomes "Saved to this computer.", the top bar's ring becomes a solid dot beside "Saved just now", the guide says "Nothing was uploaded." **Nothing on the page is lit**: the tile is lined, not solid, and the sheet's two buttons are outlined. Any later edit flips the top bar back to "Changes not saved".

Saving is never blocked by open checks. The sheet says what is still open and saves it as open.

### Reopening

**6.1 Opening a case** [Reopen a case]
- Sees: guide "Opening the case…" and a card, Reading strictly, whose four lines tick in turn: it is a spine-case file, version 1; four films under their fixed names; the data file matches the format exactly; 17 vertebrae, pins and L5 as saved.
- Worked: the bench opens in Adjust exactly as saved. Guide: "Everything is as you left it." "Read strictly from the case file: 4 films, 17 vertebrae, 4 of them set by you."

**6.2 A case that will not open** [Case won't open]
- Guide: "This case file can't be opened." "Its data file does not match the format this app reads (spine-case, version 1). Nothing was changed and nothing was guessed." Lit **Choose another file**.

### Always there

**The promise** [On this device]
- The badge "On this device only · 0 sent" opens a paper popover with an ink keyline: "Nothing leaves this computer." Three guards in plain words (the browser blocks it; the code is not allowed to ask; a test watches the wire), a large "0 requests sent this session", and "Turn the Wi-Fi off and everything still works." The same zero is always in the status line.
- In the app the zero is a real count, made in the page: every request that would leave the page's own origin, and every attempt the content policy blocked.

**Reduced motion** [toggle in the mock menu]
- Scenes change at once, nothing breathes, and the pass becomes six still pictures, one per finished beat, 750 ms apart.

---

## 4. Screens

| Screen | Purpose | Signature moment |
|---|---|---|
| The bank (Films) | Get four films in and make the on-device promise before any file is touched. | Lamp-on: a white column turns black, the film flickers on like a viewbox and the picture develops from a bright blur. |
| The question on the film (Size & facing) | Two facts per film, and one decision only a person may make. | The film that needs you steps forward. The letters lock and glide onto its corners. A ruler turns from amber px to mm. |
| The pass (Find) | Show how the answer was reached, so it is trusted the right amount. | Two ribbons peel off two films, straighten, and are stitched together level by level. Colour arrives last. |
| The workbench (Adjust) | Fix what is wrong in as few clicks as possible, and know when it is right. | The ring and the lit button are one action. A split makes a rung come apart and every name below roll. |
| The case sheet (Save) | One file, visibly local. | Four small films fold into one ink-lined tile. Then nothing is lit. |
| The promise (always) | Make "nothing leaves this computer" checkable in ten seconds. | A large zero. |

---

## 5. Tokens

The tokens are one file, [`tokens.css`](tokens.css), beside this page. The app's copy is `app/src/look-and-feel/tokens.css`, and a test holds the two to the same values. Read the file for the values; this section says how they are used.

| Token group | Rule |
|---|---|
| `--room-*` | Paper. `--room-0`, `--room-1` and `--room-2` are all white: the page, the bars, a section. `--room-3` is a row or a control under the pointer; `--room-4` is pressed or selected. None is ever a panel's fill. |
| `--well` | Black. Behind a hung film, and nowhere else. |
| `--rule` | The hairline the frame is drawn with. `--line-1` is a lighter rule between the rows inside a section; `--line-3` is the outline of a control, at 3.4:1. |
| `--keyline` | The one-pixel ink line round anything that floats: a popover, a sheet, a toast, a tooltip, a label on a film. |
| `--ink-*` | Text. `--ink-4` is decoration only: never text smaller than 24 px. |
| `--lumen-*` | Only the next action. On paper: the lit button's ink, the line round a pair of choices, the focus ring. On film: the ring, the ghost line, a chosen outline, a letter that is set. Never a hue and never decoration. |
| `--sure`, `--look`, `--needs`, `--by-you`, `--inferred` | Only data. Each always arrives with its line pattern, its glyph and its word, and each has an on-paper and an on-film value. |
| `--casing` | Under every stroke and label drawn on a film. |
| `--r-*` | Small: 2 to 6 px. A well is square. `--r-pill` is for what is round: a step's number, a round playback button. |
| `--lift-*`, `--glass`, `--glass-blur`, `--glow-lumen` | Kept by name, so nothing has to be renamed, and they say none: nothing has a shadow, a blur or a glow. `--glow-film` is a hairline round a film on its well. |
| `--t-*`, `--ease-*` | Every duration and curve in the app is one of these, or one of the named timings below. No literal milliseconds in a component's CSS. |

A few of the named animations have a duration or a gap of their own that is not a token: the band up a slot, an outline's refit, the ticks of the case sheet. Those are custom properties too, in `app/src/look-and-feel/named-timings.css`, so they shrink under reduced motion with the rest.

There is one breakpoint, at 1200 px wide. Below it `tokens.css` narrows the guide, shrinks the voice and hero sizes, and narrows the tool column, the ladder and the bending stack.

### One name, two grounds

`data-ground="film"` on an element gives every token inside it its on-film value. `data-ground="paper"` turns back, for a paper label laid on a film. With neither, the ground is paper.

| Token | On paper | On film |
|---|---|---|
| `--ink-1` to `--ink-4` | `#111417`, `#41474e`, `#636a72`, `#8a9198` | `#f4f6f8`, `#c2c8cf`, `#98a0a9`, `#6b737c` |
| `--line-1` to `--line-3` | `#e4e7ea`, `#cfd3d8`, `#868c93` | white at 12%, 22%, 45% |
| `--lumen` and `--on-lumen` | ink `#111417`, with white on it | white, with near-black on it |
| the focus ring | 2 px ink, with a 2 px paper gap | 2 px white, with a 2 px black gap |
| the confidence colours | the darker set, for white | the brighter set, drawn over a dark casing |

A component's CSS uses the plain names only (`--ink-1`, `--sure`) and never names a ground. The values for each ground are in `tokens.css` as `--ink-1-paper`, `--ink-1-film` and so on.

In the app the film viewer says `data-ground="film"`, so every layer and HUD inside it is drawn in on-film values with no rule of its own. `--c`, the colour of a confidence state, is worked out where `data-conf` stands, so `data-conf` goes inside the ground its mark is drawn on.

### Confidence, in full

The four states are seen on two grounds: on a film, as outlines, and on paper, in the ladder, the legend and the guide. Each has one colour for each ground, and the same line, glyph and word on both.

| State | Word | On film | On paper | Line on film | Glyph | Rung bar, on paper |
|---|---|---|---|---|---|---|
| sure | Sure | `#5cc8ff` blue, 11.2:1 | `#0b57d0` blue, 6.4:1 | solid, 2 px | filled dot | solid |
| look | Worth a look | `#ffd23f` yellow, 14.5:1 | `#a86400` amber, 4.7:1 | dashed 7 5, 2 px | half dot | dashed |
| needs | Needs you | `#f0626e` coral, 6.7:1 | `#9c1022` red, 8.4:1 | dotted 1.5 5.5, 3.2 px, tinted fill | hollow ring | dotted, a little wider |
| byyou | Set by you | `#f4f9ff` white, 19.8:1 | `#111417` ink, 18.5:1 | solid, 2.2 px | tick | solid ink |
| sacrum / inferred | Not counted / inferred | `#aab4c3` grey, 10.0:1 | `#7b828a` grey, 3.9:1 | long dash 12 6, 1.4 px | short dash | thin, long-dashed, faint |

Contrast is by the WCAG formula: on film against the black well, which is what the dark casing under every mark gives it on bright bone; on paper against white. The four states pass AA for text on both grounds. Inferred is a mark and never small text, and always carries its word, so 3:1 is its floor.

"Set by you" is a fourth state on purpose. It separates what a person confirmed from what the machine was sure of. It is the strongest neutral of its ground: white on a film, ink on paper.

**Told apart without hue.** Each set was measured with a colour-vision simulation (Machado 2009), as the CIE76 distance between the closest pair of all five:

| | Typical | Red-blind | Green-blind | Blue-blind |
|---|---|---|---|---|
| On paper | 45 (look, needs) | 27 (needs, set by you) | 25 (look, needs) | 25 (sure, inferred) |
| On film | 25 (set by you, inferred) | 23 (sure, inferred) | 26 (set by you, inferred) | 25 (set by you, inferred) |

As greys the paper set is five steps of lightness (CIE L*): set by you 6, needs 33, sure 40, look 49, inferred 54. On film the same states are 98, 60, 77, 86 and 73. The closest pair on paper for red-green colour blindness, look and needs, also differs by 16 L*, by line (dashed against dotted) and by glyph (half dot against ring). Measure the pairs again whenever a colour changes.

**A rung is a ruled row.** Its rule is the frame's hairline whatever the state. How sure is said by the bar at its edge, its glyph and its name, all on paper. Its slice of the unrolled strip is film: it wears no state, and is marked only when the rung is pointed at or chosen.

`sacrum` is the value for an outline that is drawn and not counted: C7 above T1, and any outline below L5.

In the app this table is one stylesheet, `app/src/look-and-feel/confidence.css`. It turns `data-conf` into custom properties (`--c` for the colour; a line width, dash pattern, cap, tint and border style; how strongly an outline, its casing and a rung's bar and name show; a rung's bar), which the outlines on a film, the rungs of the ladder and the chips of the legend all read. Everything that differs from one state to the next is there, so no part writes a `data-conf` rule of its own. The five glyphs are `ConfidenceGlyph`.

The dots of a needs-you outline are `1.5 5.5`, as in the table. The token `--dash-needs` says `1.5 5`; nothing reads it.

In the app two tests read the tokens: one holds the two grounds to the same names, and one works out every contrast above and the order of the greys, so a colour cannot be changed to one that fails.

---

## 6. Typography and icons

Three families, all under the SIL Open Font License 1.1, shipped as woff2 inside the app and declared with `@font-face` in `app/src/look-and-feel/fonts.css`. The files come from the font packages the app installs (`@fontsource/instrument-serif`, `@fontsource-variable/geist`, `@fontsource-variable/geist-mono`) and are served from the app's own origin. No font is ever fetched from anywhere else.

| Role | Family | Used for |
|---|---|---|
| Voice | **Instrument Serif** (Regular, Italic) | The guide's one sentence (32/36), sheet and popover titles (26 to 28), the L and R side letters (48). Nothing else. |
| Interface | **Geist** (variable, 400 to 600) | Everything read as a sentence or pressed. Body 14/21, small 12/18, micro 11/16. Card and bay headings are 11 px capitals with .09em tracking. |
| Data | **Geist Mono** | Anything measured, counted or named as a level: T1 to L5, millimetres, pixel sizes, zoom, counters, file names, key hints. Every figure is one width and the zero is slashed. |

Fallbacks are system fonts (Iowan Old Style, SF Pro, SF Mono, and their Windows and Linux peers). The mock runs on the fallbacks, so its line breaks differ a little from the app's. In the app every face is `font-display: block`, so text is first seen in its own face and never in a fallback that is then swapped away.

The shipped faces have the up and down arrows and no left or right arrow, and none of the signs printed on an Apple keyboard's Command, Shift and Option keys. A key hint draws all four arrows and those three signs as icons, and every other key as its letter or word in the data face. A character the faces lack would be drawn by whatever the system has, in another size and weight.

**Rules**

- Sentence case everywhere. The guide's sentence is one short sentence with a full stop.
- Numbers never appear without a unit.
- Text drawn on a film is Geist Mono at 11.5 px on screen whatever the zoom, with the casing stroke under it.
- Nothing in the main flow is an icon alone. Tools carry a word and a key. Undo and redo carry a tooltip that names the action.
- The weights are the same on white: 400 for reading, 500 and 600 for what is pressed. Where you are is said in weight and ink, not in a fill: the current step and a chosen rung are 600, the tool in hand 700.
- Small capitals with .09em tracking head every section, every bay and the two columns of the bench. They are `--ink-2` on paper, a step darker than a caption, because on white they are also what a hairline is read with.

**Icons.** Every icon is drawn through one component, `Icon`, by name: 20 px across, a 1.6 px line, round ends, `currentColor`. Two sets share the names.

- A set drawn for this app on a 20 px grid: pin (the pointer that drags a corner), region wand, boundary wand (two bodies and a broken gap), magnifier, ruler, on-device (a house with a tick), case file, unreadable file.
- The few icons taken from Lucide (ISC licence), through the installed `lucide-react`, which inlines them at build time: check, plus, minus, close, undo, redo, play, pause, replay, skip, and seven for key hints: the four arrows, and the Command, Shift and Option signs.

Three more families of signs are drawn the same way and are not icons to ask for by name: the four view figures (front, side, bending left, bending right), the five confidence glyphs, and the state icons of a row or a chip (ok, waiting, busy, set by you, warning, wrong). Each sign in a family is its own shape, so none is told from another by colour.

---

## 7. Motion

**Language: things arrive, cameras move, nothing bounces for fun.** Things appear by fading in, and a film by coming into focus. Films and their marks move only as one camera. Bays glide to where they hang next. Exactly one thing loops, and only on a film: the ring that marks where the next action will land. Nothing on paper loops, and nothing glows anywhere. Every duration and curve is a token.

**The attention cue.** On paper the cue is the lit button itself: the one solid ink thing on a white page needs no help, so it stands still, and so does a pair of choices. On a film a still mark can be lost among bone, so the ring keeps one slow breath: its line eases between 55% and full strength every 2.6 s. That is the whole of it.

### Named animations

| Name | What moves | Duration | Curve |
|---|---|---|---|
| `breathe` | The ring on a film: its line and its faint wash ease between rest and full strength. The only loop. The lit button and a pair of choices do not move. | 2.6 s | ease-in-out, infinite |
| `settle` | Guide lines and cards (ruled sections) arrive 8 px up with opacity. Lines stagger 40 ms, cards 60 ms. | 420 ms | `--ease-out` |
| `reading` | A grey band with a hairline at its edge travels up a slot while its file is read. | 1.1 s | linear, loops while reading |
| `lamp-on` | A hung film's opacity flickers 0, .75, .18, .6, .35, 1, as its column turns from paper to black. | 420 ms | linear keyframes |
| `develop` | With lamp-on: blur 16 px and 1.9 brightness to sharp. | 700 ms | `--ease-out` |
| `re-hang` | Bays and the lane glide between the layout's rectangles. | 700 ms | `--ease-in-out` |
| `focus-pull` | The camera travels to a vertebra, to fit, or follows its partner. | 700 ms | `--ease-in-out` |
| `chip-pop` | A fact chip changes. Scale .8 to 1. | 240 ms | `--ease-snap` |
| `lock-letter` | The L and R letters land. Scale .7 to 1. Then the flags glide to the film's corners with `re-hang`. | 240 ms | `--ease-snap` |
| `lock-in` | An outline's stroke flashes 6 px then rests. | 260 ms | `--ease-snap` |
| `warm` | Confidence arrives: stroke colour, dash and fill. | 420 ms | `--ease-out` |
| `roll` | A changed name rises 8 px with opacity, on the films and the ladder together, staggered 22 ms by distance from the edit. | 240 ms | `--ease-out` |
| `rung-glide` | Ladder rungs change top and height after a split or merge. | 420 ms | `--ease-in-out` |
| `flash` | New outlines after a split or merge: white stroke and fill fading. A new rung's slice of the strip flashes white with them. On paper a flash starts from a wash of ink. | 700 ms | `--ease-out` |
| `cut` | A white line along the gap, 6 px to 1 px, fading. | 620 ms | `--ease-out` |
| `flood` | Region wand: a soft disc grows from the click and fades (coral when trimming). Then the outline refits by tween, 280 ms. | 360 ms | `--ease-out` |
| `pin-drop` | A pinned corner's stroke 7 px to rest. Neighbours ease to their re-solved place in 280 ms. | 160 ms | `--ease-snap` |
| `ghost` | The boundary wand's line and the region wand's wall. No animation: they track the pointer. | none | none |
| `tip` | The word at the pointer fades in. | 160 ms | linear |
| `loupe` | The magnifier: scale .6 to 1 with opacity. | 240 ms | `--ease-snap` |
| `toast` | 16 px up with opacity. Leaves after 4.6 s. | 240 ms | `--ease-out` |
| `pop-in` | The promise popover: 6 px down, scale .985 to 1. | 240 ms | `--ease-out` |
| `sheet-in` | The veil and the case sheet. | 420 ms | `--ease-out` |
| `case-fold` | Manifest lines tick 170 ms apart from 200 ms. At 1.1 s the four small films collapse (700 ms, `--ease-lens`) and the case tile scales in (420 ms, `--ease-snap`, 520 ms later). | 1.1 s then 1.2 s | as listed |
| `narrate` | The live narration line opens its explanation (grid rows 0fr to 1fr). | 420 ms | `--ease-out` |
| `check-flash` | A check that turns from warning to pass flashes its row once, from a grey wash. | 700 ms | `--ease-out` |

Four of these are used by more than one area and live in `app/src/choreography/`: `breathe`, `settle`, `roll` and `flash`. `breathe` names no colour, and `flash` starts from white on a film and from a wash of ink on paper. Every other keyframe sits in the CSS of the one component that uses it.

### The automatic pass

11.8 seconds at normal speed, with half and double speed. It is one pure function of time, so pause, scrub in both directions, skip and exact replay are free. In the app the worker records what it found at each stage and the animation plays that recording at this fixed pace. It never recomputes. If the math is quicker the animation still plays unless skipped. If the math is slower the current beat holds on its last frame.

| Time | Beat | What happens |
|---|---|---|
| 0 to 0.45 s | Quiet | Both films dim to 58% brightness. The bending stack dims to 38%. |
| 0.4 to 2.4 s | **1 Evidence** | Three maps wipe top to bottom on both films, 640 ms each (`--ease-in-out`), starting at 0.4, 1.06 and 1.72 s: edges as crisp line art, ridges as soft bands, pedicle rings as small bright rings with a few decoys. A white leading line and a faint trail mark each wipe; nothing glows. Ribs are edges too, and the narration says so. |
| 2.4 to 3.7 s | **2 Centre line** | Evidence steps back to 50%. Three candidate paths fan out and one bright thread with a white head draws neck to pelvis in 1.1 s (`--ease-in-out`) on both films. The losers fade 3.1 to 3.65 s. The head fades by 3.8 s. |
| 3.35 s | The films part | Front and Side glide apart and the lane opens between them (`re-hang`). The lane is a black well, because film is about to be carried across it. |
| 3.7 to 5.4 s | **3 Unroll** | The corridor around each spine outlines itself (280 ms). From 3.98 s each spine lifts off in 8-pixel slices, top first over 400 ms, each slice flying to the lane for 980 ms on `--ease-lens`, turning upright and narrowing on a slight outward arc. By 5.36 s there are two straight strips, front and side. 5.1 to 5.42 s: a barcode wipes in beside each strip, bright for bone and dark for gaps. Evidence falls to 19%. |
| 5.4 to 9.0 s | **4 Search** | One line sweeps down both strips at constant speed (linear, 3.6 s) with a 64 px trail. A matching bar rides down each real spine, at right angles to it, joined to the lane by a dotted thread. Candidate boundaries flicker beside each barcode and fade; chosen ones stay. As the line passes each level a hairline stitch with a dot joins the two strips, which is the two films agreeing, and a thin dashed outline appears on both films. A counter on a paper label reads "10 of 17". Dashed means guess, and the narration says so. |
| 9.0 to 10.4 s | **5 Lock in** | The line rests and fades. From 9.15 s outlines turn solid white **from the pelvis up**, 36 ms apart, each with `lock-in`. One boundary that was a wrong guess slides to its final place in 300 ms, leaving a dotted ghost and the word "moved" for about a second: this is the visible "one may shift". Stitches brighten and their dots pop. The counter reads "locking", then "17 locked". From 9.8 s names roll in from both ends at once, 38 ms apart, and meet in the middle. |
| 10.4 to 11.6 s | **6 Confidence** | Colour arrives last, top to bottom, 34 ms apart (`warm`). Line patterns and glyphs change with it. The how-sure counts tick up. Barcodes, stitches and the side strip fade. At 10.4 s the lane turns from black back to paper. The front strip stays, as film, and becomes the ladder: its rungs are printed on the paper in step with the colour. Films return to full brightness. |
| 11.8 s | Settle | The lane narrows to the ladder, the bays glide to their Adjust places, the guide reads the result. |

The pass's beat times are the one exception to "every duration is a token": they are this table, held as data.

**Reduced motion.** Set by the system's own setting or by the app, as `data-motion="reduced"` on the app root. Token durations collapse to 1 to 160 ms, and every named timing to 1 ms or none. The breathing loop and the lamp flicker stop. A settling line brightens without rising. Bays and cameras land at once. The pass becomes six still pictures, the end of each beat, 750 ms apart, with no travelling light. Skip and the bar work the same. How long a toast stays is not motion and does not shorten. Nothing depends on motion to be understood.

---

## 8. Components

Grouped by the folder each lives in under `app/src/`. Names are plain on purpose.

### app-shell: the frame

| Component | Job |
|---|---|
| `AppFrame` | Three rows: top bar, room (guide and bench), status line. Owns the one breakpoint at 1200. |
| `StepRail` | Numbered steps; done steps show a tick and can be clicked to go back; grows by appending steps and folds done ones to ticks when crowded. |
| `CaseTitle` | Case name and saved state: Not saved yet, Changes not saved, Saved just now. A ring until the case is saved, then a solid dot. |
| `UndoRedo` | Two icon buttons whose tooltips name the action they would undo or redo. |
| `StatusLine` | The standing claim; pointer position in millimetres; pixel size; requests sent. |

### on-this-device: the promise

| Component | Job |
|---|---|
| `PromiseBadge` | "On this device only · 0 sent", always in the top bar. |
| `PromisePopover` | The three guards in plain words and the live request counter. |

### guide

| Component | Job |
|---|---|
| `Guide` | Eyebrow, one sentence, one paragraph, one action, up to two quiet actions. A live region, so the sentence is spoken when it changes. |
| `LitButton` | The single primary action, and the only solid ink button on the page. It stands still. Bound to Enter. While a sheet is open the sheet's button is the lit one, and the guide's stands down to an outline. |
| `ChoiceCards` | Two equal options for a question only a person may answer. No default. Ink-lined, never solid. Mirrors targets on the film. |
| `QuietActions` | At most two text buttons under the lit one. |
| `FilmsCard`, `FilmFactsCard` | The four films and what is known about each, with where each fact came from. |
| `WhatHappensCard` | Kept, Dropped, Sent anywhere: never. |
| `NarrationCard` | Six plain lines for the pass; the live one is opened; finished ones tick. |
| `HowSureCard` | The key and the tally in one: four ruled cells, each with a state's glyph, word, count and a sample of its line. |
| `ChecksCard` | Three standing checks with one sentence each: how many of 17, in order, level names. |
| `StrictReadCard` | The four lines of a case file being read, ticking in turn. |

### review

| Part | Job |
|---|---|
| The review queue | Not a widget. The ordered list of what needs a person; it drives the guide, the cameras, the tool in hand and the ring. |

### bench

| Component | Job |
|---|---|
| `Bench` | Holds the bays and the lane. Calls the one layout function. Drop target for files. |
| `benchLayout` | A pure function: step and size in, a rectangle per bay and for the lane out. Unit-tested. |
| `Bay` | One film, hung: head, viewport, camera. The head is paper; the viewport is the film's black well. The same part hosts the 3D view in Wave 2. |
| `BayHead` | Film name, two fact chips, zoom. Ruled underneath, across the bench. |
| `FactChip` | One fact about a film, set as data: a sign and a few words, with no box. Ticked when read, a person icon when set by the user. When it needs them it is boxed in amber, with a warning sign. |
| `FilmSlot` | The empty, reading and unreadable faces of a bay, on paper, with its own retry. |
| `EdgeRuler` | Millimetre ticks down the film's edge; amber `px` until the size is known. |

### film-viewer

| Component | Job |
|---|---|
| `FilmCanvas` | Draws the film's pixels on the GPU. Never resizes or mirrors the image. |
| The camera | Fit, zoom at the pointer, pan, travel to a point, follow a partner, stay inside the film. |

### marks

| Component | Job |
|---|---|
| `MarksOverlay` | One SVG over a film, in film coordinates. Styled only by data attributes, so CSS files do all the theming. |
| `VertebraMark` | Casing, outline, four pins, two pedicle rings, glyph and name. Focusable, with a spoken label such as "T3, needs you, in the shoulder shadow". |
| `Pin` | A corner. Hollow when free, filled white when pinned. Arrow keys move it by 1 px, Shift by 5. |
| `PedicleRing` | A ring at each pedicle; hollow and dashed with the words "not visible" when it could not be seen. |
| `EndplateLines` | The only marks on bending films. |
| `Landmarks` | C7, the top of the sacrum, the hip centres. |
| `HintRing` | The breathing ring that marks where the lit action will happen. White on a dark casing. |
| `ChoiceBadge` | The A and B badges on the film. |
| `InferredMark` (Wave 2) | Hollow, long-dash, always carrying the word "inferred". |

### ladder

| Component | Job |
|---|---|
| `Ladder` | The unrolled spine between the films: a ruled list on paper, with the strip, which is film, running down it. A listbox: arrows walk levels, Home and End jump. |
| `Rung` | One vertebra, as a ruled row: its rule across the ladder, its confidence bar, its slice of the strip, its glyph and name. Height follows the vertebra's real height. A chosen rung is tinted either side of the strip and its slice is framed in white. |
| `LadderWindow` | A thumb down the ladder's right edge, as on a scroll bar, showing what part of the spine the zoomed films are looking at. |

### auto-pass: the pass, as something to watch

| Component | Job |
|---|---|
| `PassPlayer` | The clock: time, playing, speed. Plays a recording of the pass; never recomputes. |
| `EvidenceLayer` | The three maps wiping down a film. |
| `CentreLine` | The winning thread and the paths it beat. |
| `LaneCanvas` | Everything that crosses between films: the ribbons, the two strips, the barcodes, the sweep, the stitches, the counter. |
| `Transport` | Watch again, play or pause, the six-part bar (a keyboard slider), speed, skip. |

### toolbox

| Component | Job |
|---|---|
| `ToolDock` | Pins, Region, Boundary, Magnify, each with icon, word and key: a ruled column down the bench's left edge, headed Tools. Help is a tooltip. |
| The pins tool, the region wand, the boundary wand | Each owns its probe (what would this click do?) and its commit. |
| `GhostLine`, `GhostWall` | What the boundary wand and region wand will do, before the click. |
| `TwoClickRuler` | Two points, a line with its pixel length, and a box for the real length. |
| `Magnifier` | A round view beside the pointer with film and marks together, ringed in white with a keyline. |
| `PointerWord` | The word at the pointer: what this click will do. |

### steps/2-size-and-facing

| Component | Job |
|---|---|
| `SideQuestion` | The two halves of the front film, the L and R flags, and the note on the lead marker. |

### building-blocks

| Component | Job |
|---|---|
| `Toast` | One line confirming what changed, with Undo. A slip of paper with a keyline. |
| `Button`, `IconButton`, `Kbd`, `Tooltip`, `Popover`, `Dialog`, `Slider` | The small pieces more than one area uses. |

### case-sheet and reopen-case

| Component | Job |
|---|---|
| `CaseSheet` | The save dialog: file name, what goes in, the local-only promise, packing, saved. Paper with a keyline, over a paper veil. |
| `Manifest` | The lines that tick. |
| `FilmFan`, `CaseTile` | Four small films that fold into one ink-lined tile. |
| `OpeningCase` | The guide while a saved case is read strictly, and when it is refused. |

### look-and-feel and choreography

| Part | Job |
|---|---|
| `Icon`, `StateIcon`, `ConfidenceGlyph`, `BrandMark`, the four view figures | Every sign the app draws. |
| `breathe`, `settle`, `roll`, `flash` | The four shared keyframes. |
| The one loop | The single animation loop every script-driven motion joins. |

### later waves

| Component | Job |
|---|---|
| `Bay3D` | A three.js canvas in a bay. It takes the lead position; Front and Side become its partners. |
| `MeasurementRow` | A number with its band ("Cobb 42°, band 37 to 47") and "within measurement error" when a change is under the stated threshold. |
| `ViewSwitch` | Realistic bones, X-ray style, outlines over the films. |

---

## 9. Wording

- The tool shows **previews with error bands**. It never says predict, prediction, optimal, safe or validated.
- Anything the app filled in is drawn hollow and carries the word **inferred**. A pedicle that could not be seen says **not visible**.
- **Not a medical device** is in the status line on every screen and in the case sheet.
- Plain words only. Not segmentation, inference, dynamic programming, pipeline. Say outline, guess, search, pass.
- Say what a click does: add, trim, split, merge, drag, pin.
- Say where a fact came from: from the file; from your two clicks on a 25 mm ball; read from the bulge of the upper back; confirmed by you.
- An error says what happened, that nothing else changed, and the one thing to do next.
- Confidence is always a word as well as a colour: Sure, Worth a look, Needs you, Set by you.

Every sentence the app says lives in `app/src/words/`, where a test reads each file for claims the app must never make.

---

## 10. Accessibility

- **Keyboard.** Enter presses the lit button on every screen. Arrows or A and B answer a choice. Escape closes the sheet or popover, or leaves the review. V, R, B and M choose tools. Cmd or Ctrl+Z and Shift undo and redo. Cmd or Ctrl+S saves. Space pauses the pass; the bar is a slider with arrow keys. 0 fits; plus and minus zoom. The whole flow, from first launch to Saved, can be driven with the keyboard and two clicks.
- **Tab order.** Top bar, guide, bench. On the bench the ladder is one stop; arrows walk levels; Tab then visits the selected vertebra's four corners only.
- **Focus.** A 2 px ring with a 2 px gap on every focusable thing: ink with a paper gap on paper, white with a black gap on a film. Where a ring would not follow the thing's shape, the part draws a focus of its own that shows as plainly (a corner pin takes a brighter, wider line; a rung shows as the selected rung). Nothing focusable is left with no sign of focus.
- **Not by colour alone.** Each confidence state has a colour, a line pattern, a glyph and a word. A fact that needs the person has a warning icon and the word. Every state icon is its own shape. Both sets of confidence colours, on paper and on film, were measured with a colour-vision simulation and as greys (section 5).
- **Contrast.** Worked out with the WCAG formula, on each ground. On paper, text is at least 5.5:1 on white and 5.0:1 on a row under the pointer; a control's outline is 3.4:1; `--ink-4` is 3.2:1 on white and 2.9:1 under the pointer, so it is for decoration and for type of 24 px and larger on white only. On a film, text is at least 7.9:1 against the well, and every mark carries a dark casing, so it keeps its contrast on bright bone.

  | Token | On paper, against white | On film, against the well |
  |---|---|---|
  | `--ink-1` | 18.5:1 | 19.4:1 |
  | `--ink-2` | 9.4:1 | 12.5:1 |
  | `--ink-3` | 5.5:1 | 7.9:1 |
  | `--ink-4` | 3.2:1 | 4.4:1 |
  | `--sure` | 6.4:1 | 11.2:1 |
  | `--look` | 4.7:1 | 14.5:1 |
  | `--needs` | 8.4:1 | 6.7:1 |
  | `--by-you` | 18.5:1 | 19.8:1 |
  | `--inferred` | 3.9:1 | 10.0:1 |

  The words on the lit button (`--on-lumen` on `--lumen`) are 18.5:1, and 13.0:1 under the pointer.
- **Spoken.** The guide is a live region. Toasts are status messages. Each vertebra and rung has a label with its name, state and reason. Icons, glyphs and figures are decoration and are hidden from assistive technology; the words beside them carry the meaning.
- **Targets.** Controls are at least 28 px. Pins have a hit radius of 12 px whatever they look like.
- **Reduced motion.** Section 7.
- **One theme.** Light only, on purpose, with every film on black. There is no dark theme and no switch, and `color-scheme` is `light`. A film is never inverted or tinted by the page round it: its well is black to the edges of its bay, so a radiograph is read against black whatever is beyond the hairline.

---

## 11. Growing into Waves 2 and 3

Nothing in the shell changes.

| Part | Wave 2 (3D, measurements) | Wave 3 (bones) |
|---|---|---|
| Rail | Appends 5 3D spine and 6 Measure. Done steps fold to ticks. | Appends 7 Bones. |
| Bench | A `Bay3D` takes the lead position. Front and Side stand beside it as its cross-checks, with the fitted outline drawn over the clicked points. | The same bay shows fitted bones. The tool column gains a view switch. |
| Ladder | Stays the index. Each rung gains a whisker for its error band. Selecting a rung turns the 3D spine to that vertebra and lights it on both films. | Each rung shows surface error in millimetres. |
| Guide | Gains a Measurements card in the slot Checks uses. Every value is a number with a band. A change under 10 degrees reads "within measurement error". | Gains an Export action in the case sheet. |
| Marks | `InferredMark` for points filled from neighbours: hollow, long-dash, with the word. | Outlines of fitted bones over the films. |
| Pass | The same player plays the joint fit as a recording. | The same player plays the refinement. |

---

## 12. Build notes

### CSS architecture

- **Plain CSS files with custom properties.** The app imports one stylesheet, `look-and-feel/base.css`, which pulls in the tokens, the named timings, the fonts, the confidence mapping and the four shared keyframes, in that order, then sets the reset, the body and the focus ring. Every component has one CSS file beside it, which that component imports. No CSS-in-JS and nothing that injects a `<style>` tag at run time: the content policy is `style-src 'self'`.
- **What the policy allows.** Setting styles through the DOM is allowed: React's `style` prop on the client, `element.style.setProperty`, the Web Animations API. What it blocks: `<style>` elements, `style="…"` attributes that arrive as markup (so never build HTML strings with style attributes, and no `dangerouslySetInnerHTML` with styles), and `setAttribute('style', …)`. SVG presentation attributes such as `stroke-width="2"` are attributes, not styles, and are allowed.
- **Where a style may be set from script.** Only geometry that is data: a bay's rectangle, a camera's matrix, a rung's top and height, a custom property such as `--p` for progress or `--i` for a place in a cascade. Colour, line pattern, font and timing are never set from script.
- **The data-attribute contract.** Components expose state as attributes and CSS does the rest. The value sets are types in `look-and-feel/data-attributes.ts`.

| Attribute | On | Values |
|---|---|---|
| `data-step` | app root | `films`, `facing`, `find`, `adjust` (later `spine3d`, `measure`, `bones`) |
| `data-motion` | app root | `full`, `reduced` |
| `data-tool` | app root | `pins`, `region`, `boundary` |
| `data-lane` | app root | `off`, `pass`, `ladder` |
| `data-slot` | bay | `empty`, `reading`, `hung`, `bad` |
| `data-conf` | vertebra, rung, legend chip | `sure`, `look`, `needs`, `byyou`, `sacrum` |
| `data-ground` | whatever holds a film and what is drawn on it; a paper label laid on a film; the lane | `film`, `paper`. Paper is the ground when nothing says otherwise. The lane is `film` from the moment the films part until colour arrives at 10.4 s, and `paper` otherwise. |
| `data-phase` | vertebra | `hidden`, `ghost`, `locked`, `named`, `coloured` |
| `data-state` | fact chip, check row, card row | `ok`, `needs`, `warn`, `bad`, `you`, `wait`, `busy` |

  The four steps are folders, `steps/1-films`, `2-size-and-facing`, `3-find` and `4-adjust`. `data-step` carries each step's short name, so the second reads `facing`.

- **One breakpoint,** at 1200 px wide. Below it the guide narrows to 312, the tool column drops its words, the rail keeps only the current step's name, and layout tokens shrink. Minimum supported width 960.
- **Nothing is glass, and nothing floats on a shadow.** No `backdrop-filter`, no `box-shadow` used as a shadow, no gradient as a surface's fill. What floats (a popover, a sheet, a toast, a tooltip, a label on a film) is paper with a one-pixel `--keyline`. A `box-shadow` is used only as a line: the focus ring, an inset ink line down the edge of the tool in hand.
- **Rules are borders, and bays touch.** `--bay-gap` and `--bench-pad` are 0. Each bay, the lane and the tool column draws its own left and top hairline, and the box that holds them is set one pixel up and to the left, so the first row's and first column's lines fall under the frame's and no line is ever doubled. Every head is `--bay-head` tall, so the rules under them join into one line across the bench.
- **The well is the bay's viewport.** It is `--well` when the bay's `data-slot` is `hung`, and paper otherwise. The film, its ruler and its marks sit inside `data-ground="film"`.
- **A tint never covers film.** A rung under the pointer or chosen is tinted with a background that is clear where the strip runs.

### How each animation is built

| Animation | Build it with |
|---|---|
| `reading`, `lamp-on`, `develop`, `chip-pop`, `lock-letter`, `lock-in`, `cut`, `flood`, `pin-drop`, `pop-in`, `check-flash` | CSS `@keyframes` in the component's CSS file, started by adding a class or by mounting. |
| `settle`, `roll`, `flash` | The shared `@keyframes` in `choreography/`, used by name. They say only where a thing starts, so it ends on its own colours and place. |
| `breathe` | The shared keyframes move one number, `--breath`, between 0 and 1, and the ring on a film turns it into how strongly its line shows. It takes the whole animation, `animation: var(--breathing)`, which is `none` under reduced motion. The keyframes name no colour, and nothing on paper takes them. |
| `warm`, `tip`, `toast`, `sheet-in`, `loupe`, `rung-glide`, `narrate`, `case-fold` | CSS transitions on the properties named, started by a class or data attribute. `narrate` transitions `grid-template-rows` from `0fr` to `1fr`. |
| `re-hang` | `benchLayout()` returns rectangles; the bay sets `left`, `top`, `width`, `height` through the DOM; a CSS transition moves it. A `snap` class turns transitions off for layouts that must land at once (resize, deep link, scrubbing while paused, reduced motion). |
| `focus-pull` | Script, on the one loop, with `--t-slow` and `--ease-in-out` read through `choreography/timing.ts`. The film is drawn on a canvas the size of its viewport and the camera reaches it as a matrix, so a film of 3000 by 7000 pixels is never an element that large. Pointer-driven changes land at once. |
| A cascade (`roll`, `settle`) | Each item's place is written as `--i`, a whole number, and CSS works out `animation-delay: calc(var(--i) * var(--stagger))`. A delay is never set from script. |
| Outline refit after an edit | A tween of the four corners on the one loop, `--t-refit` and ease-out. With reduced motion, set at once. |
| The pass | One `render(t)` that draws from a recording of the pass. Evidence: the three maps as images in a layer over the film. Centre line, outlines, names: SVG in the marks overlay, switched by `data-phase`. Everything between films: one 2D canvas over the bench that reads each film's on-screen rectangle every frame, so it stays right while bays glide. It is drawn for black: white lines on the films and the lane, a paper label for the count, and no glow. The one loop advances `t` by `dt × speed`. Scrub and skip call `render` directly. |

**Marks stay sharp at any zoom.** The overlay SVG is in film coordinates and is scaled by the camera. Strokes use `vector-effect: non-scaling-stroke`, so line widths and dash patterns are in screen pixels. Pin radius and label size are divided by the current scale when the camera changes.

### Libraries

| Allowed | For |
|---|---|
| React, TypeScript, Vite | Everything seen and clicked. |
| three.js | The 3D bay, from Wave 2. Not loaded in Wave 1. |
| zustand | App state: the case, undo history, the pass clock. |
| `lucide-react` | The few generic icons, through `Icon`. |
| The Geist, Geist Mono and Instrument Serif font packages | Type, as woff2 from the app's own origin. |

The strict read of a case file and its zip are the Rust engine's work, so the app carries no schema or zip library of its own.

| Not allowed | Why |
|---|---|
| styled-components, Emotion, any CSS-in-JS | They inject style tags, which the content policy forbids. |
| UI kits (MUI, Chakra, Mantine) and headless kits that inject styles | Same reason, and the app needs five small primitives (popover, dialog, tooltip, toast, slider) that the mock shows in full. |
| Animation libraries (Framer Motion, GSAP) | Not needed: CSS keyframes and transitions, the Web Animations API and one `requestAnimationFrame` loop cover everything here. |
| Tailwind | Tokens live in CSS files as custom properties; one system is enough. |
| Icon fonts, font CDNs, any remote asset | The app makes no network calls. |

---

## 13. Looking at the mock

Open [`mock.html`](mock.html) in a browser. It is one self-contained file: it makes no request, holds no address and shows no patient image, because every film in it is drawn by its own script.

A small magenta tab at the bottom, "Mock only: jump to a state", lists every state and toggles reduced motion and the later-wave rail. It is not part of the product.

Deep links take the same names: `mock.html#s=first-launch`, `films-being-read`, `a-film-can-t-be-read`, `four-films-up`, `left-or-right`, `no-size-two-click-ruler`, `ruler-measured`, `films-all-set`, `automatic-pass`, `pass-result`, `review-which-is-l5`, `review-split`, `review-fix-t3`, `review-confirm-t1`, `all-clear`, `free-adjusting`, `save-sheet`, `saved`, `reopen-a-case`, `case-won-t-open`, `on-this-device`. Add `&t=7400` to freeze the pass at a time in milliseconds, `&motion=reduced`, `&later=on`, `&tool=boundary`.

### What the mock fakes

- The result of the pass is scripted (17 outlines, one really two, one short of its bone, the lowest on the sacrum) so the story repeats. The evidence maps, the strips and the barcodes are really computed from the drawn films.
- The region wand and the neighbour re-solve move corners by simple rules.
- The two-click ruler snaps to the marker ball.
- Choosing, dropping, saving and opening files are simulated.
- "Requests sent: 0" is a static label in the mock and is a real counter in the app.
- The mock uses an inline style tag and script, which the app's content policy forbids. The design itself needs neither.
- The mock stands the guide's button down under an open sheet with a `:has()` rule. The app does it from state.
- The mock draws the casing of the ring, the ghost line and the centre line as a tight dark `drop-shadow`. The app draws a casing line under each, as it does under an outline.
- The mock paints the pass's lane with literal whites from script. In the app a canvas takes its colours from the tokens of the ground it is on.
- The mock moves each film with a CSS transform. Its films are drawn at 720 by 1720 and are soft above about 250% zoom. That is the mock, not the design.
- Pins in the mock can only be grabbed on their visible dot. The app gives them the 12 px hit radius.
- The two-click ruler has no keyboard path in the mock other than "Same as the front film". The app gives it one: arrow keys move a crosshair, Enter drops a point.
- The mock runs on system fonts. The app ships the three families.
