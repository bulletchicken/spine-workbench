import { StateIcon } from '@/look-and-feel/StateIcon';
import { ON_THIS_DEVICE_WORDS as WORDS } from '@/words/on-this-device';
import './PromisePopover.css';
import type { RequestsSent } from './requests-sent';

interface PromisePopoverProps {
  requests: RequestsSent;
}

/**
 * The promise, checkable: that nothing leaves this computer, the three
 * guards that keep it so in plain words, and the count of requests sent,
 * which is read from the browser. When the browser has refused an attempt
 * to reach outside, a line says how many.
 */
export function PromisePopover({ requests }: PromisePopoverProps) {
  return (
    <div className="promise">
      <h2 className="promise-title">{WORDS.title}</h2>
      <p className="promise-sub">{WORDS.sub}</p>
      <ol className="promise-guards">
        {WORDS.guards.map((guard) => (
          <li key={guard.title}>
            <StateIcon state="ok" />
            <div>
              <b>{guard.title}</b>
              <span>{guard.line}</span>
            </div>
          </li>
        ))}
      </ol>
      <p className="promise-count">
        <b>{requests.sent}</b>
        <span>{WORDS.countLine(requests.sent)}</span>
      </p>
      {requests.blocked > 0 && (
        <p className="promise-blocked">
          <StateIcon state="warn" />
          <span>{WORDS.blocked(requests.blocked)}</span>
        </p>
      )}
      <p className="promise-fine">{WORDS.fine}</p>
    </div>
  );
}
