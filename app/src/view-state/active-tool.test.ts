import { describe, expect, it } from 'vitest';
import { createActiveTool } from './active-tool';

const pin = { film: 'side', target: { on: 'vertebra', vertebra: 4, mark: 'supL' } } as const;

describe('active tool', () => {
  it('starts with no tool in hand, the magnifier off and nothing dragged', () => {
    const state = createActiveTool().getState();
    expect(state.tool).toBeNull();
    expect(state.magnifier).toBe(false);
    expect(state.dragging).toBeNull();
  });

  it('picks a tool up and puts it down', () => {
    const tools = createActiveTool();
    tools.getState().pick('pin');
    expect(tools.getState().tool).toBe('pin');
    tools.getState().pick(null);
    expect(tools.getState().tool).toBeNull();
  });

  it('toggles the magnifier independently of the tool', () => {
    const tools = createActiveTool();
    tools.getState().pick('ruler');
    tools.getState().setMagnifier(true);
    expect(tools.getState()).toMatchObject({ tool: 'ruler', magnifier: true });
    tools.getState().setMagnifier(false);
    expect(tools.getState().magnifier).toBe(false);
  });

  it('records a pin being dragged until it ends', () => {
    const tools = createActiveTool();
    tools.getState().pick('pin');
    tools.getState().startDrag(pin);
    expect(tools.getState().dragging).toEqual(pin);
    tools.getState().endDrag();
    expect(tools.getState().dragging).toBeNull();
  });

  it('ends a drag when another tool is picked, and keeps it when the same tool is picked again', () => {
    const tools = createActiveTool();
    tools.getState().pick('pin');
    tools.getState().startDrag(pin);
    tools.getState().pick('pin');
    expect(tools.getState().dragging).toEqual(pin);
    tools.getState().pick('region-wand');
    expect(tools.getState().dragging).toBeNull();
  });
});
