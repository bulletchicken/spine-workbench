import { useSyncExternalStore } from 'react';
import { SYSTEM_ASKS_FOR_REDUCED } from './reduced-motion';

function listen(onChange: () => void): () => void {
  const asked = window.matchMedia(SYSTEM_ASKS_FOR_REDUCED);
  asked.addEventListener('change', onChange);
  return () => {
    asked.removeEventListener('change', onChange);
  };
}

function read(): boolean {
  return window.matchMedia(SYSTEM_ASKS_FOR_REDUCED).matches;
}

/** True while the system asks for reduced motion; it follows the setting as it changes. */
export function useSystemReducedMotion(): boolean {
  return useSyncExternalStore(listen, read);
}
