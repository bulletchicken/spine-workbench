// The name a saved case is offered under: spine-case-YYYY-MM-DD.spinecase,
// from the calendar date alone. The name of a dropped or chosen file is
// never kept, so nothing about the case's name comes from a file.

function twoDigits(value: number): string {
  return String(value).padStart(2, '0');
}

/** The file name for a case saved on `on`'s local calendar date. */
export function caseFileName(on: Date): string {
  const year = String(on.getFullYear()).padStart(4, '0');
  return `spine-case-${year}-${twoDigits(on.getMonth() + 1)}-${twoDigits(on.getDate())}.spinecase`;
}
