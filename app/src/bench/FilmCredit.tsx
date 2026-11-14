import { useStore } from 'zustand';
import { filmOrigins } from '@/view-state/film-origins';
import { useCurrentStep } from '@/view-state/current-step';
import { FILMS_WORDS } from '@/words/films';
import { StageCaption } from './StageCaption';

export function FilmCredit() {
  const credit = useStore(filmOrigins, (s) => s.credit);
  const { slug } = useCurrentStep();
  return credit === null || slug === 'place' ? null : <StageCaption className="film-credit" reserveRoom>{FILMS_WORDS.credit(credit)}</StageCaption>;
}
