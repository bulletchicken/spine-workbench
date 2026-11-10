// The evidence ledger: for each feature the app offers, the published number
// behind it, the population it was measured in, and the public source it came
// from. An entry is `supported` only when someone read its source and the
// source covers the feature as built; everything else is `experimental`.

/** A published source, cited by its identifiers. No links: the app never goes online. */
export interface EvidenceSource {
  /** Authors, year, title and journal, as written in a reference list. */
  readonly citation: string;
  /** PubMed id, digits only. */
  readonly pmid: string | null;
  /** Digital object identifier, such as `10.1148/radiol.11081511`. */
  readonly doi: string | null;
}

export interface EvidenceEntry {
  /** Stable kebab-case id the app refers to, such as `transitional-anatomy-behind-confirm-l5`. */
  readonly id: string;
  /** What the app does, in the user's words. */
  readonly feature: string;
  /** The number as published, with its unit and spread. Every entry has one. */
  readonly publishedNumber: string;
  /** Who the number was measured in. */
  readonly population: string;
  readonly sources: readonly EvidenceSource[];
  /** Whether a person read the cited source and checked the number above against it. */
  readonly checked: boolean;
  /** `supported`: the published evidence covers the feature as built. Anything less is `experimental`. */
  readonly status: 'supported' | 'experimental';
}

const CARRINO_2011: EvidenceSource = {
  citation:
    'Carrino JA, Campbell PD Jr, Lin DC, Morrison WB, Schweitzer ME, Flanders AE, Eng J, Vaccaro AR. ' +
    'Effect of spinal segment variants on numbering vertebral levels at lumbar MR imaging. ' +
    'Radiology. 2011;259(1):196-202.',
  pmid: '21436097',
  doi: '10.1148/radiol.11081511',
};

const FILICE_FRANTZ_2019: EvidenceSource = {
  citation:
    'Filice RW, Frantz SK. Effectiveness of Deep Learning Algorithms to Determine Laterality in Radiographs. ' +
    'J Digit Imaging. 2019;32(4):656-664.',
  pmid: '31065828',
  doi: '10.1007/s10278-019-00226-y',
};

export const EVIDENCE_LEDGER = [
  {
    id: 'transitional-anatomy-behind-confirm-l5',
    feature:
      'Asking you to confirm which vertebra is L5 when counting from the first rib and from the sacrum disagree',
    publishedNumber:
      'Lumbosacral transitional vertebrae in 22 of 147 subjects (15.0%); an abnormal total vertebra count in 12 of 147 (8.2%); an odds ratio of 7.4 (95% CI 2.2 to 24.8) for an abnormal count with lumbosacral transitional anatomy',
    population:
      '147 subjects who had lumbar MR imaging, with spine radiography as the reference standard; n = 147',
    sources: [CARRINO_2011],
    checked: true,
    status: 'supported',
  },
  {
    id: 'side-tags-behind-confirm-left-right',
    feature:
      'Asking you to choose left or right on the front film, with no default. The marker reading is shown as evidence',
    publishedNumber:
      'Of 4,619 exams, 694 (15%) had no laterality in their DICOM data. 67 images from 23 exams had DICOM laterality that contradicted the lead marker. 237 images (1.5%) had lead markers that were missing or could not be read',
    population:
      '15,405 radiographs of nine limb body parts (no spine films) from 4,619 exams at one institution; the lead markers were the reference',
    sources: [FILICE_FRANTZ_2019],
    checked: true,
    status: 'experimental',
  },
] as const satisfies readonly EvidenceEntry[];

/** Every id in the ledger, as a type: a later screen cannot ask for an id that is not there. */
export type EvidenceId = (typeof EVIDENCE_LEDGER)[number]['id'];

export function evidenceEntry(id: EvidenceId): EvidenceEntry {
  const entry = EVIDENCE_LEDGER.find((candidate) => candidate.id === id);
  if (entry === undefined) throw new Error(`no ledger entry ${id}`);
  return entry;
}
