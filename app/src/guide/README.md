# guide

The left column's voice: one sentence about what to do now, one lit
button that does it, and this state's cards. Everything here draws from
props. Nothing in this folder reads the case, the view state or the
engine, so every state can be shown in the gallery (`?gallery=guide/Guide`).

A step's `Screen` renders one guide, from a pure function of its state:

```tsx
<Guide spec={sizeAndFacingGuide(state)}>
  <FilmFactsCard films={facts} />
</Guide>
```

| File | Holds |
|---|---|
| `guide-spec.ts` | `GuideSpec`: what a step tells the guide to say and offer |
| `Guide.tsx` | eyebrow, sentence, paragraph, the action, quiet actions, then the cards |
| `LitButton.tsx` | the next action: the one solid ink button on a page |
| `ChoiceCards.tsx` | two equal answers to a question only a person may answer |
| `HalfFilmFigure.tsx` | a film in small with one half picked out, for a choice's `figure` |
| `QuietActions.tsx` | at most two text buttons under the action |
| `lit-stands-down.ts` | the frame's word that a sheet has the lit button for now |
| `cards/GuideCard.tsx` | the frame every card shares: a ruled section with a heading and a count |
| `cards/CardRows.tsx` | rows with the sign of their state, a title and lines |
| `cards/` | the seven cards, and `standing-check-rows.ts`, which words the review's standing checks |

## What the guide keeps

- **One sentence, one light.** A spec has at most one of `lit` and
  `choices`. With neither, nothing is lit: there is nothing to do, or the
  person is finished.
- **Enter presses the lit button**, from anywhere, once per press. A
  focused button keeps Enter for itself, so Enter on a quiet action
  presses that action. While a sheet with a lit button of its own is
  open the guide's stands down to an outline and gives Enter up; the
  frame says when, through `LitStandsDown`.
- **A choice has no default.** Its two keys answer it (`keys`, as
  `KeyboardEvent.key` names; the first is shown on the card), and
  `ChoiceCards` is the only place they are answered, so a twin on the
  film (a half of the film, an A or B badge) is a pointer target and
  never listens for keys. Enter answers nothing. A card reports the
  pointer and the keyboard's focus through `onHotChoice`, and lights from
  `hotChoice`, so the card and its twin on the film light together.
- **A sentence is spoken once.** The eyebrow, sentence and paragraph are
  one polite live region. They are drawn afresh only when they read
  differently, so a render that says the same (new functions, a card lit
  from the film) touches nothing and speaks nothing.
- **Lines arrive; what stays, stays.** New words and a new offer arrive
  with `settle`, each line a moment after the one above. A card arrives
  once, when it is first shown: give a card a `key` if it can change
  places, and it will not arrive again.

## The cards

Each takes plain props and counts for itself.

| Card | Props | Its count |
|---|---|---|
| `FilmsCard` | each film's name and the face of its slot | films up, of films |
| `FilmFactsCard` | each film's name, state and two facts | films set, of films |
| `WhatHappensCard` | none | none |
| `NarrationCard` | the lines, which is live, how many are finished | none |
| `HowSureCard` | the tally, `calibrated`, `withKey` | none |
| `ChecksCard` | rows; `standingCheckRows` words the review's | rows passing, of rows |
| `StrictReadCard` | each line and its state | lines checked, of lines |

`ChecksCard` flashes a row once when it turns from open to passing.
`HowSureCard` says "Not yet checked against real films." until it is
told the engine's confidence has been (`Confidence.calibrated`).

The cards' fixed words are in `words/guide.ts`. A sentence that belongs
to a step, a film's name, the six narration lines and the strict-read
lines come in as props from the words file of whoever shows the card.
