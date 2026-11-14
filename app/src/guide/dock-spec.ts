/** The dock keeps these scenes mounted so their words and controls can fade together. */
export type DockScene = 'add' | 'reading' | 'ready' | 'finding' | 'found' | 'review' | 'failed' | 'engine' | 'building' | 'build-failed' | 'three' | 'compare';

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

interface DockBase {
  scene: DockScene;
  sentence: string;
  helper?: DockHelper;
  quiet?: DockAction & { expanded?: boolean; controls?: string };
}

export type DockSpec = DockBase & (
  | { primary?: DockAction & { disabled?: boolean }; secondary?: never; views?: never }
  | { primary?: never; secondary: DockAction; views?: never }
  | { primary?: never; secondary?: never; views: { chosen: 'front' | 'side' | 'top' | null; choose(name: 'front' | 'side' | 'top'): void } }
);
