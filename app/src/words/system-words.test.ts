import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { faultWords } from './faults';
import { FILM_NAMES } from './film-names';
import { FILMS_WORDS } from './films';
import { FIND_WORDS } from './find';
import { MARK_WORDS } from './marks';
import { ON_THIS_DEVICE_WORDS } from './on-this-device';
import { PREVIEW_NOTICE } from './preview-notice';
import { SHELL_WORDS } from './shell';
import { PLACE_EDIT_WORDS, LEVEL_RULE_NAMES, PLACE_WORDS } from './place';
import { STAGE_WORDS } from './stage';

const design = readFileSync(new URL('../../../DESIGN.md', import.meta.url), 'utf8');
const words = design.split('## 8. Words')[1]?.split('## 9.')[0] ?? '';

/**
 * Words the app says that the document does not hold yet. Each was asked
 * for by a design review after the document was last copied in. The
 * document takes it at its next copy, and then it must come off this list:
 * the last test here fails when the document has caught up.
 */
const NOT_IN_THE_DOCUMENT_YET: readonly string[] = [];

describe('the system’s words', () => {
  it('holds the Films, Find, reading, failed and engine scenes to section 8, to the letter', () => {
    const fixed = [...Object.values(FILMS_WORDS), ...Object.values(FIND_WORDS)].filter((value) => typeof value === 'string');
    const forms = [
      FILMS_WORDS.addHelp.join(''), FILMS_WORDS.goesIn('front'), FILMS_WORDS.readCount(2, 4),
      FILMS_WORDS.someIn(2), FILMS_WORDS.someIn(3), FILMS_WORDS.sampleIn(2, 6),
      FILMS_WORDS.refused(1), FILMS_WORDS.leftOut(1),
      FIND_WORDS.count(9, 17), FIND_WORDS.found(17), FIND_WORDS.found(1),
      FIND_WORDS.needALook(2), FIND_WORDS.needALook(1),
      FIND_WORDS.marked(['L5'], 4), FIND_WORDS.marked(['T1', 'L5'], 4),
      FIND_WORDS.marked(['T1', 'T5', 'L5'], 4), FIND_WORDS.marked(['T1', 'T5', 'T9', 'L1', 'L5'], 4),
      FIND_WORDS.review(1), FIND_WORDS.review(2), FIND_WORDS.needsALook('T1'), FIND_WORDS.reviewCount(1, 2),
      FIND_WORDS.searching(1), FIND_WORDS.searching(3),
      FIND_WORDS.faultHelp(faultWords('engine-load-failed').title), SHELL_WORDS.reload,
    ];
    for (const text of [...fixed, ...forms]) if (!NOT_IN_THE_DOCUMENT_YET.includes(text)) expect(words, text).toContain(text);
  });

  it('holds the bar, trust note, notice, places, films and outlines to section 8', () => {
    for (const text of [
      SHELL_WORDS.appName, SHELL_WORDS.stepsLabel, SHELL_WORDS.stepThree,
      SHELL_WORDS.stageLabel, SHELL_WORDS.dockLabel, `${SHELL_WORDS.requestsSent} 0`,
      ON_THIS_DEVICE_WORDS.badge, ON_THIS_DEVICE_WORDS.lead, ON_THIS_DEVICE_WORDS.boneCredit,
      ON_THIS_DEVICE_WORDS.blocked(1), PREVIEW_NOTICE.short, STAGE_WORDS.display,
      ...Object.values(FILM_NAMES), STAGE_WORDS.choose('front'), STAGE_WORDS.replace('front'), STAGE_WORDS.reading('front'),
      ...Object.keys(FILM_NAMES).map((kind) => STAGE_WORDS.film(kind as keyof typeof FILM_NAMES)),
      MARK_WORDS.outlinesOn('front'), MARK_WORDS.outline('T3', false), MARK_WORDS.outline('T3', true),
    ]) expect(words, text).toContain(text);
  });

  it('holds hand changes in Place to section 8',()=>{
    const fixed=Object.values(PLACE_EDIT_WORDS).flatMap((value)=>typeof value==='string'?[value]:typeof value==='object'?Object.values(value):[]);
    const forms=[PLACE_EDIT_WORDS.yours('T4','L2'),PLACE_EDIT_WORDS.rules(5,7),PLACE_EDIT_WORDS.ruleAt('End vertebra plus one','L1',19,32),PLACE_EDIT_WORDS.ruleUnread('Shoulder level'),PLACE_EDIT_WORDS.rodOption('Cobalt chrome',5.5),PLACE_EDIT_WORDS.lengthSize(25),PLACE_WORDS.stage];
    for(const text of [...fixed,...Object.values(LEVEL_RULE_NAMES),...forms])expect(words,text).toContain(text);
  });

  it('lists as waiting only words the document really lacks', () => {
    for (const text of NOT_IN_THE_DOCUMENT_YET) expect(words, text).not.toContain(text);
  });
});
