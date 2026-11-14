/** The dock keeps these scenes mounted so their words and controls can fade together. */
export type DockScene = 'add' | 'reading' | 'ready' | 'finding' | 'found' | 'review' | 'failed' | 'engine' | 'building' | 'build-failed' | 'three' | 'compare' | 'place';

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

export interface DockSpec extends DockBase {
  focus?: 'primary' | 'quiet' | 'chosen-stop';
  primary?: DockAction & { disabled?: boolean };
  secondary?: DockAction;
  views?: { chosen: 'front' | 'side' | 'top' | 'back' | null; choose(name: 'front' | 'side' | 'top' | 'back'): void; back?: boolean };
  stops?: { chosen: import('@/shapes').BenchStop; choose(name: import('@/shapes').BenchStop): void };
  more?: DockAction & { expanded?: boolean; controls?: string };
  small?: string;
  control?: {label:string;chosen:string;options:readonly {id:string;label:string}[];choose:(id:string)=>void};
  actions?: readonly DockAction[];
  review?: { choices?:readonly (DockAction & {chosen:boolean;reason:string|null;busy:boolean})[]; label: string; sentences: readonly string[]; count: string; details?: readonly { summary: string; rows: readonly string[] }[] };
}
