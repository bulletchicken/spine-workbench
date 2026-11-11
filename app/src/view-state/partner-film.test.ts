import { describe, expect, it } from 'vitest';
import { createPartnerFilm } from './partner-film';

describe('partner film', () => {
  it('starts with the side film beside Front', () => {
    expect(createPartnerFilm().getState().partner).toBe('side');
  });

  it('takes each bending film in turn', () => {
    const partner = createPartnerFilm();
    partner.getState().setPartner('bend-left');
    expect(partner.getState().partner).toBe('bend-left');
    partner.getState().setPartner('bend-right');
    expect(partner.getState().partner).toBe('bend-right');
  });
});
