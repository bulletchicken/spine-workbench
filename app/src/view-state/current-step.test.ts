import { describe, expect, it } from 'vitest';
import { createCurrentStep } from './current-step';

describe('current step', () => {
  it('starts on the first step, films, unless told otherwise', () => {
    expect(createCurrentStep().getState().slug).toBe('films');
    expect(createCurrentStep('adjust').getState().slug).toBe('adjust');
  });

  it('moves forwards and back to any step by its slug', () => {
    const step = createCurrentStep();
    step.getState().go('find');
    expect(step.getState().slug).toBe('find');
    step.getState().go('films');
    expect(step.getState().slug).toBe('films');
  });
});
