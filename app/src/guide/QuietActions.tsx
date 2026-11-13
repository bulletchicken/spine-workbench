import { Button } from '@/building-blocks/Button';
import type { QuietAction } from './guide-spec';
import './QuietActions.css';

interface QuietActionsProps {
  /** At most two; any more are not shown. */
  actions: readonly QuietAction[];
}

/** The other things a person may do now, as plain words under the next action. */
export function QuietActions({ actions }: QuietActionsProps) {
  return (
    <div className="quiet-actions">
      {actions.slice(0, 2).map((action) => (
        <Button key={action.label} look="text" onPress={action.press}>
          {action.label}
        </Button>
      ))}
    </div>
  );
}
