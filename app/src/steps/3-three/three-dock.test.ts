import { expect, it } from 'vitest';
import { FRONT_ONLY_CASE, MADE_UP_CASE } from '@/spine-3d/made-up-case/made-up-case';
import { threeDock } from './three-dock';
it('uses the accepted films for words, and exposes only named views', () => {
  const pair = threeDock(MADE_UP_CASE.fit, true, null, () => undefined);
  expect(pair).toMatchObject({ sentence: 'The spine in 3D.', helper: { words: ['Built from the front and side films. Drag to turn it.'] }, views: { chosen: null } });
  expect(pair.primary).toBeUndefined();
  const front = threeDock(FRONT_ONLY_CASE.fit, true, 'front', () => undefined);
  expect(front.helper).toEqual({ kind: 'words', words: ['Built from the front film. Its depth and side curve are typical values, not measured. The side film could not be used.'] });
});
