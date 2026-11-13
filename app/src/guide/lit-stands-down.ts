// Whether the guide's lit button stands down to an outline. There is one
// solid ink button on a page: while a sheet with its own lit button is
// open, that one is the next action and the guide's steps back. The frame
// says so from its own state; the guide never looks.

import { createContext } from 'react';

export const LitStandsDown = createContext(false);
