import { describe, expect, it } from 'vitest';
import type { ActiveTool } from '@/toolbox/tool-contract';
import { toolChanged, watchTool } from './tool-changes';

function aTool(): ActiveTool {
  return {
    pointerDown: () => undefined,
    pointerMove: () => undefined,
    pointerUp: () => undefined,
    pointerLeave: () => undefined,
    escape: () => undefined,
    view: () => ({ cursor: 'crosshair', hint: null, preview: null }),
    stop: () => undefined,
  };
}

describe('tool changes', () => {
  it('tells every viewer watching a tool, and only that tool’s, until each stops', () => {
    const tool = aTool();
    const other = aTool();
    const heard: string[] = [];
    const stopFirst = watchTool(tool, () => heard.push('first'));
    watchTool(tool, () => heard.push('second'));
    watchTool(other, () => heard.push('other'));
    toolChanged(tool);
    stopFirst();
    toolChanged(tool);
    expect(heard).toEqual(['first', 'second', 'second']);
  });

  it('tells no one once the only watcher has stopped', () => {
    const tool = aTool();
    let told = 0;
    const stop = watchTool(tool, () => {
      told += 1;
    });
    stop();
    toolChanged(tool);
    expect(told).toBe(0);
  });
});
