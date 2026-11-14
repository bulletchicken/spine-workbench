import type { AnchorProposal, BodySide, ImageRight, ImplantLayout, LayoutProposals, Level, LookCode } from '@/shapes';
import { TEMPLATE_LEVELS } from '@/shapes';
import { CANNOT_TELL_WORDS, SHARED_LOOK_WORDS, PLACE_WORDS as W, lookWords } from '@/words/place';

/** Facing names the body's side at the right of the upright front film. */
export function filmSide(side: BodySide, namedFor: ImageRight): 'left' | 'right' {
  return (side === 'left') === (namedFor === 'patient-right') ? 'left' : 'right';
}
export interface ReviewItem { id: string; label: string; levels: Level[]; sentences: string[]; anchor: number | null; details?: { summary: string; rows: string[] }[] }
function pedicleCodes(anchor: AnchorProposal): LookCode[] {
  return [...new Set(anchor.looks.map((code) => code === 'may-be-too-narrow' && anchor.pedicle.outerWidthMm === null ? 'no-table-for-this-age' : code))];
}
export function placementLooks(layout: ImplantLayout) {
  const codes = layout.anchors.map(pedicleCodes);
  const shared = (Object.keys(SHARED_LOOK_WORDS) as (keyof typeof SHARED_LOOK_WORDS)[]).filter((code) => codes.length > 0 && codes.every((list) => list.includes(code)));
  return { shared, local: codes.map((list) => list.filter((code) => !shared.some((common) => common === code))) };
}
export function pedicleSentences(anchor: AnchorProposal, shortest?: number): string[] {
  return pedicleCodes(anchor).map((code) => lookWords(code, anchor.kind, anchor.lengthMm, shortest));
}
function scenarioSentences(codes: readonly LookCode[], kind: AnchorProposal['kind'], length: number | null, shortest?: number): string[] {
  const filtered = codes.includes('no-table-for-this-age') ? codes.filter((code) => code !== 'may-be-too-narrow') : codes;
  return [...new Set(filtered)].map((code) => lookWords(code, kind, length, shortest));
}
export function reviewItems(result: LayoutProposals, namedFor: ImageRight = 'patient-left'): ReviewItem[] {
  const layout = result.layouts[0]; if (layout === undefined) return [];
  const looks = placementLooks(layout), shortest = result.choices.screwLengthsMm[0];
  const items: ReviewItem[] = layout.anchors.flatMap((anchor, i) => {
    const codes = looks.local[i] ?? [];
    return codes.length === 0 ? [] : [{ id: `anchor:${String(i)}`, label: W.pedicleName(anchor.level, filmSide(anchor.side, namedFor)), levels: [anchor.level], sentences: codes.map((code) => lookWords(code, anchor.kind, anchor.lengthMm, shortest)), anchor: i }];
  });
  const unsettled = result.cannotTell.filter((reason) => reason.code === 'ends-unsettled').flatMap((reason) => reason.levels);
  if (!result.firstIsSketch) {
    const both = unsettled.includes(layout.upper) && unsettled.includes(layout.lower);
    const pair = both ? result.pairs.find((p) => p.upper === layout.upper && p.lower === layout.lower) : undefined;
    for (const [upper, level, counts] of [[true, layout.upper, result.upperEnds], [false, layout.lower, result.lowerEnds]] as const) {
      if (!unsettled.includes(level)) continue;
      const drawn = counts.find((c) => c.level === level), other = counts.find((c) => c.level !== level);
      const sentence = drawn === undefined
        ? other === undefined ? `${W.endDrawn(upper, level)} ${CANNOT_TELL_WORDS['ends-unsettled']}` : W.endUnchosen(upper, level, other.level, other.versions, result.versions)
        : other === undefined ? W.endAlone(upper, level, drawn.versions, result.versions) : W.endWithOther(upper, level, drawn.versions, result.versions, other.level, other.versions);
      items.push({ id: upper ? 'upper' : 'lower', label: W.end(upper, level), levels: [level], sentences: [sentence, ...(pair === undefined ? [] : [W.pair(layout.upper, layout.lower, pair.versionsHolding, result.versions)])], anchor: null });
    }
  }
  items.sort((a, b) => TEMPLATE_LEVELS.standard.indexOf(a.levels[0] ?? layout.upper) - TEMPLATE_LEVELS.standard.indexOf(b.levels[0] ?? layout.upper));
  if (looks.shared.length > 0) items.unshift({ id: 'every-pedicle', label: W.everyPedicle, levels: [], sentences: looks.shared.map((code) => SHARED_LOOK_WORDS[code]), anchor: null });
  if (result.cannotTell.some((reason) => reason.code === 'names-may-be-one-off')) items.push({ id: 'names', label: W.levelNames, levels: [], sentences: [W.names], anchor: null,
    details: result.namingScenarios.filter((s) => s.shift === -1 || s.shift === 1).map((s) => ({ summary: W.namingSummary(s.shift, s.changes.length), rows: s.changes.flatMap((change) => [W.namingChange(change, filmSide(change.side, namedFor)), ...(change.looksAdded.length === 0 ? [] : [W.addedLooks(scenarioSentences(change.looksAdded, change.kindTo, change.lengthToMm, shortest))]), ...(change.looksRemoved.length === 0 ? [] : [W.removedLooks(scenarioSentences(change.looksRemoved, change.kindFrom, change.lengthFromMm, shortest))])]) })) });
  return items;
}
export function placementCounts(result: LayoutProposals) {
  const anchors = result.layouts[0]?.anchors ?? [];
  return { screws: anchors.filter((a) => a.kind === 'screw').length, hooks: anchors.filter((a) => a.kind === 'hook').length, empty: anchors.filter((a) => a.kind === 'none').length, looks: reviewItems(result).length };
}
export function countWords(result: LayoutProposals): string { const c = placementCounts(result); return W.counts(c.screws, c.hooks, c.empty); }
