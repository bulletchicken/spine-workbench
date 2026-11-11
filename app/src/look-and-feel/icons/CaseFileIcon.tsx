import { DrawnIcon, type IconProps } from './DrawnIcon';

/** A case file: a folder with a tick inside. */
export function CaseFileIcon({ className }: IconProps) {
  return (
    <DrawnIcon name="case-file" className={className}>
      <path d="M2.6 6a1.6 1.6 0 0 1 1.6-1.6h3.2a1 1 0 0 1 .8.4l1.1 1.5h6.5a1.6 1.6 0 0 1 1.6 1.6v6.7a1.6 1.6 0 0 1-1.6 1.6H4.2a1.6 1.6 0 0 1-1.6-1.6z" />
      <path d="M7.3 11.3l1.9 1.9 3.5-3.8" />
    </DrawnIcon>
  );
}
