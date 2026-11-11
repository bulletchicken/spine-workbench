import {
  ArrowBigUp,
  ArrowDown,
  ArrowLeft,
  ArrowRight,
  ArrowUp,
  Check,
  Command,
  Minus,
  Option,
  Pause,
  Play,
  Plus,
  Redo2,
  RotateCcw,
  SkipForward,
  Undo2,
  X,
  type LucideIcon,
} from 'lucide-react';
import type { ComponentType } from 'react';
import { BoundaryWandIcon } from './BoundaryWandIcon';
import { CaseFileIcon } from './CaseFileIcon';
import type { IconProps } from './DrawnIcon';
import './icon.css';
import type { IconName, LucideIconName } from './icon-name';
import { MagnifierIcon } from './MagnifierIcon';
import { OnDeviceIcon } from './OnDeviceIcon';
import { PinIcon } from './PinIcon';
import { RegionWandIcon } from './RegionWandIcon';
import { RulerIcon } from './RulerIcon';
import { UnreadableFileIcon } from './UnreadableFileIcon';

/** A Lucide icon under this app's name for it, drawn as the drawn set is. */
function fromLucide(name: LucideIconName, Drawing: LucideIcon): ComponentType<IconProps> {
  return function LucideDrawing({ className }: IconProps) {
    return <Drawing className={className === undefined ? 'icon' : `icon ${className}`} data-icon={name} aria-hidden="true" focusable="false" />;
  };
}

const ICONS: Record<IconName, ComponentType<IconProps>> = {
  pin: PinIcon,
  'region-wand': RegionWandIcon,
  'boundary-wand': BoundaryWandIcon,
  magnifier: MagnifierIcon,
  ruler: RulerIcon,
  'on-device': OnDeviceIcon,
  'case-file': CaseFileIcon,
  'unreadable-file': UnreadableFileIcon,
  check: fromLucide('check', Check),
  plus: fromLucide('plus', Plus),
  minus: fromLucide('minus', Minus),
  close: fromLucide('close', X),
  undo: fromLucide('undo', Undo2),
  redo: fromLucide('redo', Redo2),
  play: fromLucide('play', Play),
  pause: fromLucide('pause', Pause),
  replay: fromLucide('replay', RotateCcw),
  skip: fromLucide('skip', SkipForward),
  'arrow-left': fromLucide('arrow-left', ArrowLeft),
  'arrow-right': fromLucide('arrow-right', ArrowRight),
  'arrow-up': fromLucide('arrow-up', ArrowUp),
  'arrow-down': fromLucide('arrow-down', ArrowDown),
  command: fromLucide('command', Command),
  shift: fromLucide('shift', ArrowBigUp),
  option: fromLucide('option', Option),
};

interface NamedIconProps extends IconProps {
  name: IconName;
}

/**
 * The one way an icon is drawn: by name, 20 px across with a 1.6 px line,
 * in the colour of the text around it. A part that wants another size sets
 * `--icon-size` in its own CSS.
 *
 * An icon is decoration and is hidden from assistive technology: the
 * control it sits in carries the words.
 */
export function Icon({ name, className }: NamedIconProps) {
  const Drawing = ICONS[name];
  return <Drawing className={className} />;
}
