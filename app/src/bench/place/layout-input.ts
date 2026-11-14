import type { CaseFileV1, FilmKind, LayoutFilm, LayoutInput, SpineFit } from '@/shapes';
import { renamedFor } from '@/case-state/renamed-for';

/** The fit is already accepted; placement reads marks without fitting it again. */
export function layoutInput(fit: SpineFit, c: CaseFileV1, ageYears:number|null=null): LayoutInput {
  const film = (kind: FilmKind): LayoutFilm | null => {
    const record = c.films.find((f) => f.kind === kind), marks = c.marks.find((m) => m.film === kind);
    if (record === undefined || marks === undefined) return null;
    const facing = record.geometry.imageRight ?? marks.namedFor;
    return { marks: renamedFor(marks, facing), geometry: { ...record.geometry, imageRight: facing }, size: [record.image.width, record.image.height] };
  };
  return { fit, front: film('front'), bendLeft: film('bend-left'), bendRight: film('bend-right'),
    child: { ageYears, weightKg: null, activity: null }, naming: c.marks.find((m) => m.film === 'front')?.naming.flags.includes('estimated') === true || c.marks.find((m) => m.film === 'front')?.naming.l5ConfirmedOn == null ? 'may-be-one-off' : 'as-read',
    versions: 32, seed: 7, pedicleChildShare: null };
}
