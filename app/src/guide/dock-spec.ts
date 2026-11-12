/** The dock keeps these scenes mounted so their words and controls can fade together. */
export type DockScene = 'add' | 'reading' | 'ready' | 'finding' | 'found' | 'review' | 'failed' | 'engine';

export interface DockAction {
  label: string;
  press: () => void;
}

export type DockWords = readonly (string | { link: string; press: () => void })[];

export type DockHelper =
  | { kind: 'words'; words: DockWords }
  | { kind: 'look'; words: string }
  | { kind: 'meter'; share: number; words: string }
  | { kind: 'count'; count: string; spoken: string; share: number; words?: string };

export interface DockSpec {
  scene: DockScene;
  sentence: string;
  helper?: DockHelper;
  quiet?: DockAction;
  primary?: DockAction & { disabled?: boolean };
}
