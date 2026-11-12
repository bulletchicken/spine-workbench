import { useMemo, useState } from 'react';
import type { FilmKind } from '@/shapes';
import { numberOnPage } from '@/choreography/timing';
import type { GalleryPage } from '@/gallery/gallery-page';
import { fromFilmMarks } from '@/marks/from-film-marks';
import { SAMPLE_FILMS } from '@/sample-case/films';
import { caseAt } from '@/sample-case/stages';
import { MARK_WORDS } from '@/words/marks';
import { Stage, type Held, type Moment, type StageFilm } from './Stage';
import { FILM_ORDER } from './stage-layout';
import './Stage.gallery.css';

interface PlateProps { moment: Moment; present?: readonly FilmKind[]; reading?: readonly FilmKind[]; review?: boolean; held?: Held }

function Plate({ moment: firstMoment, present = [], reading = [], review = false, held = { over: false, target: null } }: PlateProps) {
  const [inPlace, setInPlace] = useState(present);
  const [moment, setMoment] = useState(firstMoment);
  const films = useMemo(() => Object.fromEntries(FILM_ORDER.map((kind) => {
    if (!inPlace.includes(kind)) return [kind, null];
    const sample = SAMPLE_FILMS[kind];
    const found = caseAt('pass-result').case.marks.find((marks) => marks.film === kind);
    const outlines = found === undefined ? [] : fromFilmMarks(found, (vertebra) => ({ look: vertebra.level === 'T3' ? 'look' : 'found', spoken: MARK_WORDS.outline(vertebra.level, vertebra.level === 'T3') }), () => true);
    const corners = outlines.flatMap((mark) => mark.quad);
    const only = review ? outlines.find((mark) => mark.name === 'T3') ?? null : null;
    const big = kind === 'front' || kind === 'side';
    const frame: StageFilm['frame'] = moment === 'films' || corners.length === 0 ? { kind: 'whole' }
      : only !== null && big ? { kind: 'vertebra', centre: [only.quad.reduce((sum, p) => sum + p[0], 0) / 4, only.quad.reduce((sum, p) => sum + p[1], 0) / 4], names: true, over: numberOnPage('--zoom-review') }
      : { kind: 'spine', box: { left: Math.min(...corners.map((p) => p[0])), top: Math.min(...corners.map((p) => p[1])), right: Math.max(...corners.map((p) => p[0])), bottom: Math.max(...corners.map((p) => p[1])) }, names: big, most: numberOnPage(big ? '--zoom-find' : '--zoom-find-small') };
    const film: StageFilm = { size: { width: sample.facts.width, height: sample.facts.height }, pixels: sample.pixels(), thumbnail: sample.thumbnail, outlines, names: big && moment === 'find', on: null, only: only?.key ?? null, frame, light: null };
    return [kind, film];
  })) as Record<FilmKind, StageFilm | null>, [inPlace, moment, review]);
  return (
    <div data-moment={moment} data-films={reading.length > 0 ? 'landing' : inPlace.length === 4 ? 'ready' : 'empty'} data-drag={held.over || undefined} data-ready>
      <div className="stage-gallery-moments">
        <button className="quiet" onClick={() => { setMoment('films'); }}>Films</button>
        <button className="quiet" disabled={!inPlace.includes('front')} onClick={() => { setMoment('find'); }}>Find</button>
      </div>
      <main className="stage" aria-label="The films">
        <Stage moment={moment} films={films} reading={reading} held={held} leaving={false} marked={moment === 'find'} reviewing={review && moment === 'find'} replay={null}
          onPressPlace={(kind) => { setInPlace((was) => was.includes(kind) ? was : [...was, kind]); }} />
      </main>
    </div>
  );
}

const scenes: readonly { name: string; props: PlateProps }[] = [
  { name: 'Empty', props: { moment: 'films' } },
  { name: 'Files held over Front', props: { moment: 'films', held: { over: true, target: 'front' } } },
  { name: 'Reading', props: { moment: 'films', reading: ['front'] } },
  { name: 'Front only in Films', props: { moment: 'films', present: ['front'] } },
  { name: 'Four films', props: { moment: 'films', present: FILM_ORDER } },
  { name: 'Front alone', props: { moment: 'find', present: ['front'] } },
  { name: 'Find with two films', props: { moment: 'find', present: ['front', 'side'] } },
  { name: 'Find with three films', props: { moment: 'find', present: ['front', 'side', 'bend-left'] } },
  { name: 'Find with Front and a bending film', props: { moment: 'find', present: ['front', 'bend-right'] } },
  { name: 'Find with Front and both bending films', props: { moment: 'find', present: ['front', 'bend-left', 'bend-right'] } },
  { name: 'Found', props: { moment: 'find', present: FILM_ORDER } },
  { name: 'Review T3', props: { moment: 'find', present: FILM_ORDER, review: true } },
];

/** The four cameras are shared, so only one stage is mounted at a time. */
function Scenes() {
  const [chosen, setChosen] = useState(0);
  const scene = scenes[chosen];
  return (
    <div className="stage-gallery">
      <nav aria-label="Stage scenes" className="stage-gallery-scenes">
        {scenes.map((each, index) => <button key={each.name} className="quiet" aria-pressed={chosen === index} onClick={() => { setChosen(index); }}>{each.name}</button>)}
      </nav>
      {scene !== undefined && <Plate key={chosen} {...scene.props} />}
    </div>
  );
}

const page: GalleryPage = { area: 'bench', part: 'Stage', states: [
  { name: 'Films and Find: choose a scene, hang films, and travel between moments', show: () => <Scenes /> },
] };
export default page;
