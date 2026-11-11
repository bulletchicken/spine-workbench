// Which film stands beside Front: the side film, or one of the bending
// films. Never saved with the case.

import { useStore } from 'zustand';
import { createStore, type StoreApi } from 'zustand/vanilla';
import type { FilmKind } from '@/shapes';

/** Every film but Front. */
export type PartnerFilm = Exclude<FilmKind, 'front'>;

export interface PartnerFilmState {
  partner: PartnerFilm;
  setPartner(partner: PartnerFilm): void;
}

export type PartnerFilmApi = StoreApi<PartnerFilmState>;

/** A store with the side film beside Front. The app has one, `partnerFilm`; tests make their own. */
export function createPartnerFilm(): PartnerFilmApi {
  return createStore<PartnerFilmState>()((set) => ({
    partner: 'side',
    setPartner: (partner) => {
      set({ partner });
    },
  }));
}

/** The app's partner film. */
export const partnerFilm: PartnerFilmApi = createPartnerFilm();

/** The partner film, in a component. */
export function usePartnerFilm(): PartnerFilmState {
  return useStore(partnerFilm);
}
