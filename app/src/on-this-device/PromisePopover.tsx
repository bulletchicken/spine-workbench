import { ON_THIS_DEVICE_WORDS as WORDS } from '@/words/on-this-device';
import { SHELL_WORDS } from '@/words/shell';
import type { RequestsSent } from './requests-sent';

/** The browser's count, followed by the credit for the bone shapes. */
export function PromisePopover({ requests }: { requests: RequestsSent }) {
  return (
    <>
      <div>
        <p className="note-lead">{WORDS.lead}</p>
        <p className="note-count">{SHELL_WORDS.requestsSent} <span className="num">{requests.sent}</span></p>
        {requests.blocked > 0 && <p className="note-blocked">{WORDS.blocked(requests.blocked)}</p>}
      </div>
      <p className="note-credit">{WORDS.boneCredit}</p>
    </>
  );
}
