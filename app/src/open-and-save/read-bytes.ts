// Reads a file given to the app in full, when its film or case opens. Files
// are held as blobs from the drop until then, so the system can refuse the
// read by this time: the file was moved, deleted or changed after it was
// given. That is the same failure the drop refuses as `film-truncated`, so
// it is refused here with the same code, as an `EngineFailure` like every
// other failure an opening shows.

import { EngineFailure } from '@/engine-room/engine-client';

/** The whole of `blob`, or an `EngineFailure` with `film-truncated` when the system will not give it. */
export async function readBytes(blob: Blob): Promise<ArrayBuffer> {
  try {
    return await blob.arrayBuffer();
  } catch {
    throw new EngineFailure({ code: 'film-truncated', message: 'the file could not be read in full', detail: null });
  }
}
