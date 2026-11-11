# gallery

Every part in every state, for looking at and for comparing with the
design. Development only: `src/main.tsx` reaches it behind
`import.meta.env.DEV`, and a production build holds none of it.

```sh
pnpm dev        # then open http://localhost:5173/?gallery
```

## Adding a page

Put a file named `<Part>.gallery.tsx` beside the part. It is found by
its name; there is no list to add it to.

```tsx
import type { GalleryPage } from '@/gallery/gallery-page';
import { Button } from './Button';

const page: GalleryPage = {
  area: 'building-blocks',
  part: 'Button',
  states: [
    { name: 'Standard', show: () => <Button>Not yet</Button> },
    { name: 'Standard, disabled', show: () => <Button disabled>Not yet</Button> },
  ],
};

export default page;
```

Each state is drawn on a plate under its name. A state may use hooks: it
is drawn as a component of its own. Small states share a row; a state
whose content is wide takes a row to itself, and one that cannot shrink
to the window keeps its plate around it while the page scrolls sideways.
A part that needs surroundings (a region for a dialog's veil, room for a
popover) brings them in the page file, with a CSS file beside it if it
needs one (`Dialog.gallery.css` beside `Dialog.gallery.tsx`). A quiet
line beside a part, such as a value as it changes, wears the gallery's
own `gallery-note` class.

A page takes its data from `sample-case/` and never from the engine.

## The address

| Address | Shows |
|---|---|
| `?gallery` | the first page, with the index |
| `?gallery=building-blocks/Button` | that page: its path under `src/`, without `.gallery.tsx` |
| `&width=1024` | the page framed 1024 wide, below the one breakpoint (1440 otherwise) |
| `&motion=reduced` | with `data-motion="reduced"` |
| `&alone` | the page with no index, at the window's own size |

With the index, the page is shown in a frame that is a real window of
the chosen width, so the breakpoint and every media query answer as they
do in the app; when there is less room than that, the frame is drawn
smaller, never reflowed. The system's own reduced-motion setting always
wins over `motion`.

## Taking pictures

Open a page alone in a window of the size wanted, and find a state by
its name:

```ts
await page.setViewportSize({ width: 1024, height: 768 });
await page.goto('http://localhost:5173/?gallery=building-blocks/Dialog&alone');
const plate = page.locator('[data-gallery-state="A sheet over the bench"]');
await plate.getByRole('button', { name: 'Open the sheet' }).click();
await plate.screenshot({ path: 'dialog.png' });
```

The stage scrolls inside itself, so take a state's picture, which scrolls
it into view, and not the full page's.

A dialog holds the page's focus while it is open, so a state that shows
one starts closed, behind a button that opens it; Tab then still reaches
every other plate. A popover lets focus go, so one state may start with
its popover open.
