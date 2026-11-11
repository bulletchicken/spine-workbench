// Key hints as each kind of keyboard writes them. The expected hints are
// the ones menus show: ⌘Z and ⇧⌘Z on a Mac, Ctrl+Z and Ctrl+Shift+Z on
// Windows and Linux.

import { describe, expect, it } from 'vitest';
import { keyHint } from './key-hint';

const APPLE = true;
const OTHER = false;

describe('a key hint', () => {
  it('names a key by itself the same on every keyboard', () => {
    for (const keyboard of [APPLE, OTHER]) {
      expect(keyHint(['Enter'], keyboard)).toBe('Enter');
      expect(keyHint(['Escape'], keyboard)).toBe('Esc');
      expect(keyHint([' '], keyboard)).toBe('Space');
      expect(keyHint(['Space'], keyboard)).toBe('Space');
      expect(keyHint(['ArrowLeft'], keyboard)).toBe('←');
      expect(keyHint(['ArrowRight'], keyboard)).toBe('→');
      expect(keyHint(['PageDown'], keyboard)).toBe('PageDown');
    }
  });

  it('shows a letter in capitals and a digit or sign as it is', () => {
    expect(keyHint(['v'], APPLE)).toBe('V');
    expect(keyHint(['B'], OTHER)).toBe('B');
    expect(keyHint(['0'], APPLE)).toBe('0');
    expect(keyHint(['+'], OTHER)).toBe('+');
  });

  it('shows a true minus sign for the hyphen key', () => {
    expect(keyHint(['-'], APPLE)).toBe('−');
  });

  it('writes undo and redo as an Apple keyboard does', () => {
    expect(keyHint(['Mod', 'z'], APPLE)).toBe('⌘Z');
    expect(keyHint(['Mod', 'Shift', 'z'], APPLE)).toBe('⇧⌘Z');
    expect(keyHint(['Mod', 's'], APPLE)).toBe('⌘S');
    expect(keyHint(['Alt', 'Mod', 'Shift', 'z'], APPLE)).toBe('⌥⇧⌘Z');
  });

  it('writes undo and redo as other keyboards do', () => {
    expect(keyHint(['Mod', 'z'], OTHER)).toBe('Ctrl+Z');
    expect(keyHint(['Mod', 'Shift', 'z'], OTHER)).toBe('Ctrl+Shift+Z');
    expect(keyHint(['Mod', 's'], OTHER)).toBe('Ctrl+S');
    expect(keyHint(['Shift', 'Alt', 'Mod', 'z'], OTHER)).toBe('Ctrl+Alt+Shift+Z');
  });

  it('puts held keys first whatever order they were given in', () => {
    expect(keyHint(['z', 'Shift', 'Mod'], APPLE)).toBe('⇧⌘Z');
    expect(keyHint(['z', 'Shift', 'Mod'], OTHER)).toBe('Ctrl+Shift+Z');
  });
});
