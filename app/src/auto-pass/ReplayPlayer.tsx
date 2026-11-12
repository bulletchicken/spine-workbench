import { useEffect } from 'react';
import { useStore } from 'zustand';
import { joinLoop } from '@/choreography/one-loop';
import type { FindReplayClock } from './find-replay-clock';

export function ReplayPlayer({ clock }: { clock: FindReplayClock }) {
  const playing = useStore(clock, (state) => state.playing);
  useEffect(() => {
    if (!playing) return;
    return joinLoop((dt) => { clock.getState().advance(dt); });
  }, [clock, playing]);
  return null;
}
