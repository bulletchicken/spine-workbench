import { describe, expect, it } from 'vitest';
import type { AnswerKey } from '@/shapes';
import { createAnswerKey } from './answer-key';

describe('answer key', () => {
  it('holds nothing for a case that is not made up', () => {
    expect(createAnswerKey().getState().key).toBeNull();
  });

  it('holds a made-up spine answer key, and clears it', () => {
    const answers = createAnswerKey();
    const key = { recipe: 'default' } as unknown as AnswerKey;
    answers.getState().setKey(key);
    expect(answers.getState().key).toBe(key);
    answers.getState().setKey(null);
    expect(answers.getState().key).toBeNull();
  });
});
