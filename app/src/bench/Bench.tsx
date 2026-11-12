import { useMemo } from 'react';
import { useStore } from 'zustand';
import { useCase } from '@/case-state/case-store';
import { filmPixels } from '@/case-state/film-pixels';
import { useCurrentStep } from '@/view-state/current-step';
import { useFilmIntakeState } from '@/view-state/film-intake';
import { useFilmIntake } from './film-intake';
import { Stage, type StageFilm } from './Stage';
import { FILM_ORDER } from './stage-layout';
import { useFindOnStage } from './find-on-stage';

/** The stage stays mounted as the step changes, so each film keeps its tile. */
export default function Bench() {
  const { held, pressPlace } = useFilmIntake();
  const intake = useFilmIntakeState();
  const { slug } = useCurrentStep();
  const records = useCase((state) => state.case.films);
  const pixels = useStore(filmPixels, (state) => state.films);
  const finding = useFindOnStage();
  const films = useMemo(() => Object.fromEntries(FILM_ORDER.map((kind) => {
    const record = records.find((film) => film.kind === kind);
    const opened = pixels[kind];
    const film: StageFilm | null = record === undefined || opened === undefined ? null : {
      size: { width: record.image.width, height: record.image.height }, ...opened,
      outlines: [], names: slug === 'find' && (kind === 'front' || kind === 'side'), on: null, only: null, frame: { kind: 'whole' }, light: null, ...finding.films[kind],
    };
    return [kind, film];
  })) as Record<(typeof FILM_ORDER)[number], StageFilm | null>, [records, pixels, slug, finding.films]);
  return <Stage moment={slug === 'find' ? 'find' : 'films'} films={films} reading={intake.making ? FILM_ORDER : intake.reading} held={held} onPressPlace={pressPlace} leaving={intake.leaving} marked={finding.marked} reviewing={finding.reviewing} replay={finding.replay} {...(finding.onPoint === undefined ? {} : { onPoint: finding.onPoint })} />;
}
