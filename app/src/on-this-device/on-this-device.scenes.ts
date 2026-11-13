// The promise, open: the app as it first launches, with the popover that
// makes "nothing leaves this computer" checkable already showing.

import type { DevScene } from '@/app-shell/dev-scenes';
import { popoverOpen } from './popover-open';

const scenes: DevScene[] = [
  {
    name: 'on-this-device',
    enter: () => {
      popoverOpen.getState().setOpen(true);
    },
  },
];

export default scenes;
