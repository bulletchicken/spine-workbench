import { DrawnIcon, type IconProps } from './DrawnIcon';

/** A file that cannot be read: a page with a cross on it. */
export function UnreadableFileIcon({ className }: IconProps) {
  return (
    <DrawnIcon name="unreadable-file" className={className}>
      <path d="M6 2.6h5.4l4 4V16a1.4 1.4 0 0 1-1.4 1.4H6A1.4 1.4 0 0 1 4.6 16V4A1.4 1.4 0 0 1 6 2.6z" />
      <path d="M11.4 2.6v2.8a1.2 1.2 0 0 0 1.2 1.2h2.8" />
      <path d="M8.3 10.4l3.4 3.4M11.7 10.4l-3.4 3.4" />
    </DrawnIcon>
  );
}
