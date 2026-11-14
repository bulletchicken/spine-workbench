/** A local destination chosen before encoding, while the button press is still active. */
interface CaseFileHandle {
  createWritable(): Promise<{ write(data: Blob): Promise<void>; close(): Promise<void>; abort(): Promise<void> }>;
}
declare global {
  interface Window {
    showSaveFilePicker?: (options: { suggestedName: string }) => Promise<CaseFileHandle>;
  }
}

export const SAVED_CASE_NAME = 'spine-case.spinecase';
export type CaseWriteResult = 'written' | 'download';
export type CaseFileWriter = (bytes: Uint8Array) => Promise<CaseWriteResult>;
const DOWNLOAD_URL_LIFETIME_MS = 60_000;

/** The native save dialog where available; otherwise a local blob download. */
export async function chooseCaseDestination(): Promise<CaseFileWriter> {
  if (window.showSaveFilePicker !== undefined) {
    const handle = await window.showSaveFilePicker({ suggestedName: SAVED_CASE_NAME });
    return async (bytes) => {
      const file = await handle.createWritable();
      try {
        await file.write(new Blob([new Uint8Array(bytes)], { type: 'application/zip' }));
        await file.close();
        return 'written';
      } catch (error) {
        await file.abort().catch(() => undefined);
        throw error;
      }
    };
  }
  return (bytes) => {
    const url = URL.createObjectURL(new Blob([new Uint8Array(bytes)], { type: 'application/zip' }));
    const link = document.createElement('a');
    link.href = url;
    link.download = SAVED_CASE_NAME;
    document.body.append(link);
    link.click();
    link.remove();
    // Keep the bytes available while the browser starts a large download.
    setTimeout(() => { URL.revokeObjectURL(url); }, DOWNLOAD_URL_LIFETIME_MS);
    return Promise.resolve('download');
  };
}
