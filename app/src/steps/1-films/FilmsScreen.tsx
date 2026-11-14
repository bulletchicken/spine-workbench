import { useEffect, useState } from 'react';
import { chooseFilms, takeInNextSample } from '@/bench/film-intake';
import { useSearchFilms } from '@/bench/search-films';
import { startOver } from '@/bench/start-over';
import { useCase } from '@/case-state/case-store';
import { useEngine } from '@/engine-room/use-engine';
import { sampleFilmFilesFor } from '@/sample-films/sample-films';
import { currentStep } from '@/view-state/current-step';
import { useShowInDock } from '@/view-state/dock-now';
import { filmIntake, useFilmIntakeState } from '@/view-state/film-intake';
import { passRequest } from '@/view-state/pass-request';
import { filmsDock } from './films-dock';

export function FilmsScreen() {
  const engine = useEngine();
  const records = useCase((state) => state.case.films);
  const intake = useFilmIntakeState();
  const search = useSearchFilms();
  const [sampleFilms, setSampleFilms] = useState(0);
  useEffect(() => {
    let here = true;
    void sampleFilmFilesFor(engine).then((files) => { if (here) setSampleFilms(files.length); }, () => undefined);
    return () => { here = false; };
  }, [engine]);
  useShowInDock(filmsDock({ inPlace: records.map((film) => film.kind), intake, sampleFilms, canFind: search.canFind, sideExclusion: search.sideExclusion }, {
    choose: () => { void chooseFilms(engine); },
    sample: () => { void takeInNextSample(engine); },
    startOver: () => { void startOver(engine); },
    find: () => {
      filmIntake.getState().tell(null);
      passRequest.getState().request();
      currentStep.getState().go('find');
    },
  }));
  return null;
}
