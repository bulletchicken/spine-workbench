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
  // Placement and load evidence. None has been checked against its paper.
  // Some publication titles are omitted in these short references; the source
  // registries retain their full titles. Missing identifiers stay null.
  {
    id: 'place-layout-anchor-patterns-larson-2013-density',
    feature: 'Anchor density and hooks at the upper end — density',
    publishedNumber:
      'key density, group practice: 1.06 to 2.0 implants-per-level. Published implants per fused level; includes ' +
      'anchor types beyond screws.',
    population:
      'larson-2013: Anchor-density systematic review; AIS context. Sample: 10 studies; 929 patients. ' +
      '[peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Larson AN et al. 2013. Are More Screws Better? A Systematic Review of Anchor Density and Curve Correction ' +
          'in Adolescent Idiopathic Scoliosis. Spine Deform 1(4):237-247.',
        pmid: '27927354',
        doi: '10.1016/j.jspd.2013.05.009',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-anchor-patterns-mimo-2024-density',
    feature: 'Anchor density and hooks at the upper end — density',
    publishedNumber:
      'key density, group mimo-low-max: 1.4 implants-per-level. key density, group mimo-high-min: 1.8 ' +
      'implants-per-level. Low ≤1.4; high ≥1.8 implants per fused level.',
    population:
      'mimo-2024: Single thoracic curves of 45–65°; randomised density comparison; two-year correction. Sample: ' +
      '211 patients: high-density 108; low-density 103. [peer-reviewed; abstract].',
    sources: [
      {
        citation: 'Larson AN et al. 2024. J Bone Joint Surg Am 106(3):180-189.',
        pmid: '37973031',
        doi: '10.2106/jbjs.23.00178',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-anchor-patterns-mimo-2024-two-year-correction-itt',
    feature: 'Anchor density and hooks at the upper end — two year correction itt',
    publishedNumber:
      'key two-year-correction-itt, group mimo-high: 67.6 percent. key two-year-correction-itt, group mimo-low: ' +
      '65.7 percent.',
    population:
      'mimo-2024: Single thoracic curves of 45–65°; randomised density comparison; two-year correction. Sample: ' +
      '211 patients: high-density 108; low-density 103. [peer-reviewed; abstract].',
    sources: [
      {
        citation: 'Larson AN et al. 2024. J Bone Joint Surg Am 106(3):180-189.',
        pmid: '37973031',
        doi: '10.2106/jbjs.23.00178',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-anchor-patterns-zhao-2024-hook-pjk-odds',
    feature: 'Anchor density and hooks at the upper end — hook pjk odds',
    publishedNumber:
      'key hook-pjk-odds, group hooks-at-uiv: 0.48 ratio. Association; table confidence interval preferred over ' +
      'misprinted body text.',
    population:
      'zhao-2024: AIS proximal junctional kyphosis meta-analysis. Sample: 24 studies; stated total 4,063; 718 ' +
      'with PJK and 3,885 without; patients per hook comparison not given. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Zhao J, Huang C, Liu Y, Liu D, Liao D. 2024. Systematic review and meta-analysis for the proximal ' +
          'junctional kyphosis in adolescent idiopathic scoliosis. Front Pediatr 12:1387841.',
        pmid: '39205666',
        doi: '10.3389/fped.2024.1387841',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-anchor-patterns-zhao-2024-hook-pjk-confidence-interval',
    feature: 'Anchor density and hooks at the upper end — hook pjk confidence interval',
    publishedNumber: 'key hook-pjk-confidence-interval, group hooks-at-uiv: 0.31 to 0.74 ratio.',
    population:
      'zhao-2024: AIS proximal junctional kyphosis meta-analysis. Sample: 24 studies; stated total 4,063; 718 ' +
      'with PJK and 3,885 without; patients per hook comparison not given. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Zhao J, Huang C, Liu Y, Liu D, Liao D. 2024. Systematic review and meta-analysis for the proximal ' +
          'junctional kyphosis in adolescent idiopathic scoliosis. Front Pediatr 12:1387841.',
        pmid: '39205666',
        doi: '10.3389/fped.2024.1387841',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-anchor-patterns-zhao-2024-hook-pjk-i-squared',
    feature: 'Anchor density and hooks at the upper end — hook pjk i squared',
    publishedNumber: 'key hook-pjk-i-squared, group hooks-at-uiv: 47 percent.',
    population:
      'zhao-2024: AIS proximal junctional kyphosis meta-analysis. Sample: 24 studies; stated total 4,063; 718 ' +
      'with PJK and 3,885 without; patients per hook comparison not given. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Zhao J, Huang C, Liu Y, Liu D, Liao D. 2024. Systematic review and meta-analysis for the proximal ' +
          'junctional kyphosis in adolescent idiopathic scoliosis. Front Pediatr 12:1387841.',
        pmid: '39205666',
        doi: '10.3389/fped.2024.1387841',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-anchor-patterns-cammarata-2014-junction-index-reduction',
    feature: 'Anchor density and hooks at the upper end — junction index reduction',
    publishedNumber:
      'key junction-index-reduction, group hooks-at-uiv: 26 percent. About26 percent reduction in proximal ' +
      'angle, force and moment; six adult scoliosis models, not an AIS result estimate.',
    population:
      'cammarata-2014: Adult scoliosis patient-specific multibody models. Sample: 6 patients; 576 simulations. ' +
      '[peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Cammarata M, Aubin CÉ, Wang X, Mac-Thiong JM. 2014. Biomechanical risk factors for proximal junctional ' +
          'kyphosis: a detailed numerical analysis of surgical instrumentation variables. Spine (Phila Pa 1976) ' +
          '39(8):E500-7.',
        pmid: '24480964',
        doi: '10.1097/brs.0000000000000222',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-channel-grades-akazawa-2015-grade-1',
    feature: 'Inner pedicle channel grades — grade 1',
    publishedNumber:
      'key failure-rate, group grade-1: 0.5 percent. key inner-diameter-min, group grade-1: 4 mm. Grade 1: inner ' +
      'diameter ≥4 mm (inclusive minimum).',
    population:
      'akazawa-2015: AIS; inner channel graded on CT. Sample: 55 AIS patients; 810 planned pedicles. ' +
      '[peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Akazawa T, Kotani T, Sakuma T, Minami S, Tsukamoto S, Ishige M. 2015. Evaluation of pedicle screw ' +
          'placement by pedicle channel grade in adolescent idiopathic scoliosis: should we challenge narrow ' +
          'pedicles? J Orthop Sci 20(5):818-822.',
        pmid: '26124077',
        doi: '10.1007/s00776-015-0746-0',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-channel-grades-akazawa-2015-grade-2',
    feature: 'Inner pedicle channel grades — grade 2',
    publishedNumber:
      'key failure-rate, group grade-2: 2.9 percent. key inner-diameter-min, group grade-2: 2 mm. key ' +
      'inner-diameter-max, group grade-2: 4 mm. Grade 2: inner diameter ≥2 mm and <4 mm (inclusive minimum, ' +
      'exclusive maximum).',
    population:
      'akazawa-2015: AIS; inner channel graded on CT. Sample: 55 AIS patients; 810 planned pedicles. ' +
      '[peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Akazawa T, Kotani T, Sakuma T, Minami S, Tsukamoto S, Ishige M. 2015. Evaluation of pedicle screw ' +
          'placement by pedicle channel grade in adolescent idiopathic scoliosis: should we challenge narrow ' +
          'pedicles? J Orthop Sci 20(5):818-822.',
        pmid: '26124077',
        doi: '10.1007/s00776-015-0746-0',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-channel-grades-akazawa-2015-grade-3',
    feature: 'Inner pedicle channel grades — grade 3',
    publishedNumber:
      'key failure-rate, group grade-3: 12.0 percent. key inner-diameter-min, group grade-3: 1 mm. key ' +
      'inner-diameter-max, group grade-3: 2 mm. Grade 3: inner diameter ≥1 mm and <2 mm (inclusive minimum, ' +
      'exclusive maximum).',
    population:
      'akazawa-2015: AIS; inner channel graded on CT. Sample: 55 AIS patients; 810 planned pedicles. ' +
      '[peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Akazawa T, Kotani T, Sakuma T, Minami S, Tsukamoto S, Ishige M. 2015. Evaluation of pedicle screw ' +
          'placement by pedicle channel grade in adolescent idiopathic scoliosis: should we challenge narrow ' +
          'pedicles? J Orthop Sci 20(5):818-822.',
        pmid: '26124077',
        doi: '10.1007/s00776-015-0746-0',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-channel-grades-akazawa-2015-grade-4',
    feature: 'Inner pedicle channel grades — grade 4',
    publishedNumber:
      'key failure-rate, group grade-4: 31.5 percent. key inner-diameter-max, group grade-4: 1 mm. Grade 4: ' +
      'inner diameter <1 mm (exclusive maximum), cortical channel.',
    population:
      'akazawa-2015: AIS; inner channel graded on CT. Sample: 55 AIS patients; 810 planned pedicles. ' +
      '[peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Akazawa T, Kotani T, Sakuma T, Minami S, Tsukamoto S, Ishige M. 2015. Evaluation of pedicle screw ' +
          'placement by pedicle channel grade in adolescent idiopathic scoliosis: should we challenge narrow ' +
          'pedicles? J Orthop Sci 20(5):818-822.',
        pmid: '26124077',
        doi: '10.1007/s00776-015-0746-0',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-channel-grades-akazawa-2015-all-grades',
    feature: 'Inner pedicle channel grades — all grades',
    publishedNumber:
      'key failure-rate, curveSide convex, group all-grades: 5.9 percent. key failure-rate, curveSide concave, ' +
      'group all-grades: 9.0 percent.',
    population:
      'akazawa-2015: AIS; inner channel graded on CT. Sample: 55 AIS patients; 810 planned pedicles. ' +
      '[peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Akazawa T, Kotani T, Sakuma T, Minami S, Tsukamoto S, Ishige M. 2015. Evaluation of pedicle screw ' +
          'placement by pedicle channel grade in adolescent idiopathic scoliosis: should we challenge narrow ' +
          'pedicles? J Orthop Sci 20(5):818-822.',
        pmid: '26124077',
        doi: '10.1007/s00776-015-0746-0',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-channel-grades-akazawa-2015-neutral-vertebra',
    feature: 'Inner pedicle channel grades — neutral vertebra',
    publishedNumber:
      'key failure-rate, group neutral-vertebra: 8.0 percent. Neutral vertebra group; no concave/convex ' +
      'allocation given.',
    population:
      'akazawa-2015: AIS; inner channel graded on CT. Sample: 55 AIS patients; 810 planned pedicles. ' +
      '[peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Akazawa T, Kotani T, Sakuma T, Minami S, Tsukamoto S, Ishige M. 2015. Evaluation of pedicle screw ' +
          'placement by pedicle channel grade in adolescent idiopathic scoliosis: should we challenge narrow ' +
          'pedicles? J Orthop Sci 20(5):818-822.',
        pmid: '26124077',
        doi: '10.1007/s00776-015-0746-0',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-chords-zheng-2009-pedicle-length',
    feature: 'Length along the pedicle axis — pedicle length',
    publishedNumber:
      'key pedicle-length, ageBand 5-8, group zheng-children, sex male: 28.11 to 32.26 mm. key pedicle-length, ' +
      'ageBand 5-8, group zheng-children, sex female: 27.28 to 32.29 mm. key pedicle-length, level T1, ageBand ' +
      '5-8, sex male, group zheng-children: 28.57 ± 2.18 (spread statistic not stated) mm; printed 28.57 ± 2.18. ' +
      'key pedicle-length, level T1, ageBand 5-8, sex female, group zheng-children: 27.28 ± 1.86 (spread ' +
      'statistic not stated) mm; printed 27.28 ± 1.86. key pedicle-length, level T1, ageBand 9-11, sex male, ' +
      'group zheng-children: 29.99 ± 3.71 (spread statistic not stated) mm; printed 29.99 ± 3.71. key ' +
      'pedicle-length, level T1, ageBand 9-11, sex female, group zheng-children: 29.06 ± 1.43 (spread statistic ' +
      'not stated) mm; printed 29.06 ± 1.43. key pedicle-length, level T1, ageBand 12-14, sex male, group ' +
      'zheng-children: 31.94 ± 3.1 (spread statistic not stated) mm; printed 31.94 ± 3.10. key pedicle-length, ' +
      'level T1, ageBand 12-14, sex female, group zheng-children: 31.03 ± 2.61 (spread statistic not stated) mm; ' +
      'printed 31.03 ± 2.61. key pedicle-length, level T2, ageBand 5-8, sex male, group zheng-children: 28.11 ± ' +
      '2.46 (spread statistic not stated) mm; printed 28.11 ± 2.46. key pedicle-length, level T2, ageBand 5-8, ' +
      'sex female, group zheng-children: 27.79 ± 2.49 (spread statistic not stated) mm; printed 27.79 ± 2.49. ' +
      'key pedicle-length, level T2, ageBand 9-11, sex male, group zheng-children: 30.33 ± 2.44 (spread ' +
      'statistic not stated) mm; printed 30.33 ± 2.44. key pedicle-length, level T2, ageBand 9-11, sex female, ' +
      'group zheng-children: 29.2 ± 1.63 (spread statistic not stated) mm; printed 29.20 ± 1.63. key ' +
      'pedicle-length, level T2, ageBand 12-14, sex male, group zheng-children: 32.46 ± 3.7 (spread statistic ' +
      'not stated) mm; printed 32.46 ± 3.70. key pedicle-length, level T2, ageBand 12-14, sex female, group ' +
      'zheng-children: 30.89 ± 2.43 (spread statistic not stated) mm; printed 30.89 ± 2.43. key pedicle-length, ' +
      'level T3, ageBand 5-8, sex male, group zheng-children: 28.43 ± 2.46 (spread statistic not stated) mm; ' +
      'printed 28.43 ± 2.46. key pedicle-length, level T3, ageBand 5-8, sex female, group zheng-children: 28.95 ' +
      '± 2.45 (spread statistic not stated) mm; printed 28.95 ± 2.45. key pedicle-length, level T3, ageBand ' +
      '9-11, sex male, group zheng-children: 31.81 ± 4.32 (spread statistic not stated) mm; printed 31.81 ± ' +
      '4.32. key pedicle-length, level T3, ageBand 9-11, sex female, group zheng-children: 29.31 ± 2.31 (spread ' +
      'statistic not stated) mm; printed 29.31 ± 2.31. key pedicle-length, level T3, ageBand 12-14, sex male, ' +
      'group zheng-children: 33.17 ± 3.06 (spread statistic not stated) mm; printed 33.17 ± 3.06. key ' +
      'pedicle-length, level T3, ageBand 12-14, sex female, group zheng-children: 31.79 ± 2.93 (spread statistic ' +
      'not stated) mm; printed 31.79 ± 2.93. key pedicle-length, level T4, ageBand 5-8, sex male, group ' +
      'zheng-children: 29.75 ± 2.9 (spread statistic not stated) mm; printed 29.75 ± 2.90. key pedicle-length, ' +
      'level T4, ageBand 5-8, sex female, group zheng-children: 29.04 ± 2.05 (spread statistic not stated) mm; ' +
      'printed 29.04 ± 2.05. key pedicle-length, level T4, ageBand 9-11, sex male, group zheng-children: 32.28 ± ' +
      '3.53 (spread statistic not stated) mm; printed 32.28 ± 3.53. key pedicle-length, level T4, ageBand 9-11, ' +
      'sex female, group zheng-children: 30.17 ± 1.63 (spread statistic not stated) mm; printed 30.17 ± 1.63. ' +
      'key pedicle-length, level T4, ageBand 12-14, sex male, group zheng-children: 34.1 ± 2.94 (spread ' +
      'statistic not stated) mm; printed 34.10 ± 2.94. key pedicle-length, level T4, ageBand 12-14, sex female, ' +
      'group zheng-children: 31.97 ± 2.15 (spread statistic not stated) mm; printed 31.97 ± 2.15. key ' +
      'pedicle-length, level T5, ageBand 5-8, sex male, group zheng-children: 28.91 ± 1.98 (spread statistic not ' +
      'stated) mm; printed 28.91 ± 1.98. key pedicle-length, level T5, ageBand 5-8, sex female, group ' +
      'zheng-children: 29.41 ± 2.25 (spread statistic not stated) mm; printed 29.41 ± 2.25. key pedicle-length, ' +
      'level T5, ageBand 9-11, sex male, group zheng-children: 31.53 ± 3.81 (spread statistic not stated) mm; ' +
      'printed 31.53 ± 3.81. key pedicle-length, level T5, ageBand 9-11, sex female, group zheng-children: 30.86 ' +
      '± 2.6 (spread statistic not stated) mm; printed 30.86 ± 2.60. key pedicle-length, level T5, ageBand ' +
      '12-14, sex male, group zheng-children: 34.64 ± 2.61 (spread statistic not stated) mm; printed 34.64 ± ' +
      '2.61. key pedicle-length, level T5, ageBand 12-14, sex female, group zheng-children: 32.37 ± 2.98 (spread ' +
      'statistic not stated) mm; printed 32.37 ± 2.98. key pedicle-length, level T6, ageBand 5-8, sex male, ' +
      'group zheng-children: 29.75 ± 2.38 (spread statistic not stated) mm; printed 29.75 ± 2.38. key ' +
      'pedicle-length, level T6, ageBand 5-8, sex female, group zheng-children: 29.34 ± 1.69 (spread statistic ' +
      'not stated) mm; printed 29.34 ± 1.69. key pedicle-length, level T6, ageBand 9-11, sex male, group ' +
      'zheng-children: 32.83 ± 3.18 (spread statistic not stated) mm; printed 32.83 ± 3.18. key pedicle-length, ' +
      'level T6, ageBand 9-11, sex female, group zheng-children: 31.86 ± 3.19 (spread statistic not stated) mm; ' +
      'printed 31.86 ± 3.19. key pedicle-length, level T6, ageBand 12-14, sex male, group zheng-children: 34.61 ' +
      '± 2.96 (spread statistic not stated) mm; printed 34.61 ± 2.96. key pedicle-length, level T6, ageBand ' +
      '12-14, sex female, group zheng-children: 33.35 ± 3.53 (spread statistic not stated) mm; printed 33.35 ± ' +
      '3.53. key pedicle-length, level T7, ageBand 5-8, sex male, group zheng-children: 30.15 ± 3.15 (spread ' +
      'statistic not stated) mm; printed 30.15 ± 3.15. key pedicle-length, level T7, ageBand 5-8, sex female, ' +
      'group zheng-children: 30.17 ± 2.39 (spread statistic not stated) mm; printed 30.17 ± 2.39. key ' +
      'pedicle-length, level T7, ageBand 9-11, sex male, group zheng-children: 33.64 ± 3.51 (spread statistic ' +
      'not stated) mm; printed 33.64 ± 3.51. key pedicle-length, level T7, ageBand 9-11, sex female, group ' +
      'zheng-children: 32.4 ± 2.99 (spread statistic not stated) mm; printed 32.40 ± 2.99. key pedicle-length, ' +
      'level T7, ageBand 12-14, sex male, group zheng-children: 35.79 ± 3.78 (spread statistic not stated) mm; ' +
      'printed 35.79 ± 3.78. key pedicle-length, level T7, ageBand 12-14, sex female, group zheng-children: ' +
      '33.99 ± 3.32 (spread statistic not stated) mm; printed 33.99 ± 3.32. key pedicle-length, level T8, ' +
      'ageBand 5-8, sex male, group zheng-children: 30.15 ± 2.82 (spread statistic not stated) mm; printed 30.15 ' +
      '± 2.82. key pedicle-length, level T8, ageBand 5-8, sex female, group zheng-children: 30.81 ± 2.53 (spread ' +
      'statistic not stated) mm; printed 30.81 ± 2.53. key pedicle-length, level T8, ageBand 9-11, sex male, ' +
      'group zheng-children: 34.21 ± 3.49 (spread statistic not stated) mm; printed 34.21 ± 3.49. key ' +
      'pedicle-length, level T8, ageBand 9-11, sex female, group zheng-children: 31.98 ± 2.55 (spread statistic ' +
      'not stated) mm; printed 31.98 ± 2.55. key pedicle-length, level T8, ageBand 12-14, sex male, group ' +
      'zheng-children: 37.86 ± 3.58 (spread statistic not stated) mm; printed 37.86 ± 3.58. key pedicle-length, ' +
      'level T8, ageBand 12-14, sex female, group zheng-children: 34.55 ± 3.66 (spread statistic not stated) mm; ' +
      'printed 34.55 ± 3.66. key pedicle-length, level T9, ageBand 5-8, sex male, group zheng-children: 31.37 ± ' +
      '3.08 (spread statistic not stated) mm; printed 31.37 ± 3.08. key pedicle-length, level T9, ageBand 5-8, ' +
      'sex female, group zheng-children: 31.36 ± 2.49 (spread statistic not stated) mm; printed 31.36 ± 2.49. ' +
      'key pedicle-length, level T9, ageBand 9-11, sex male, group zheng-children: 35.02 ± 3.64 (spread ' +
      'statistic not stated) mm; printed 35.02 ± 3.64. key pedicle-length, level T9, ageBand 9-11, sex female, ' +
      'group zheng-children: 33.54 ± 3.15 (spread statistic not stated) mm; printed 33.54 ± 3.15. key ' +
      'pedicle-length, level T9, ageBand 12-14, sex male, group zheng-children: 37.97 ± 4.4 (spread statistic ' +
      'not stated) mm; printed 37.97 ± 4.40. key pedicle-length, level T9, ageBand 12-14, sex female, group ' +
      'zheng-children: 35.36 ± 3.07 (spread statistic not stated) mm; printed 35.36 ± 3.07. key pedicle-length, ' +
      'level T10, ageBand 5-8, sex male, group zheng-children: 32.37 ± 3.13 (spread statistic not stated) mm; ' +
      'printed 32.37 ± 3.13. key pedicle-length, level T10, ageBand 5-8, sex female, group zheng-children: 31.83 ' +
      '± 2.8 (spread statistic not stated) mm; printed 31.83 ± 2.80. key pedicle-length, level T10, ageBand ' +
      '9-11, sex male, group zheng-children: 35.44 ± 3.53 (spread statistic not stated) mm; printed 35.44 ± ' +
      '3.53. key pedicle-length, level T10, ageBand 9-11, sex female, group zheng-children: 33.69 ± 2.49 (spread ' +
      'statistic not stated) mm; printed 33.69 ± 2.49. key pedicle-length, level T10, ageBand 12-14, sex male, ' +
      'group zheng-children: 39.22 ± 4.04 (spread statistic not stated) mm; printed 39.22 ± 4.04. key ' +
      'pedicle-length, level T10, ageBand 12-14, sex female, group zheng-children: 34.7 ± 4.71 (spread statistic ' +
      'not stated) mm; printed 34.70 ± 4.71. key pedicle-length, level T11, ageBand 5-8, sex male, group ' +
      'zheng-children: 32.07 ± 2.8 (spread statistic not stated) mm; printed 32.07 ± 2.80. key pedicle-length, ' +
      'level T11, ageBand 5-8, sex female, group zheng-children: 31.78 ± 3.2 (spread statistic not stated) mm; ' +
      'printed 31.78 ± 3.20. key pedicle-length, level T11, ageBand 9-11, sex male, group zheng-children: 34.95 ' +
      '± 4.87 (spread statistic not stated) mm; printed 34.95 ± 4.87. key pedicle-length, level T11, ageBand ' +
      '9-11, sex female, group zheng-children: 33.24 ± 3.2 (spread statistic not stated) mm; printed 33.24 ± ' +
      '3.20. key pedicle-length, level T11, ageBand 12-14, sex male, group zheng-children: 38.08 ± 3.55 (spread ' +
      'statistic not stated) mm; printed 38.08 ± 3.55. key pedicle-length, level T11, ageBand 12-14, sex female, ' +
      'group zheng-children: 36.22 ± 4.75 (spread statistic not stated) mm; printed 36.22 ± 4.75. key ' +
      'pedicle-length, level T12, ageBand 5-8, sex male, group zheng-children: 32.26 ± 2.89 (spread statistic ' +
      'not stated) mm; printed 32.26 ± 2.89. key pedicle-length, level T12, ageBand 5-8, sex female, group ' +
      'zheng-children: 32.29 ± 3.62 (spread statistic not stated) mm; printed 32.29 ± 3.62. key pedicle-length, ' +
      'level T12, ageBand 9-11, sex male, group zheng-children: 34.91 ± 5.27 (spread statistic not stated) mm; ' +
      'printed 34.91 ± 5.27. key pedicle-length, level T12, ageBand 9-11, sex female, group zheng-children: ' +
      '34.01 ± 3.58 (spread statistic not stated) mm; printed 34.01 ± 3.58. key pedicle-length, level T12, ' +
      'ageBand 12-14, sex male, group zheng-children: 39.42 ± 4.12 (spread statistic not stated) mm; printed ' +
      '39.42 ± 4.12. key pedicle-length, level T12, ageBand 12-14, sex female, group zheng-children: 37.71 ± ' +
      '5.65 (spread statistic not stated) mm; printed 37.71 ± 5.65. Printed pooled length range for ages 5–8, ' +
      'with sex retained; no individual level is assigned. The complete Table 3 cells are retained separately; ' +
      'the prose interval is not a substitute for their minima and maxima. Table 3 defines length from posterior ' +
      'cortex to anterior cortex of the vertebra; Figure 1 instead says posterior cortex of the pedicle to ' +
      'posterior longitudinal ligament along the pedicle axis. Full text. The paper does not state what ± ' +
      'denotes; reading it as a standard deviation is this project\'s own inference. Numbers of children per age ' +
      'band and sex are not stated.',
    population:
      'zheng-2009: Chinese children aged 5–14 from a hospital registry, stratified random sampling with thoracic ' +
      'abnormalities excluded; not scoliosis-specific; 2 mm axial CT. Sample: 120 children aged 5–14; number per ' +
      'age band and sex is not stated. [peer-reviewed; full-text].',
    sources: [
      {
        citation: 'Zheng C, Huang Q, Hu Y, Wang X, Chen W. 2009. Int Orthop 33(6):1663-1668.',
        pmid: '18956181',
        doi: '10.1007/s00264-008-0675-z',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-chords-pluemvitayaporn-2025-trajectory-length',
    feature: 'Length along the pedicle axis — trajectory length',
    publishedNumber:
      'key trajectory-length, group pluemvitayaporn-t1, level T1: 28.4 to 28.6 mm. key trajectory-length, group ' +
      'pluemvitayaporn-t12, level T12: 45.5 mm. key trajectory-length, group pluemvitayaporn-l1-l5: 46 to 51 mm. ' +
      'T1 range as printed; side allocation and sex not supplied. Lenke 5 series, n = 112. About 45.5 mm at T12; ' +
      'approximate, with side allocation and sex not supplied. Lenke 5 series, n = 112. One pooled L1–L5 range, ' +
      'not individual-level or side-specific cells; Lenke 5 series, n = 112. P4 repeats the same range in a ' +
      'mixed-series summary citing Balikci, Kuraishi, Upendra and Pluemvitayaporn.',
    population:
      'pluemvitayaporn-2025: Thai patients with Lenke 5 curves; mean age 14.7 (12–22), mean Cobb 48° (36–84°). ' +
      'Sample: 112 patients; 3,808 pedicles. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Pluemvitayaporn T, Kaewmano T, Kunakornsawat S, Surapuchong S, Ratanakoosakul W, Tiracharnvut K, ' +
          'Pruttikul P, Piyasakulkaew C, Kittithamvongs P. 2025. Pedicle Morphology Analysis in Adolescents with ' +
          'Lenke Type 5 Idiopathic Scoliosis in Thai Population. Asian J Neurosurg 20(1):132-137.',
        pmid: '40041581',
        doi: '10.1055/s-0044-1800812',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-chords-pluemvitayaporn-2025-trajectory-side-difference',
    feature: 'Length along the pedicle axis — trajectory side difference',
    publishedNumber:
      'key trajectory-side-difference, group pluemvitayaporn-l2-l4: 2.1 to 3.2 mm. Concave trajectories are ' +
      'longer than convex trajectories by 2.1–3.2 mm at L2–L4. Regional concave-minus-convex difference, not an ' +
      'absolute trajectory; n = 112.',
    population:
      'pluemvitayaporn-2025: Thai patients with Lenke 5 curves; mean age 14.7 (12–22), mean Cobb 48° (36–84°). ' +
      'Sample: 112 patients; 3,808 pedicles. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Pluemvitayaporn T, Kaewmano T, Kunakornsawat S, Surapuchong S, Ratanakoosakul W, Tiracharnvut K, ' +
          'Pruttikul P, Piyasakulkaew C, Kittithamvongs P. 2025. Pedicle Morphology Analysis in Adolescents with ' +
          'Lenke Type 5 Idiopathic Scoliosis in Thai Population. Asian J Neurosurg 20(1):132-137.',
        pmid: '40041581',
        doi: '10.1055/s-0044-1800812',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-chords-upendra-2010-chord',
    feature: 'Length along the pedicle axis — chord',
    publishedNumber:
      'key chord, level T1, curveSide convex, group upendra-ais: 29.75 ± 2.31 SD mm (range 25.1 to 32.3); n = ' +
      '12; printed 29.75. key chord, level T1, curveSide concave, group upendra-ais: 27.67 ± 2.89 SD mm (range ' +
      '25.1 to 32.6); n = 12; printed 27.67. key chord, level T2, curveSide convex, group upendra-ais: 30.5 ± ' +
      '2.38 SD mm (range 27.2 to 32.3); n = 10; printed 30.5. key chord, level T2, curveSide concave, group ' +
      'upendra-ais: 32.83 ± 3.55 SD mm (range 28.1 to 37.3); n = 10; printed 32.83. key chord, level T3, ' +
      'curveSide convex, group upendra-ais: 32.8 ± 6.59 SD mm (range 18.1 to 41.3); n = 13; printed 32.8. key ' +
      'chord, level T3, curveSide concave, group upendra-ais: 37.33 ± 3.89 SD mm (range 32.1 to 40.5); n = 13; ' +
      'printed 37.33. key chord, level T4, curveSide convex, group upendra-ais: 34.83 ± 2.11 SD mm (range 30.1 ' +
      'to 37.5); n = 14; printed 34.83. key chord, level T4, curveSide concave, group upendra-ais: 35.75 ± 4.12 ' +
      'SD mm (range 30.4 to 40.5); n = 14; printed 35.75. key chord, level T5, curveSide convex, group ' +
      'upendra-ais: 35.33 ± 1.96 SD mm (range 32.2 to 38.6); n = 9; printed 35.33. key chord, level T5, ' +
      'curveSide concave, group upendra-ais: 37.0 ± 4.22 SD mm (range 32.1 to 43.1); n = 9; printed 37.0. key ' +
      'chord, level T6, curveSide convex, group upendra-ais: 38.5 ± 4.66 SD mm (range 30.8 to 43.2); n = 11; ' +
      'printed 38.5. key chord, level T6, curveSide concave, group upendra-ais: 39.1 ± 3.11 SD mm (range 37.1 to ' +
      '43.2); n = 11; printed 39.1. key chord, level T7, curveSide convex, group upendra-ais: 41.5 ± 1.95 SD mm ' +
      '(range 38.2 to 43.1); n = 9; printed 41.5. key chord, level T7, curveSide concave, group upendra-ais: ' +
      '41.8 ± 2.12 SD mm (range 39.2 to 43.8); n = 9; printed 41.8. key chord, level T8, curveSide convex, group ' +
      'upendra-ais: 40.2 ± 1.86 SD mm (range 38.5 to 41.6); n = 14; printed 40.2. key chord, level T8, curveSide ' +
      'concave, group upendra-ais: 42.1 ± 1.68 SD mm (range 40.0 to 44.8); n = 14; printed 42.1. key chord, ' +
      'level T9, curveSide convex, group upendra-ais: 43.5 ± 4.96 SD mm (range 37.1 to 53.2); n = 12; printed ' +
      '43.5. key chord, level T9, curveSide concave, group upendra-ais: 44.2 ± 4.97 SD mm (range 35.4 to 46.5); ' +
      'n = 12; printed 44.2. key chord, level T10, curveSide convex, group upendra-ais: 33.35 ± 4.22 SD mm ' +
      '(range 28.4 to 41.2); n = 15; printed 33.35; doubtful. key chord, level T10, curveSide concave, group ' +
      'upendra-ais: 41.5 ± 3.12 SD mm (range 38.1 to 46.8); n = 15; printed 41.5. key chord, level T11, ' +
      'curveSide convex, group upendra-ais: 37.5 ± 1.69 SD mm (range 35.1 to 42.6); n = 16; printed 37.5. key ' +
      'chord, level T11, curveSide concave, group upendra-ais: 39.5 ± 4.2 SD mm (range 35.1 to 46.1); n = 16; ' +
      'printed 39.5. key chord, level T12, curveSide convex, group upendra-ais: 40.27 ± 5.86 SD mm (range 32.5 ' +
      'to 47.1); n = 19; printed 40.27. key chord, level T12, curveSide concave, group upendra-ais: 42.2 ± 5.66 ' +
      'SD mm (range 28.9 to 50.4); n = 19; printed 42.2. key chord, level L1, curveSide convex, group ' +
      'upendra-ais: 41.46 ± 4.39 SD mm (range 35.4 to 52.6); n = 18; printed 41.46. key chord, level L1, ' +
      'curveSide concave, group upendra-ais: 46.8 ± 5.76 SD mm (range 36.1 to 55.7); n = 18; printed 46.8. key ' +
      'chord, level L2, curveSide convex, group upendra-ais: 47.37 ± 4.96 SD mm (range 32.1 to 54.8); n = 13; ' +
      'printed 47.37. key chord, level L2, curveSide concave, group upendra-ais: 49.86 ± 4.11 SD mm (range 43.2 ' +
      'to 54.8); n = 13; printed 49.86. key chord, level L3, curveSide convex, group upendra-ais: 48.56 ± 3.64 ' +
      'SD mm (range 42.5 to 52.9); n = 11; printed 48.56. key chord, level L3, curveSide concave, group ' +
      'upendra-ais: 51.2 ± 3.21 SD mm (range 45.1 to 55.4); n = 11; printed 51.2. key chord, level L4, curveSide ' +
      'convex, group upendra-ais: 46.2 ± 2.14 SD mm (range 44.5 to 47.6); n = 9; printed 46.2. key chord, level ' +
      'L4, curveSide concave, group upendra-ais: 47.65 ± 2.11 SD mm (range 45.1 to 50.9); n = 9; printed 47.65. ' +
      'key chord, level L5, curveSide convex, group upendra-ais: 47.0 ± 2.08 SD mm (range 45.2 to 50.8); n = 8; ' +
      'printed 47.0. key chord, level L5, curveSide concave, group upendra-ais: 48.2 ± 2.89 SD mm (range 45.1 to ' +
      '50.8); n = 8; printed 48.2. Printed Table 3 mean, SD, per-cell pedicle count and range; no rounding or ' +
      'interpolation. [own inference] Convex is read as right and concave as left because all 24 patients had ' +
      'right thoracic/thoracolumbar curves; the paper does not explicitly equate convex with right.',
    population:
      'upendra-2010: AIS; 14 girls and 10 boys; all right thoracic or thoracolumbar curves (King II 12, III 7, ' +
      'IV 5); ages 12.3–18.3; Cobb 58–98°; 3 mm CT. Sample: 24 patients; width n 213 convex/223 concave; angle n ' +
      '217 convex/216 concave; chord n 213 on each side; per-cell pedicle counts retained. [peer-reviewed; ' +
      'full-text].',
    sources: [
      {
        citation:
          'Upendra B, Meena D, Kandwal P, Ahmed A, Chowdhury B, Jayaswal A. 2010. Pedicle morphometry in patients ' +
          'with adolescent idiopathic scoliosis. Indian J Orthop 44(2):169-176.',
        pmid: '20419004',
        doi: '10.4103/0019-5413.62084',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-concave-ratios-sato-2022-outer-height-ratio',
    feature: 'Pedicle proportions beside the curve apex and neutral vertebra — outer height ratio',
    publishedNumber:
      'key outer-height-ratio, group apex-one: 0.86 ± 0.52 (spread statistic not stated) ratio (range 0.75 to ' +
      '0.97); printed 0.86 ± 0.52 (0.75–0.97); retained only, excluded from use; probable misprint. key ' +
      'outer-height-ratio, group neutral-one: 0.97 ± 0.06 (spread statistic not stated) ratio (range 0.82 to ' +
      '1.1); printed 0.97 ± 0.06 (0.82–1.10). Published left/right ratio; left is concave and right is convex. ' +
      'The table prints ± without defining its statistic. Cohort: 57 patients, 49 female and 8 male; Lenke 1 ' +
      '(25) or Lenke 2 (32), 684 pedicles. Enrolled ages 12–20, Table 1 age 16.2 ± 2.4; 10–20 is the abstract ' +
      'inclusion range. The paper states right is always convex and left always concave. Per-cell observation ' +
      'counts are not supplied.',
    population:
      'sato-2022: Japanese AIS, enrolled ages 12–20 (inclusion range 10–20), mean age 16.2; 49 girls and 8 boys; ' +
      'Lenke 1/2 right main thoracic curves; 0.5 mm CT. Right is always convex and left always concave. Sample: ' +
      '57 adolescents; 684 pedicles; 15 patients in the 26.3 percent group. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Sato T, Nojiri H, Okuda T, Miyagawa K, Kobayashi N, Takahashi R, Shimura A, Tamagawa S, Ohara Y, Hara T, ' +
          'Ishijima M. 2022. Three-dimensional morphological analysis of the thoracic pedicle and related ' +
          'radiographic factors in adolescent idiopathic scoliosis. BMC Musculoskelet Disord 23(1):847.',
        pmid: '36068555',
        doi: '10.1186/s12891-022-05799-4',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-concave-ratios-sato-2022-p-value-max',
    feature: 'Pedicle proportions beside the curve apex and neutral vertebra — p value max',
    publishedNumber:
      'key p-value-max, group sato-outer-height-ratio-apex-vs-neutral: 0.001 ratio; printed < 0.001*. key ' +
      'p-value-max, group sato-concave-convex-ratio-apex-vs-neutral: 0.001 ratio; printed < 0.001*. key ' +
      'p-value-max, group sato-inner-height-ratio-apex-vs-neutral: 0.001 ratio; printed < 0.001*. key ' +
      'p-value-max, group sato-inner-width-ratio-apex-vs-neutral: 0.001 ratio; printed < 0.001*. Comparison of ' +
      'APEX ±1 with NV ±1 for this ratio; p is strictly less than 0.001 as printed. Cohort: 57 patients, 49 ' +
      'female and 8 male; Lenke 1 (25) or Lenke 2 (32), 684 pedicles. Enrolled ages 12–20, Table 1 age 16.2 ± ' +
      '2.4; 10–20 is the abstract inclusion range. The paper states right is always convex and left always ' +
      'concave. Per-cell observation counts are not supplied.',
    population:
      'sato-2022: Japanese AIS, enrolled ages 12–20 (inclusion range 10–20), mean age 16.2; 49 girls and 8 boys; ' +
      'Lenke 1/2 right main thoracic curves; 0.5 mm CT. Right is always convex and left always concave. Sample: ' +
      '57 adolescents; 684 pedicles; 15 patients in the 26.3 percent group. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Sato T, Nojiri H, Okuda T, Miyagawa K, Kobayashi N, Takahashi R, Shimura A, Tamagawa S, Ohara Y, Hara T, ' +
          'Ishijima M. 2022. Three-dimensional morphological analysis of the thoracic pedicle and related ' +
          'radiographic factors in adolescent idiopathic scoliosis. BMC Musculoskelet Disord 23(1):847.',
        pmid: '36068555',
        doi: '10.1186/s12891-022-05799-4',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-concave-ratios-sato-2022-concave-convex-ratio',
    feature: 'Pedicle proportions beside the curve apex and neutral vertebra — concave convex ratio',
    publishedNumber:
      'key concave-convex-ratio, group apex-one: 0.73 ± 0.13 SD ratio (range 0.28 to 1.0); printed 0.73 ± 0.13 ' +
      '(0.28–1.00). key concave-convex-ratio, group neutral-one: 1.08 ± 0.17 (spread statistic not stated) ratio ' +
      '(range 0.77 to 1.54); printed 1.08 ± 0.17 (0.77–1.54). Published cortical-width left/right ratio; ' +
      'screw-sizing-and-placement.md P7 explicitly names its 0.13 as the ratio SD. The full-table note ' +
      'transcribes the same ±. Cohort: 57 patients, 49 female and 8 male; Lenke 1 (25) or Lenke 2 (32), 684 ' +
      'pedicles. Enrolled ages 12–20, Table 1 age 16.2 ± 2.4; 10–20 is the abstract inclusion range. The paper ' +
      'states right is always convex and left always concave. Per-cell observation counts are not supplied. ' +
      'Published left/right ratio; left is concave and right is convex. The table prints ± without defining its ' +
      'statistic. Cohort: 57 patients, 49 female and 8 male; Lenke 1 (25) or Lenke 2 (32), 684 pedicles. ' +
      'Enrolled ages 12–20, Table 1 age 16.2 ± 2.4; 10–20 is the abstract inclusion range. The paper states ' +
      'right is always convex and left always concave. Per-cell observation counts are not supplied.',
    population:
      'sato-2022: Japanese AIS, enrolled ages 12–20 (inclusion range 10–20), mean age 16.2; 49 girls and 8 boys; ' +
      'Lenke 1/2 right main thoracic curves; 0.5 mm CT. Right is always convex and left always concave. Sample: ' +
      '57 adolescents; 684 pedicles; 15 patients in the 26.3 percent group. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Sato T, Nojiri H, Okuda T, Miyagawa K, Kobayashi N, Takahashi R, Shimura A, Tamagawa S, Ohara Y, Hara T, ' +
          'Ishijima M. 2022. Three-dimensional morphological analysis of the thoracic pedicle and related ' +
          'radiographic factors in adolescent idiopathic scoliosis. BMC Musculoskelet Disord 23(1):847.',
        pmid: '36068555',
        doi: '10.1186/s12891-022-05799-4',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-concave-ratios-sato-2022-inner-height-ratio',
    feature: 'Pedicle proportions beside the curve apex and neutral vertebra — inner height ratio',
    publishedNumber:
      'key inner-height-ratio, group apex-one: 0.79 ± 0.13 (spread statistic not stated) ratio (range 0.36 to ' +
      '1.11); printed 0.79 ± 0.13 (0.36–1.11). key inner-height-ratio, group neutral-one: 1.06 ± 0.22 (spread ' +
      'statistic not stated) ratio (range 0.58 to 1.65); printed 1.06 ± 0.22 (0.58–1.65). Published left/right ' +
      'ratio; left is concave and right is convex. The table prints ± without defining its statistic. Cohort: 57 ' +
      'patients, 49 female and 8 male; Lenke 1 (25) or Lenke 2 (32), 684 pedicles. Enrolled ages 12–20, Table 1 ' +
      'age 16.2 ± 2.4; 10–20 is the abstract inclusion range. The paper states right is always convex and left ' +
      'always concave. Per-cell observation counts are not supplied.',
    population:
      'sato-2022: Japanese AIS, enrolled ages 12–20 (inclusion range 10–20), mean age 16.2; 49 girls and 8 boys; ' +
      'Lenke 1/2 right main thoracic curves; 0.5 mm CT. Right is always convex and left always concave. Sample: ' +
      '57 adolescents; 684 pedicles; 15 patients in the 26.3 percent group. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Sato T, Nojiri H, Okuda T, Miyagawa K, Kobayashi N, Takahashi R, Shimura A, Tamagawa S, Ohara Y, Hara T, ' +
          'Ishijima M. 2022. Three-dimensional morphological analysis of the thoracic pedicle and related ' +
          'radiographic factors in adolescent idiopathic scoliosis. BMC Musculoskelet Disord 23(1):847.',
        pmid: '36068555',
        doi: '10.1186/s12891-022-05799-4',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-concave-ratios-sato-2022-inner-width-ratio',
    feature: 'Pedicle proportions beside the curve apex and neutral vertebra — inner width ratio',
    publishedNumber:
      'key inner-width-ratio, group apex-one: 0.67 ± 0.24 (spread statistic not stated) ratio (range 0.11 to ' +
      '1.23); printed 0.67 ± 0.24 (0.11–1.23). key inner-width-ratio, group neutral-one: 1.38 ± 0.48 (spread ' +
      'statistic not stated) ratio (range 0.53 to 2.73); printed 1.38 ± 0.48 (0.53–2.73). Published left/right ' +
      'ratio; left is concave and right is convex. The table prints ± without defining its statistic. Cohort: 57 ' +
      'patients, 49 female and 8 male; Lenke 1 (25) or Lenke 2 (32), 684 pedicles. Enrolled ages 12–20, Table 1 ' +
      'age 16.2 ± 2.4; 10–20 is the abstract inclusion range. The paper states right is always convex and left ' +
      'always concave. Per-cell observation counts are not supplied.',
    population:
      'sato-2022: Japanese AIS, enrolled ages 12–20 (inclusion range 10–20), mean age 16.2; 49 girls and 8 boys; ' +
      'Lenke 1/2 right main thoracic curves; 0.5 mm CT. Right is always convex and left always concave. Sample: ' +
      '57 adolescents; 684 pedicles; 15 patients in the 26.3 percent group. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Sato T, Nojiri H, Okuda T, Miyagawa K, Kobayashi N, Takahashi R, Shimura A, Tamagawa S, Ohara Y, Hara T, ' +
          'Ishijima M. 2022. Three-dimensional morphological analysis of the thoracic pedicle and related ' +
          'radiographic factors in adolescent idiopathic scoliosis. BMC Musculoskelet Disord 23(1):847.',
        pmid: '36068555',
        doi: '10.1186/s12891-022-05799-4',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-concave-ratios-sato-2022-concave-convex-ratio-range',
    feature: 'Pedicle proportions beside the curve apex and neutral vertebra — concave convex ratio range',
    publishedNumber:
      'key concave-convex-ratio-range, group apex-one: 0.28 to 1.0 ratio; printed 0.28–1.00. Existing explicit ' +
      'range lookup retained: cortical-width left/right ratio at APEX ±1; the range is printed, not calculated.',
    population:
      'sato-2022: Japanese AIS, enrolled ages 12–20 (inclusion range 10–20), mean age 16.2; 49 girls and 8 boys; ' +
      'Lenke 1/2 right main thoracic curves; 0.5 mm CT. Right is always convex and left always concave. Sample: ' +
      '57 adolescents; 684 pedicles; 15 patients in the 26.3 percent group. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Sato T, Nojiri H, Okuda T, Miyagawa K, Kobayashi N, Takahashi R, Shimura A, Tamagawa S, Ohara Y, Hara T, ' +
          'Ishijima M. 2022. Three-dimensional morphological analysis of the thoracic pedicle and related ' +
          'radiographic factors in adolescent idiopathic scoliosis. BMC Musculoskelet Disord 23(1):847.',
        pmid: '36068555',
        doi: '10.1186/s12891-022-05799-4',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-diameter-rules-misenhimer-1989-outer-width-ratio-max',
    feature: 'Screw diameter beside the pedicle width — outer width ratio max',
    publishedNumber: 'key outer-width-ratio-max, group adult-cadaver: 0.8 ratio.',
    population:
      'misenhimer-1989: Adult cadaver spines. Sample: 6 fresh-frozen adult spines. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Misenhimer GR, Peek RD, Wiltse LL, Rothman SL, Widell EH. 1989. Anatomic analysis of pedicle cortical and ' +
          'cancellous diameter as related to screw size. Spine (Phila Pa 1976) 14(4):367-372.',
        pmid: '2718038',
        doi: '10.1097/00007632-198904000-00004',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-diameter-rules-sato-2022-outer-width-ratio-max',
    feature: 'Screw diameter beside the pedicle width — outer width ratio max',
    publishedNumber:
      'key outer-width-ratio-max, group expansion-25: 1.25 ratio. Cited expansion limit, not a direct estimate ' +
      'from this cohort; Takeshita/Yazici primaries not read in full.',
    population:
      'sato-2022: Japanese AIS, enrolled ages 12–20 (inclusion range 10–20), mean age 16.2; 49 girls and 8 boys; ' +
      'Lenke 1/2 right main thoracic curves; 0.5 mm CT. Right is always convex and left always concave. Sample: ' +
      '57 adolescents; 684 pedicles; 15 patients in the 26.3 percent group. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Sato T, Nojiri H, Okuda T, Miyagawa K, Kobayashi N, Takahashi R, Shimura A, Tamagawa S, Ohara Y, Hara T, ' +
          'Ishijima M. 2022. Three-dimensional morphological analysis of the thoracic pedicle and related ' +
          'radiographic factors in adolescent idiopathic scoliosis. BMC Musculoskelet Disord 23(1):847.',
        pmid: '36068555',
        doi: '10.1186/s12891-022-05799-4',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-diameter-rules-upendra-2010-outer-width-ratio',
    feature: 'Screw diameter beside the pedicle width — outer width ratio',
    publishedNumber:
      'key outer-width-ratio, group upendra-cited: 0.8 to 1.15 ratio. Range cited from other work, not newly ' +
      'measured by Upendra.',
    population:
      'upendra-2010: AIS; 14 girls and 10 boys; all right thoracic or thoracolumbar curves (King II 12, III 7, ' +
      'IV 5); ages 12.3–18.3; Cobb 58–98°; 3 mm CT. Sample: 24 patients; width n 213 convex/223 concave; angle n ' +
      '217 convex/216 concave; chord n 213 on each side; per-cell pedicle counts retained. [peer-reviewed; ' +
      'full-text].',
    sources: [
      {
        citation:
          'Upendra B, Meena D, Kandwal P, Ahmed A, Chowdhury B, Jayaswal A. 2010. Pedicle morphometry in patients ' +
          'with adolescent idiopathic scoliosis. Indian J Orthop 44(2):169-176.',
        pmid: '20419004',
        doi: '10.4103/0019-5413.62084',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-diameter-rules-sato-2022-pedicle-enlargement',
    feature: 'Screw diameter beside the pedicle width — pedicle enlargement',
    publishedNumber: 'key pedicle-enlargement, group sato-cited: 15 to 25 percent.',
    population:
      'sato-2022: Japanese AIS, enrolled ages 12–20 (inclusion range 10–20), mean age 16.2; 49 girls and 8 boys; ' +
      'Lenke 1/2 right main thoracic curves; 0.5 mm CT. Right is always convex and left always concave. Sample: ' +
      '57 adolescents; 684 pedicles; 15 patients in the 26.3 percent group. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Sato T, Nojiri H, Okuda T, Miyagawa K, Kobayashi N, Takahashi R, Shimura A, Tamagawa S, Ohara Y, Hara T, ' +
          'Ishijima M. 2022. Three-dimensional morphological analysis of the thoracic pedicle and related ' +
          'radiographic factors in adolescent idiopathic scoliosis. BMC Musculoskelet Disord 23(1):847.',
        pmid: '36068555',
        doi: '10.1186/s12891-022-05799-4',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-diameter-rules-sato-2022-screw-diameter',
    feature: 'Screw diameter beside the pedicle width — screw diameter',
    publishedNumber: 'key screw-diameter, group expansion-25: 4.0 mm.',
    population:
      'sato-2022: Japanese AIS, enrolled ages 12–20 (inclusion range 10–20), mean age 16.2; 49 girls and 8 boys; ' +
      'Lenke 1/2 right main thoracic curves; 0.5 mm CT. Right is always convex and left always concave. Sample: ' +
      '57 adolescents; 684 pedicles; 15 patients in the 26.3 percent group. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Sato T, Nojiri H, Okuda T, Miyagawa K, Kobayashi N, Takahashi R, Shimura A, Tamagawa S, Ohara Y, Hara T, ' +
          'Ishijima M. 2022. Three-dimensional morphological analysis of the thoracic pedicle and related ' +
          'radiographic factors in adolescent idiopathic scoliosis. BMC Musculoskelet Disord 23(1):847.',
        pmid: '36068555',
        doi: '10.1186/s12891-022-05799-4',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-diameter-rules-sato-2022-outer-width-required-above',
    feature: 'Screw diameter beside the pedicle width — outer width required above',
    publishedNumber: 'key outer-width-required-above, group expansion-25: 3.2 mm.',
    population:
      'sato-2022: Japanese AIS, enrolled ages 12–20 (inclusion range 10–20), mean age 16.2; 49 girls and 8 boys; ' +
      'Lenke 1/2 right main thoracic curves; 0.5 mm CT. Right is always convex and left always concave. Sample: ' +
      '57 adolescents; 684 pedicles; 15 patients in the 26.3 percent group. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Sato T, Nojiri H, Okuda T, Miyagawa K, Kobayashi N, Takahashi R, Shimura A, Tamagawa S, Ohara Y, Hara T, ' +
          'Ishijima M. 2022. Three-dimensional morphological analysis of the thoracic pedicle and related ' +
          'radiographic factors in adolescent idiopathic scoliosis. BMC Musculoskelet Disord 23(1):847.',
        pmid: '36068555',
        doi: '10.1186/s12891-022-05799-4',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-diameter-rules-grabala-2023-expansion-by-level',
    feature: 'Screw diameter beside the pedicle width — expansion by level',
    publishedNumber:
      'key expansion-by-level, group grabala: 47 to 71 percent. Approximate range as printed in P1; width ' +
      'definition unclear. Not a diameter-ratio conversion. Mixed-cause cohort n = 230: idiopathic 164, ' +
      'congenital 25, neuromuscular 23 and syndromic 18; not AIS only. Table 3 has internal discrepancies: T5 ' +
      'postoperative 6.07 minus preoperative 3.79 is reported in the audit as 2.28 versus printed Mean Expanding ' +
      '2.55; L3 6.88 minus 5.14 as 1.74 versus 2.89; L5 7.02 minus 5.68 as 1.34 versus 2.68. Preoperative N = ' +
      '7954 and postoperative N = 5080 are different pedicle sets. The published expansion values are retained ' +
      'without repair.',
    population:
      'grabala-2023: Children aged 2–18 with mixed causes: idiopathic 164, congenital 25, neuromuscular 23, ' +
      'syndromic 18; MRI before and CT after surgery; single centre. Not an AIS-only cohort. Sample: 230 ' +
      'patients; 7,954 preoperative pedicles and 5,080 postoperative screws, different sets. [peer-reviewed; ' +
      'full-text].',
    sources: [
      {
        citation:
          'Grabala P, Helenius IJ, Grabala M, Shah SA. 2023. Influences of Increasing Pedicle Screw Diameter on ' +
          'Widening Vertebral Pedicle Size during Surgery in Spinal Deformities in Children and Adolescents without ' +
          'Higher Risk of Pedicle and Vertebral Breaches. J Clin Med 12(16):5368.',
        pmid: '37629410',
        doi: '10.3390/jcm12165368',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-diameter-rules-grabala-2023-expansion',
    feature: 'Screw diameter beside the pedicle width — expansion',
    publishedNumber:
      'key expansion, group grabala-table-5.0: 32 percent. key expansion, group grabala-table-5.5: 39 percent. ' +
      'key expansion, group grabala-table-6.0: 49 percent. key expansion, group grabala-table-6.5: 67 percent. ' +
      'key expansion, group grabala-table-7.0: 78 percent. key expansion, group grabala-text-5.0: 39 percent. ' +
      'key expansion, group grabala-age-2-5: 49 percent. key expansion, group grabala-age-6-10: 52 percent. key ' +
      'expansion, group grabala-age-11-18: 62 percent. Percentage, not a multiplier; MRI width definition ' +
      'unclear. Mixed-cause cohort n = 230: idiopathic 164, congenital 25, neuromuscular 23 and syndromic 18; ' +
      'not AIS only. Table 3 has internal discrepancies: T5 postoperative 6.07 minus preoperative 3.79 is ' +
      'reported in the audit as 2.28 versus printed Mean Expanding 2.55; L3 6.88 minus 5.14 as 1.74 versus 2.89; ' +
      'L5 7.02 minus 5.68 as 1.34 versus 2.68. Preoperative N = 7954 and postoperative N = 5080 are different ' +
      'pedicle sets. The published expansion values are retained without repair. Internal source conflict: text ' +
      'gives 39 percent, table gives 32 percent for 5.0 mm. Mixed-cause cohort n = 230: idiopathic 164, ' +
      'congenital 25, neuromuscular 23 and syndromic 18; not AIS only. Table 3 has internal discrepancies: T5 ' +
      'postoperative 6.07 minus preoperative 3.79 is reported in the audit as 2.28 versus printed Mean Expanding ' +
      '2.55; L3 6.88 minus 5.14 as 1.74 versus 2.89; L5 7.02 minus 5.68 as 1.34 versus 2.68. Preoperative N = ' +
      '7954 and postoperative N = 5080 are different pedicle sets. The published expansion values are retained ' +
      'without repair. Mixed-cause cohort n = 230: idiopathic 164, congenital 25, neuromuscular 23 and syndromic ' +
      '18; not AIS only. Table 3 has internal discrepancies: T5 postoperative 6.07 minus preoperative 3.79 is ' +
      'reported in the audit as 2.28 versus printed Mean Expanding 2.55; L3 6.88 minus 5.14 as 1.74 versus 2.89; ' +
      'L5 7.02 minus 5.68 as 1.34 versus 2.68. Preoperative N = 7954 and postoperative N = 5080 are different ' +
      'pedicle sets. The published expansion values are retained without repair.',
    population:
      'grabala-2023: Children aged 2–18 with mixed causes: idiopathic 164, congenital 25, neuromuscular 23, ' +
      'syndromic 18; MRI before and CT after surgery; single centre. Not an AIS-only cohort. Sample: 230 ' +
      'patients; 7,954 preoperative pedicles and 5,080 postoperative screws, different sets. [peer-reviewed; ' +
      'full-text].',
    sources: [
      {
        citation:
          'Grabala P, Helenius IJ, Grabala M, Shah SA. 2023. Influences of Increasing Pedicle Screw Diameter on ' +
          'Widening Vertebral Pedicle Size during Surgery in Spinal Deformities in Children and Adolescents without ' +
          'Higher Risk of Pedicle and Vertebral Breaches. J Clin Med 12(16):5368.',
        pmid: '37629410',
        doi: '10.3390/jcm12165368',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-diameter-rules-katsumi-2025-deviation-odds',
    feature: 'Screw diameter beside the pedicle width — deviation odds',
    publishedNumber:
      'key deviation-odds, group screw-pedicle-ratio: 1.607 ratio. Increment of the ratio corresponding to the ' +
      'OR is not supplied; cannot be used as a dose-response coefficient.',
    population:
      'katsumi-2025: AIS, navigated screws; Lenke 1 and Lenke 5 compared. Sample: 67 AIS patients; 1,082 ' +
      'navigated screws. [peer-reviewed; abstract].',
    sources: [
      {
        citation: 'Katsumi S, Shinohara A, Arimura D, Obata S, Wakiya H, Saito M. 2025. Eur Spine J 34(11):5343-5352.',
        pmid: '40856839',
        doi: '10.1007/s00586-025-09210-3',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-inner-walls-chiu-2024-fully-cortical-pedicles',
    feature: 'Pedicle wall thickness by side and region — fully cortical pedicles',
    publishedNumber:
      'key fully-cortical-pedicles, group chiu-all: 264 count. key fully-cortical-pedicles, group chiu-right: ' +
      '183 count. key fully-cortical-pedicles, group chiu-left: 81 count. Fully corticalized pedicles excluded ' +
      'from the 6,230 analysed pedicles; AIS with major thoracic curves. Right-side fully corticalized pedicles ' +
      'excluded from analysis. Left-side fully corticalized pedicles excluded from analysis.',
    population:
      'chiu-2024: AIS with major thoracic curves; age range not supplied by the audit. Sample: 191 patients; ' +
      '6,230 analysed pedicles (right 3,064; left 3,166); 264 fully corticalized pedicles excluded (right 183; ' +
      'left 81). [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Chiu CK, Wang WJ, Lee YJ, Chung WH, Chan CYW, Kwan MK. 2024. The widths of the medial and lateral pedicle ' +
          'walls in adolescent idiopathic scoliosis with major thoracic curves. Spine J 24(7):1293-1301.',
        pmid: '38408520',
        doi: '10.1016/j.spinee.2024.02.014',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-inner-walls-chiu-2024-medial-wall-thickness',
    feature: 'Pedicle wall thickness by side and region — medial wall thickness',
    publishedNumber:
      'key medial-wall-thickness, group chiu-right-t1-t3: 0.89 ± 0.28 (spread statistic not stated) mm; printed ' +
      '0.89±0.28 mm. key medial-wall-thickness, group chiu-right-t4-t10: 0.75 ± 0.23 (spread statistic not ' +
      'stated) mm; printed 0.75±0.23 mm. key medial-wall-thickness, group chiu-right-t11-l5: 0.92 ± 0.3 (spread ' +
      'statistic not stated) mm; printed 0.92±0.30 mm. key medial-wall-thickness, group chiu-left-t1-t3: 0.88 ± ' +
      '0.26 (spread statistic not stated) mm; printed 0.88±0.26 mm. key medial-wall-thickness, group ' +
      'chiu-left-t4-t7: 0.76 ± 0.24 (spread statistic not stated) mm; printed 0.76±0.24 mm. key ' +
      'medial-wall-thickness, group chiu-left-t8-l5: 0.9 ± 0.27 (spread statistic not stated) mm; printed ' +
      '0.90±0.27 mm. key medial-wall-thickness, group chiu-right: 0.85 ± 0.28 (spread statistic not stated) mm; ' +
      'printed 0.85±0.28 mm. key medial-wall-thickness, group chiu-left: 0.86 ± 0.26 (spread statistic not ' +
      'stated) mm; printed 0.86±0.26 mm. key medial-wall-thickness, level T7, group chiu-side-unresolved: 0.73 ± ' +
      '0.24 SD mm; printed 0.73±0.24 mm. key medial-wall-thickness, level T8, group chiu-side-unresolved: 0.73 ± ' +
      '0.23 SD mm; printed 0.73±0.23 mm. Physical side and region as printed; the abstract does not name the ± ' +
      'statistic. AIS with major thoracic curves; 191 patients, 6230 analysed pedicles (right 3064, left 3166); ' +
      '264 fully corticalised pedicles excluded (right 183, left 81). Per-region counts are not supplied. The ' +
      'abstract says "the left concave medial pedicle width being thicker than the right convex medial pedicle ' +
      'width", but also "The thinnest medial pedicle walls were located at right concave T7" and "The right ' +
      'concave periapical region had the thinnest medial pedicle walls". Its curve-side attribution is ' +
      'unresolved; no curve side is assigned. Right-side wall value, not an all-pedicle value. The abstract ' +
      'prints ± without naming either statistic. AIS with major thoracic curves; 264 fully cortical pedicles ' +
      'excluded from analysis. Published observation retained beside regional values. AIS with major thoracic ' +
      'curves; 191 patients, 6230 analysed pedicles (right 3064, left 3166); 264 fully corticalised pedicles ' +
      'excluded (right 183, left 81). Per-region counts are not supplied. The abstract says "the left concave ' +
      'medial pedicle width being thicker than the right convex medial pedicle width", but also "The thinnest ' +
      'medial pedicle walls were located at right concave T7" and "The right concave periapical region had the ' +
      'thinnest medial pedicle walls". Its curve-side attribution is unresolved; no curve side is assigned. ' +
      'Left-side wall value. The abstract prints ± without naming either statistic. AIS with major thoracic ' +
      'curves; 264 fully cortical pedicles excluded from analysis. Published observation retained beside ' +
      'regional values. AIS with major thoracic curves; 191 patients, 6230 analysed pedicles (right 3064, left ' +
      '3166); 264 fully corticalised pedicles excluded (right 183, left 81). Per-region counts are not supplied. ' +
      'The abstract says "the left concave medial pedicle width being thicker than the right convex medial ' +
      'pedicle width", but also "The thinnest medial pedicle walls were located at right concave T7" and "The ' +
      'right concave periapical region had the thinnest medial pedicle walls". Its curve-side attribution is ' +
      'unresolved; no curve side is assigned. Side unresolved. The abstract calls these thinnest medial walls ' +
      '“right concave”, but elsewhere calls the sides “left concave” and “right convex”; no physical side or ' +
      'curve side is assigned. Per-level medial-wall mean and SD, not an unspecified-wall aggregate. Published ' +
      'observation retained beside regional values. AIS with major thoracic curves; 191 patients, 6230 analysed ' +
      'pedicles (right 3064, left 3166); 264 fully corticalised pedicles excluded (right 183, left 81). ' +
      'Per-region counts are not supplied. The abstract says "the left concave medial pedicle width being ' +
      'thicker than the right convex medial pedicle width", but also "The thinnest medial pedicle walls were ' +
      'located at right concave T7" and "The right concave periapical region had the thinnest medial pedicle ' +
      'walls". Its curve-side attribution is unresolved; no curve side is assigned.',
    population:
      'chiu-2024: AIS with major thoracic curves; age range not supplied by the audit. Sample: 191 patients; ' +
      '6,230 analysed pedicles (right 3,064; left 3,166); 264 fully corticalized pedicles excluded (right 183; ' +
      'left 81). [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Chiu CK, Wang WJ, Lee YJ, Chung WH, Chan CYW, Kwan MK. 2024. The widths of the medial and lateral pedicle ' +
          'walls in adolescent idiopathic scoliosis with major thoracic curves. Spine J 24(7):1293-1301.',
        pmid: '38408520',
        doi: '10.1016/j.spinee.2024.02.014',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-inner-walls-chiu-2024-lateral-wall-thickness',
    feature: 'Pedicle wall thickness by side and region — lateral wall thickness',
    publishedNumber:
      'key lateral-wall-thickness, group chiu-right: 0.64 ± 0.26 (spread statistic not stated) mm; printed ' +
      '0.64±0.26 mm. key lateral-wall-thickness, group chiu-left: 0.64 ± 0.26 (spread statistic not stated) mm; ' +
      'printed 0.64±0.26 mm. Right-side wall value, not an all-pedicle value. The abstract prints ± without ' +
      'naming either statistic. AIS with major thoracic curves; 264 fully cortical pedicles excluded from ' +
      'analysis. Published observation retained beside regional values. AIS with major thoracic curves; 191 ' +
      'patients, 6230 analysed pedicles (right 3064, left 3166); 264 fully corticalised pedicles excluded (right ' +
      '183, left 81). Per-region counts are not supplied. The abstract says "the left concave medial pedicle ' +
      'width being thicker than the right convex medial pedicle width", but also "The thinnest medial pedicle ' +
      'walls were located at right concave T7" and "The right concave periapical region had the thinnest medial ' +
      'pedicle walls". Its curve-side attribution is unresolved; no curve side is assigned. Left-side wall ' +
      'value. The abstract prints ± without naming either statistic. AIS with major thoracic curves; 264 fully ' +
      'cortical pedicles excluded from analysis. Published observation retained beside regional values. AIS with ' +
      'major thoracic curves; 191 patients, 6230 analysed pedicles (right 3064, left 3166); 264 fully ' +
      'corticalised pedicles excluded (right 183, left 81). Per-region counts are not supplied. The abstract ' +
      'says "the left concave medial pedicle width being thicker than the right convex medial pedicle width", ' +
      'but also "The thinnest medial pedicle walls were located at right concave T7" and "The right concave ' +
      'periapical region had the thinnest medial pedicle walls". Its curve-side attribution is unresolved; no ' +
      'curve side is assigned.',
    population:
      'chiu-2024: AIS with major thoracic curves; age range not supplied by the audit. Sample: 191 patients; ' +
      '6,230 analysed pedicles (right 3,064; left 3,166); 264 fully corticalized pedicles excluded (right 183; ' +
      'left 81). [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Chiu CK, Wang WJ, Lee YJ, Chung WH, Chan CYW, Kwan MK. 2024. The widths of the medial and lateral pedicle ' +
          'walls in adolescent idiopathic scoliosis with major thoracic curves. Spine J 24(7):1293-1301.',
        pmid: '38408520',
        doi: '10.1016/j.spinee.2024.02.014',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-inner-widths-zheng-2009-inner-width',
    feature: 'Inner pedicle widths from population tables — inner width',
    publishedNumber:
      'key inner-width, ageBand 5-8, group zheng-t4-t6: 1.55 to 1.81 mm. key inner-width, level T1, ageBand 5-8, ' +
      'sex male, group zheng-children: 3.19 ± 0.63 (spread statistic not stated) mm; printed 3.19 ± 0.63. key ' +
      'inner-width, level T1, ageBand 5-8, sex female, group zheng-children: 2.95 ± 0.53 (spread statistic not ' +
      'stated) mm; printed 2.95 ± 0.53. key inner-width, level T1, ageBand 9-11, sex male, group zheng-children: ' +
      '4.15 ± 0.15 (spread statistic not stated) mm; printed 4.15 ± 0.15; doubtful. key inner-width, level T1, ' +
      'ageBand 9-11, sex female, group zheng-children: 3.26 ± 0.52 (spread statistic not stated) mm; printed ' +
      '3.26 ± 0.52. key inner-width, level T1, ageBand 12-14, sex male, group zheng-children: 4.41 ± 0.93 ' +
      '(spread statistic not stated) mm; printed 4.41 ± 0.93. key inner-width, level T1, ageBand 12-14, sex ' +
      'female, group zheng-children: 3.76 ± 0.7 (spread statistic not stated) mm; printed 3.76 ± 0.70. key ' +
      'inner-width, level T2, ageBand 5-8, sex male, group zheng-children: 2.25 ± 0.57 (spread statistic not ' +
      'stated) mm; printed 2.25 ± 0.57. key inner-width, level T2, ageBand 5-8, sex female, group ' +
      'zheng-children: 2.61 ± 0.54 (spread statistic not stated) mm; printed 2.61 ± 0.54. key inner-width, level ' +
      'T2, ageBand 9-11, sex male, group zheng-children: 3.27 ± 0.49 (spread statistic not stated) mm; printed ' +
      '3.27 ± 0.49. key inner-width, level T2, ageBand 9-11, sex female, group zheng-children: 2.81 ± 0.51 ' +
      '(spread statistic not stated) mm; printed 2.81 ± 0.51. key inner-width, level T2, ageBand 12-14, sex ' +
      'male, group zheng-children: 3.52 ± 0.77 (spread statistic not stated) mm; printed 3.52 ± 0.77. key ' +
      'inner-width, level T2, ageBand 12-14, sex female, group zheng-children: 3.09 ± 0.65 (spread statistic not ' +
      'stated) mm; printed 3.09 ± 0.65. key inner-width, level T3, ageBand 5-8, sex male, group zheng-children: ' +
      '2.07 ± 0.38 (spread statistic not stated) mm; printed 2.07 ± 0.38. key inner-width, level T3, ageBand ' +
      '5-8, sex female, group zheng-children: 2.14 ± 0.41 (spread statistic not stated) mm; printed 2.14 ± 0.41. ' +
      'key inner-width, level T3, ageBand 9-11, sex male, group zheng-children: 2.7 ± 0.6 (spread statistic not ' +
      'stated) mm; printed 2.70 ± 0.60. key inner-width, level T3, ageBand 9-11, sex female, group ' +
      'zheng-children: 2.28 ± 0.36 (spread statistic not stated) mm; printed 2.28 ± 0.36. key inner-width, level ' +
      'T3, ageBand 12-14, sex male, group zheng-children: 2.98 ± 0.73 (spread statistic not stated) mm; printed ' +
      '2.98 ± 0.73. key inner-width, level T3, ageBand 12-14, sex female, group zheng-children: 2.5 ± 0.47 ' +
      '(spread statistic not stated) mm; printed 2.50 ± 0.47. key inner-width, level T4, ageBand 5-8, sex male, ' +
      'group zheng-children: 1.76 ± 0.42 (spread statistic not stated) mm; printed 1.76 ± 0.42. key inner-width, ' +
      'level T4, ageBand 5-8, sex female, group zheng-children: 1.55 ± 0.28 (spread statistic not stated) mm; ' +
      'printed 1.55 ± 0.28. key inner-width, level T4, ageBand 9-11, sex male, group zheng-children: 1.97 ± 0.43 ' +
      '(spread statistic not stated) mm; printed 1.97 ± 0.43. key inner-width, level T4, ageBand 9-11, sex ' +
      'female, group zheng-children: 1.94 ± 0.35 (spread statistic not stated) mm; printed 1.94 ± 0.35. key ' +
      'inner-width, level T4, ageBand 12-14, sex male, group zheng-children: 2.25 ± 0.66 (spread statistic not ' +
      'stated) mm; printed 2.25 ± 0.66. key inner-width, level T4, ageBand 12-14, sex female, group ' +
      'zheng-children: 2.06 ± 0.53 (spread statistic not stated) mm; printed 2.06 ± 0.53. key inner-width, level ' +
      'T5, ageBand 5-8, sex male, group zheng-children: 1.66 ± 0.42 (spread statistic not stated) mm; printed ' +
      '1.66 ± 0.42. key inner-width, level T5, ageBand 5-8, sex female, group zheng-children: 1.65 ± 0.27 ' +
      '(spread statistic not stated) mm; printed 1.65 ± 0.27. key inner-width, level T5, ageBand 9-11, sex male, ' +
      'group zheng-children: 2.2 ± 0.49 (spread statistic not stated) mm; printed 2.20 ± 0.49. key inner-width, ' +
      'level T5, ageBand 9-11, sex female, group zheng-children: 1.9 ± 0.31 (spread statistic not stated) mm; ' +
      'printed 1.90 ± 0.31. key inner-width, level T5, ageBand 12-14, sex male, group zheng-children: 2.29 ± 0.5 ' +
      '(spread statistic not stated) mm; printed 2.29 ± 0.50. key inner-width, level T5, ageBand 12-14, sex ' +
      'female, group zheng-children: 2.13 ± 0.43 (spread statistic not stated) mm; printed 2.13 ± 0.43. key ' +
      'inner-width, level T6, ageBand 5-8, sex male, group zheng-children: 1.81 ± 0.37 (spread statistic not ' +
      'stated) mm; printed 1.81 ± 0.37. key inner-width, level T6, ageBand 5-8, sex female, group ' +
      'zheng-children: 1.73 ± 0.27 (spread statistic not stated) mm; printed 1.73 ± 0.27. key inner-width, level ' +
      'T6, ageBand 9-11, sex male, group zheng-children: 2.24 ± 0.68 (spread statistic not stated) mm; printed ' +
      '2.24 ± 0.68. key inner-width, level T6, ageBand 9-11, sex female, group zheng-children: 1.91 ± 0.39 ' +
      '(spread statistic not stated) mm; printed 1.91 ± 0.39. key inner-width, level T6, ageBand 12-14, sex ' +
      'male, group zheng-children: 2.42 ± 0.55 (spread statistic not stated) mm; printed 2.42 ± 0.55. key ' +
      'inner-width, level T6, ageBand 12-14, sex female, group zheng-children: 2.26 ± 0.5 (spread statistic not ' +
      'stated) mm; printed 2.26 ± 0.50. key inner-width, level T7, ageBand 5-8, sex male, group zheng-children: ' +
      '1.99 ± 0.43 (spread statistic not stated) mm; printed 1.99 ± 0.43. key inner-width, level T7, ageBand ' +
      '5-8, sex female, group zheng-children: 1.86 ± 0.57 (spread statistic not stated) mm; printed 1.86 ± 0.57. ' +
      'key inner-width, level T7, ageBand 9-11, sex male, group zheng-children: 2.45 ± 0.69 (spread statistic ' +
      'not stated) mm; printed 2.45 ± 0.69. key inner-width, level T7, ageBand 9-11, sex female, group ' +
      'zheng-children: 1.96 ± 0.35 (spread statistic not stated) mm; printed 1.96 ± 0.35. key inner-width, level ' +
      'T7, ageBand 12-14, sex male, group zheng-children: 2.58 ± 0.64 (spread statistic not stated) mm; printed ' +
      '2.58 ± 0.64. key inner-width, level T7, ageBand 12-14, sex female, group zheng-children: 2.39 ± 0.49 ' +
      '(spread statistic not stated) mm; printed 2.39 ± 0.49. key inner-width, level T8, ageBand 5-8, sex male, ' +
      'group zheng-children: 2.26 ± 0.54 (spread statistic not stated) mm; printed 2.26 ± 0.54. key inner-width, ' +
      'level T8, ageBand 5-8, sex female, group zheng-children: 1.98 ± 0.35 (spread statistic not stated) mm; ' +
      'printed 1.98 ± 0.35. key inner-width, level T8, ageBand 9-11, sex male, group zheng-children: 2.71 ± 0.66 ' +
      '(spread statistic not stated) mm; printed 2.71 ± 0.66. key inner-width, level T8, ageBand 9-11, sex ' +
      'female, group zheng-children: 2.38 ± 0.53 (spread statistic not stated) mm; printed 2.38 ± 0.53. key ' +
      'inner-width, level T8, ageBand 12-14, sex male, group zheng-children: 3.16 ± 1.5 (spread statistic not ' +
      'stated) mm; printed 3.16 ± 1.50. key inner-width, level T8, ageBand 12-14, sex female, group ' +
      'zheng-children: 2.64 ± 0.51 (spread statistic not stated) mm; printed 2.64 ± 0.51. key inner-width, level ' +
      'T9, ageBand 5-8, sex male, group zheng-children: 2.6 ± 0.56 (spread statistic not stated) mm; printed ' +
      '2.60 ± 0.56. key inner-width, level T9, ageBand 5-8, sex female, group zheng-children: 2.14 ± 0.36 ' +
      '(spread statistic not stated) mm; printed 2.14 ± 0.36. key inner-width, level T9, ageBand 9-11, sex male, ' +
      'group zheng-children: 2.93 ± 0.62 (spread statistic not stated) mm; printed 2.93 ± 0.62. key inner-width, ' +
      'level T9, ageBand 9-11, sex female, group zheng-children: 2.53 ± 0.62 (spread statistic not stated) mm; ' +
      'printed 2.53 ± 0.62. key inner-width, level T9, ageBand 12-14, sex male, group zheng-children: 3.3 ± 0.9 ' +
      '(spread statistic not stated) mm; printed 3.30 ± 0.90. key inner-width, level T9, ageBand 12-14, sex ' +
      'female, group zheng-children: 2.84 ± 0.62 (spread statistic not stated) mm; printed 2.84 ± 0.62. key ' +
      'inner-width, level T10, ageBand 5-8, sex male, group zheng-children: 2.86 ± 0.71 (spread statistic not ' +
      'stated) mm; printed 2.86 ± 0.71. key inner-width, level T10, ageBand 5-8, sex female, group ' +
      'zheng-children: 2.42 ± 0.49 (spread statistic not stated) mm; printed 2.42 ± 0.49. key inner-width, level ' +
      'T10, ageBand 9-11, sex male, group zheng-children: 3.31 ± 0.65 (spread statistic not stated) mm; printed ' +
      '3.31 ± 0.65. key inner-width, level T10, ageBand 9-11, sex female, group zheng-children: 2.99 ± 0.64 ' +
      '(spread statistic not stated) mm; printed 2.99 ± 0.64. key inner-width, level T10, ageBand 12-14, sex ' +
      'male, group zheng-children: 3.98 ± 0.98 (spread statistic not stated) mm; printed 3.98 ± 0.98. key ' +
      'inner-width, level T10, ageBand 12-14, sex female, group zheng-children: 3.39 ± 0.97 (spread statistic ' +
      'not stated) mm; printed 3.39 ± 0.97. key inner-width, level T11, ageBand 5-8, sex male, group ' +
      'zheng-children: 3.21 ± 0.65 (spread statistic not stated) mm; printed 3.21 ± 0.65. key inner-width, level ' +
      'T11, ageBand 5-8, sex female, group zheng-children: 2.89 ± 0.63 (spread statistic not stated) mm; printed ' +
      '2.89 ± 0.63. key inner-width, level T11, ageBand 9-11, sex male, group zheng-children: 3.81 ± 0.78 ' +
      '(spread statistic not stated) mm; printed 3.81 ± 0.78. key inner-width, level T11, ageBand 9-11, sex ' +
      'female, group zheng-children: 3.42 ± 0.8 (spread statistic not stated) mm; printed 3.42 ± 0.80. key ' +
      'inner-width, level T11, ageBand 12-14, sex male, group zheng-children: 4.65 ± 1.0 (spread statistic not ' +
      'stated) mm; printed 4.65 ± 1.00. key inner-width, level T11, ageBand 12-14, sex female, group ' +
      'zheng-children: 4.13 ± 1.28 (spread statistic not stated) mm; printed 4.13 ± 1.28. key inner-width, level ' +
      'T12, ageBand 5-8, sex male, group zheng-children: 3.51 ± 0.69 (spread statistic not stated) mm; printed ' +
      '3.51 ± 0.69. key inner-width, level T12, ageBand 5-8, sex female, group zheng-children: 3.22 ± 0.69 ' +
      '(spread statistic not stated) mm; printed 3.22 ± 0.69. key inner-width, level T12, ageBand 9-11, sex ' +
      'male, group zheng-children: 4.31 ± 0.75 (spread statistic not stated) mm; printed 4.31 ± 0.75. key ' +
      'inner-width, level T12, ageBand 9-11, sex female, group zheng-children: 3.73 ± 0.85 (spread statistic not ' +
      'stated) mm; printed 3.73 ± 0.85. key inner-width, level T12, ageBand 12-14, sex male, group ' +
      'zheng-children: 4.99 ± 1.05 (spread statistic not stated) mm; printed 4.99 ± 1.05. key inner-width, level ' +
      'T12, ageBand 12-14, sex female, group zheng-children: 4.63 ± 1.32 (spread statistic not stated) mm; ' +
      'printed 4.63 ± 1.32. Note-supplied pooled T4–T6 range for ages 5–8. The paper gives individual level and ' +
      'sex cells with spreads; the complete Table 1 cells are retained separately. [own inference] The first ' +
      'Table 1 row at each level is read as inner width and the second as outer width; the paper does not label ' +
      'those rows. The unlabelled ± is read as SD only by project inference. Full text. The paper does not state ' +
      'what ± denotes; reading it as a standard deviation is this project\'s own inference. Numbers of children ' +
      'per age band and sex are not stated. The first of the two unlabelled Table 1 rows at each level is stored ' +
      'as inner width and the second as outer width; that assignment is this project\'s own inference.',
    population:
      'zheng-2009: Chinese children aged 5–14 from a hospital registry, stratified random sampling with thoracic ' +
      'abnormalities excluded; not scoliosis-specific; 2 mm axial CT. Sample: 120 children aged 5–14; number per ' +
      'age band and sex is not stated. [peer-reviewed; full-text].',
    sources: [
      {
        citation: 'Zheng C, Huang Q, Hu Y, Wang X, Chen W. 2009. Int Orthop 33(6):1663-1668.',
        pmid: '18956181',
        doi: '10.1007/s00264-008-0675-z',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-inner-widths-sato-2022-inner-width',
    feature: 'Inner pedicle widths from population tables — inner width',
    publishedNumber:
      'key inner-width, group sato-apex-one, curveSide convex: 2.6 ± 0.7 (spread statistic not stated) mm (range ' +
      '1.2 to 4.0); printed 2.6 ± 0.7 (1.2–4.0). key inner-width, group sato-apex-one, curveSide concave: 1.7 ± ' +
      '0.7 (spread statistic not stated) mm (range 0.2 to 3.9); printed 1.7 ± 0.7 (0.2–3.9). key inner-width, ' +
      'group sato-neutral-one, curveSide convex: 1.7 ± 0.7 (spread statistic not stated) mm (range 0.2 to 3.9); ' +
      'printed 1.7 ± 0.7 (0.2–3.9); retained only, excluded from use; probable copy. key inner-width, group ' +
      'sato-neutral-one, curveSide concave: 1.8 ± 0.7 (spread statistic not stated) mm (range 0.7 to 4.1); ' +
      'printed 1.8 ± 0.7 (0.7–4.1). Measurements are in a plane perpendicular to the pedicle axis through the ' +
      'isthmus. The table prints ± without defining its statistic. Cohort: 57 patients, 49 female and 8 male; ' +
      'Lenke 1 (25) or Lenke 2 (32), 684 pedicles. Enrolled ages 12–20, Table 1 age 16.2 ± 2.4; 10–20 is the ' +
      'abstract inclusion range. The paper states right is always convex and left always concave. Per-cell ' +
      'observation counts are not supplied.',
    population:
      'sato-2022: Japanese AIS, enrolled ages 12–20 (inclusion range 10–20), mean age 16.2; 49 girls and 8 boys; ' +
      'Lenke 1/2 right main thoracic curves; 0.5 mm CT. Right is always convex and left always concave. Sample: ' +
      '57 adolescents; 684 pedicles; 15 patients in the 26.3 percent group. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Sato T, Nojiri H, Okuda T, Miyagawa K, Kobayashi N, Takahashi R, Shimura A, Tamagawa S, Ohara Y, Hara T, ' +
          'Ishijima M. 2022. Three-dimensional morphological analysis of the thoracic pedicle and related ' +
          'radiographic factors in adolescent idiopathic scoliosis. BMC Musculoskelet Disord 23(1):847.',
        pmid: '36068555',
        doi: '10.1186/s12891-022-05799-4',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-inner-widths-sato-2022-p-value-max',
    feature: 'Inner pedicle widths from population tables — p value max',
    publishedNumber:
      'key p-value-max, group sato-apex-one-inner-width: 0.001 ratio; printed < 0.001*. Right-versus-left ' +
      'comparison for this measurement and region; strictly less than the printed bound. Cohort: 57 patients, 49 ' +
      'female and 8 male; Lenke 1 (25) or Lenke 2 (32), 684 pedicles. Enrolled ages 12–20, Table 1 age 16.2 ± ' +
      '2.4; 10–20 is the abstract inclusion range. The paper states right is always convex and left always ' +
      'concave. Per-cell observation counts are not supplied.',
    population:
      'sato-2022: Japanese AIS, enrolled ages 12–20 (inclusion range 10–20), mean age 16.2; 49 girls and 8 boys; ' +
      'Lenke 1/2 right main thoracic curves; 0.5 mm CT. Right is always convex and left always concave. Sample: ' +
      '57 adolescents; 684 pedicles; 15 patients in the 26.3 percent group. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Sato T, Nojiri H, Okuda T, Miyagawa K, Kobayashi N, Takahashi R, Shimura A, Tamagawa S, Ohara Y, Hara T, ' +
          'Ishijima M. 2022. Three-dimensional morphological analysis of the thoracic pedicle and related ' +
          'radiographic factors in adolescent idiopathic scoliosis. BMC Musculoskelet Disord 23(1):847.',
        pmid: '36068555',
        doi: '10.1186/s12891-022-05799-4',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-inner-widths-sato-2022-p-value',
    feature: 'Inner pedicle widths from population tables — p value',
    publishedNumber:
      'key p-value, group sato-neutral-one-inner-width: 0.903 ratio; printed 0.903. Right-versus-left comparison ' +
      'for this measurement and region; Exact p-value as printed. Cohort: 57 patients, 49 female and 8 male; ' +
      'Lenke 1 (25) or Lenke 2 (32), 684 pedicles. Enrolled ages 12–20, Table 1 age 16.2 ± 2.4; 10–20 is the ' +
      'abstract inclusion range. The paper states right is always convex and left always concave. Per-cell ' +
      'observation counts are not supplied.',
    population:
      'sato-2022: Japanese AIS, enrolled ages 12–20 (inclusion range 10–20), mean age 16.2; 49 girls and 8 boys; ' +
      'Lenke 1/2 right main thoracic curves; 0.5 mm CT. Right is always convex and left always concave. Sample: ' +
      '57 adolescents; 684 pedicles; 15 patients in the 26.3 percent group. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Sato T, Nojiri H, Okuda T, Miyagawa K, Kobayashi N, Takahashi R, Shimura A, Tamagawa S, Ohara Y, Hara T, ' +
          'Ishijima M. 2022. Three-dimensional morphological analysis of the thoracic pedicle and related ' +
          'radiographic factors in adolescent idiopathic scoliosis. BMC Musculoskelet Disord 23(1):847.',
        pmid: '36068555',
        doi: '10.1186/s12891-022-05799-4',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-length-rules-balikci-2019-chord-penetration',
    feature: 'Screw length beside the published chord and cortex limits — chord penetration',
    publishedNumber:
      'key chord-penetration, group krag-cited: 80 to 100 percent. Krag primary not opened; the note reports a ' +
      'cited range. Choosing 0.8 is a later rule. Series n = 120, ages 13–54 (mean 19.2): AIS 86, congenital 17, ' +
      'adult 10 and neuromuscular 7; not an exclusively AIS cohort.',
    population:
      'balikci-2019: Mixed scoliosis series: AIS 86, congenital 17, adult 10, neuromuscular 7; ages 13–54, mean ' +
      '19.2. Diameter chosen using insertional torque. Sample: 120 patients, 86 with AIS; 2,846 screws. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Balikci T, Kıyak G, Heydar AM, Bawaneh MK, Bezer M. 2019. Mid-length Pedicle Screws in Posterior ' +
          'Instrumentation of Scoliosis. Asian Spine J 13(5):815-822.',
        pmid: '31079434',
        doi: '10.31616/asj.2018.0177',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-length-rules-zheng-2009-ap-penetration',
    feature: 'Screw length beside the published chord and cortex limits — ap penetration',
    publishedNumber:
      'key ap-penetration, group children: 50 to 80 percent. Published range only; 0.65 is a project choice and ' +
      'is absent here.',
    population:
      'zheng-2009: Chinese children aged 5–14 from a hospital registry, stratified random sampling with thoracic ' +
      'abnormalities excluded; not scoliosis-specific; 2 mm axial CT. Sample: 120 children aged 5–14; number per ' +
      'age band and sex is not stated. [peer-reviewed; full-text].',
    sources: [
      {
        citation: 'Zheng C, Huang Q, Hu Y, Wang X, Chen W. 2009. Int Orthop 33(6):1663-1668.',
        pmid: '18956181',
        doi: '10.1007/s00264-008-0675-z',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-length-rules-balikci-2019-usual-length',
    feature: 'Screw length beside the published chord and cortex limits — usual length',
    publishedNumber:
      'key usual-length, group balikci-2019-t1-t5: 25 to 30 mm. key usual-length, group balikci-2019-t6-t12: 30 ' +
      'to 35 mm. key usual-length, group balikci-2019-l1-l5: 35 to 40 mm. Secondary: Balikci cites Kuraishi, J ' +
      'Spinal Disord Tech 2013, reference 25. These regional usual lengths duplicate the kuraishi-2013 rows and ' +
      'are not an independent observation. Series n = 120, ages 13–54 (mean 19.2): AIS 86, congenital 17, adult ' +
      '10 and neuromuscular 7; not an exclusively AIS cohort.',
    population:
      'balikci-2019: Mixed scoliosis series: AIS 86, congenital 17, adult 10, neuromuscular 7; ages 13–54, mean ' +
      '19.2. Diameter chosen using insertional torque. Sample: 120 patients, 86 with AIS; 2,846 screws. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Balikci T, Kıyak G, Heydar AM, Bawaneh MK, Bezer M. 2019. Mid-length Pedicle Screws in Posterior ' +
          'Instrumentation of Scoliosis. Asian Spine J 13(5):815-822.',
        pmid: '31079434',
        doi: '10.31616/asj.2018.0177',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-length-rules-kuraishi-2013-usual-length',
    feature: 'Screw length beside the published chord and cortex limits — usual length',
    publishedNumber:
      'key usual-length, group kuraishi-2013-t1-t5: 25 to 30 mm. key usual-length, group kuraishi-2013-t6-t12: ' +
      '30 to 35 mm. key usual-length, group kuraishi-2013-l1-l5: 35 to 40 mm.',
    population:
      'kuraishi-2013: Right thoracic AIS; CT navigation. Sample: 15 patients. [conference-abstract; abstract].',
    sources: [
      {
        citation:
          'Kuraishi S, Takahashi J, Hirabayashi H, Ogihara N, Mukaiyama K, Shimizu M, Kato H. 2013. Pedicle ' +
          'morphology using CT-based navigation system in adolescent idiopathic scoliosis Scoliosis 8(Suppl 1):O27.',
        pmid: null,
        doi: '10.1186/1748-7161-8-S1-O27',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-length-rules-sarwahi-2017-anterior-breach-threshold',
    feature: 'Screw length beside the published chord and cortex limits — anterior breach threshold',
    publishedNumber:
      'key anterior-breach-threshold, group aorta: 4 mm. Source threshold is breach depth of 4 mm or less; ' +
      'reusing it as a pre-cortex margin is a later project rule. Abstract population: 165 spine-deformity ' +
      'patients (2800 screws) plus 8 cadavers (285 screws); not an exclusively AIS cohort.',
    population:
      'sarwahi-2017: Patients with spine deformity plus cadavers, assessed for anterior or anterolateral screw ' +
      'breach and proximity to vital structures. Sample: 165 patients with 2,800 screws; 8 cadavers with 285 ' +
      'screws. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Sarwahi V, Payares M, Wendolowski S, Gecelter R, Maguire K, Wang D, Thornhill B, Amaral T. 2017. Spine ' +
          '(Phila Pa 1976) 42(22):E1305-E1310.',
        pmid: '28296814',
        doi: '10.1097/brs.0000000000002153',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-length-rules-sarwahi-2017-aorta-contact-depth',
    feature: 'Screw length beside the published chord and cortex limits — aorta contact depth',
    publishedNumber:
      'key aorta-contact-depth, group aorta: 5.7 ± 0.6 (spread statistic not stated) mm. The note prints ± ' +
      'without naming either statistic; the spread is not labelled SD or SE. Abstract population: 165 ' +
      'spine-deformity patients (2800 screws) plus 8 cadavers (285 screws); not an exclusively AIS cohort.',
    population:
      'sarwahi-2017: Patients with spine deformity plus cadavers, assessed for anterior or anterolateral screw ' +
      'breach and proximity to vital structures. Sample: 165 patients with 2,800 screws; 8 cadavers with 285 ' +
      'screws. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Sarwahi V, Payares M, Wendolowski S, Gecelter R, Maguire K, Wang D, Thornhill B, Amaral T. 2017. Spine ' +
          '(Phila Pa 1976) 42(22):E1305-E1310.',
        pmid: '28296814',
        doi: '10.1097/brs.0000000000002153',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-length-rules-sarwahi-2017-non-contact-depth',
    feature: 'Screw length beside the published chord and cortex limits — non contact depth',
    publishedNumber:
      'key non-contact-depth, group aorta: 3.9 ± 0.2 (spread statistic not stated) mm. The note prints ± without ' +
      'naming either statistic; the spread is not labelled SD or SE. Abstract population: 165 spine-deformity ' +
      'patients (2800 screws) plus 8 cadavers (285 screws); not an exclusively AIS cohort.',
    population:
      'sarwahi-2017: Patients with spine deformity plus cadavers, assessed for anterior or anterolateral screw ' +
      'breach and proximity to vital structures. Sample: 165 patients with 2,800 screws; 8 cadavers with 285 ' +
      'screws. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Sarwahi V, Payares M, Wendolowski S, Gecelter R, Maguire K, Wang D, Thornhill B, Amaral T. 2017. Spine ' +
          '(Phila Pa 1976) 42(22):E1305-E1310.',
        pmid: '28296814',
        doi: '10.1097/brs.0000000000002153',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-length-rules-balikci-2019-anterior-protrusion-limit',
    feature: 'Screw length beside the published chord and cortex limits — anterior protrusion limit',
    publishedNumber:
      'key anterior-protrusion-limit, group balikci-cortex: 1 count. No more than about 1 thread past the ' +
      'anterior cortical surface, as printed in P4; §4 says not >1 thread. Count is threads, not millimetres; ' +
      'pitch is not supplied and no length conversion is made. Series n = 120, ages 13–54 (mean 19.2): AIS 86, ' +
      'congenital 17, adult 10 and neuromuscular 7; not an exclusively AIS cohort.',
    population:
      'balikci-2019: Mixed scoliosis series: AIS 86, congenital 17, adult 10, neuromuscular 7; ages 13–54, mean ' +
      '19.2. Diameter chosen using insertional torque. Sample: 120 patients, 86 with AIS; 2,846 screws. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Balikci T, Kıyak G, Heydar AM, Bawaneh MK, Bezer M. 2019. Mid-length Pedicle Screws in Posterior ' +
          'Instrumentation of Scoliosis. Asian Spine J 13(5):815-822.',
        pmid: '31079434',
        doi: '10.31616/asj.2018.0177',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-length-rules-kuraishi-2013-full-paper-usual-length',
    feature: 'Screw length beside the published chord and cortex limits — usual length',
    publishedNumber:
      'key usual-length, group kuraishi-2013-full-paper-t1-t5: 25, 30 mm; printed 25 or 30 mm. key usual-length, ' +
      'group kuraishi-2013-full-paper-t6-t12: 30, 35 mm; printed 30 or 35 mm. key usual-length, group ' +
      'kuraishi-2013-full-paper-l1-l5: 35, 40 mm; printed 35 or 40 mm. Two published feasible lengths, stated as ' +
      '"or"; no intervening length or per-level assignment is inferred. Fifteen right-side thoracic AIS ' +
      'patients. The full paper and kuraishi-2013 meeting abstract are records of the same work and are not ' +
      'independent.',
    population:
      'kuraishi-2013-full-paper: Fifteen patients with right-side thoracic adolescent idiopathic scoliosis who ' +
      'underwent pedicle screw fixation. Sample: 15. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Kuraishi S, Takahashi J, Hirabayashi H, Hashidate H, Ogihara N, Mukaiyama K, Kato H. 2013. Pedicle ' +
          'morphology using computed tomography-based navigation system in adolescent idiopathic scoliosis. J Spinal ' +
          'Disord Tech 26(1):22-8.',
        pmid: '21959835',
        doi: '10.1097/BSD.0b013e31823162ef',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-level-rule-thresholds-lenke-2001-sagittal-modifier',
    feature: 'Level selection and curve criteria — sagittal modifier',
    publishedNumber:
      'key normal-kyphosis-min, group sagittal-modifier: 10 degrees. key normal-kyphosis-max, group ' +
      'sagittal-modifier: 40 degrees. T5–T12 normal band includes endpoints.',
    population:
      'lenke-2001: AIS classification using standing coronal/lateral and supine side-bending films. Sample: ' +
      '27-patient radiographic reliability set; 5 developers and 7 independent surgeons. [peer-reviewed; ' +
      'abstract].',
    sources: [
      {
        citation:
          'Lenke LG, Betz RR, Harms J, Bridwell KH, Clements DH, Lowe TG, Blanke K. 2001. Adolescent idiopathic ' +
          'scoliosis: a new classification to determine extent of spinal arthrodesis. J Bone Joint Surg Am ' +
          '83(8):1169-1181.',
        pmid: '11507125',
        doi: null,
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-level-rule-thresholds-korean-level-review-selective-thoracic',
    feature: 'Level selection and curve criteria — selective thoracic',
    publishedNumber:
      'key cobb-ratio-min, group selective-thoracic: 1.2 ratio. key translation-ratio-min, group ' +
      'selective-thoracic: 1.2 ratio. key rotation-ratio-min, group selective-thoracic: 1.2 ratio. key ' +
      'bending-cobb-max, group selective-thoracic: 25 degrees. key kyphosis-t10-l2-max, group ' +
      'selective-thoracic: 10 degrees. key cobb-max, group selective-thoracic: 60 degrees. Strictly greater ' +
      'than; review citing Lenke 2003, ratio absent from primary abstract. Strictly less than.',
    population:
      'korean-level-review: AIS selective-fusion review; primary populations differ by criterion. Sample: ' +
      'Review; aggregate n not supplied. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Kim DH, Hyun SJ, Kim KJ. 2021. Selection of Fusion Level for Adolescent Idiopathic Scoliosis Surgery : ' +
          'Selective Fusion versus Postoperative Decompensation. J Korean Neurosurg Soc 64(4):473-485.',
        pmid: '34044493',
        doi: '10.3340/jkns.2020.0258',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-level-rule-thresholds-korean-level-review-selective-lumbar',
    feature: 'Level selection and curve criteria — selective lumbar',
    publishedNumber:
      'key cobb-ratio-min, group selective-lumbar: 1.25 ratio. key translation-ratio-min, group ' +
      'selective-lumbar: 1.25 ratio. key rotation-ratio-min, group selective-lumbar: 1.25 ratio. key ' +
      'thoracic-cobb-max, group selective-lumbar: 40 degrees. key tll-bending-cobb-max, group selective-lumbar: ' +
      '25 degrees. key kyphosis-t10-l2-max, group selective-lumbar: 10 degrees. Strictly greater than. Strictly ' +
      'less than; kyphosis condition applies if UIV is between T10 and L2. Exceptions and maturity conditions in ' +
      'note are not encoded as rules here. Strictly less than 25 degrees. The selective-lumbar table prints TL/L ' +
      'side bending <25 degrees; the old notes read it as thoracic bending. The project rule uses the thoracic ' +
      'curve bending criterion as its own inference under DECISIONS, not as a statement printed in this review. ' +
      'The table also allows >25 degrees if more criteria are met.',
    population:
      'korean-level-review: AIS selective-fusion review; primary populations differ by criterion. Sample: ' +
      'Review; aggregate n not supplied. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Kim DH, Hyun SJ, Kim KJ. 2021. Selection of Fusion Level for Adolescent Idiopathic Scoliosis Surgery : ' +
          'Selective Fusion versus Postoperative Decompensation. J Korean Neurosurg Soc 64(4):473-485.',
        pmid: '34044493',
        doi: '10.3340/jkns.2020.0258',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-level-rule-thresholds-korean-level-review-sanders-selective-lumbar',
    feature: 'Level selection and curve criteria — sanders selective lumbar',
    publishedNumber:
      'key thoracic-bending-cobb-max, group sanders-selective-lumbar: 20 degrees. key cobb-ratio-min, group ' +
      'sanders-selective-lumbar: 1.25 ratio. Thoracic curve bends to 20 degrees or less (inclusive); Sanders ' +
      'criterion read through Kim 2021. Paired with TL/L-to-thoracic Cobb ratio 1.25 or greater (inclusive) and ' +
      'closed triradiate cartilage. This differs from the review text listing ratios >1.25. TL/L-to-thoracic ' +
      'Cobb ratio 1.25 or greater (inclusive), paired with thoracic bending 20 degrees or less and closed ' +
      'triradiate cartilage. Sanders is quoted through Kim 2021, not a separately opened primary; the review ' +
      'text separately listing >1.25.',
    population:
      'korean-level-review: AIS selective-fusion review; primary populations differ by criterion. Sample: ' +
      'Review; aggregate n not supplied. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Kim DH, Hyun SJ, Kim KJ. 2021. Selection of Fusion Level for Adolescent Idiopathic Scoliosis Surgery : ' +
          'Selective Fusion versus Postoperative Decompensation. J Korean Neurosurg Soc 64(4):473-485.',
        pmid: '34044493',
        doi: '10.3340/jkns.2020.0258',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-level-rule-thresholds-liu-2020-suk-neutral',
    feature: 'Level selection and curve criteria — suk neutral',
    publishedNumber:
      'key nv-ev-max, group suk-neutral: 1 levels. key nv-ev-min, group suk-neutral: 2 levels. key ' +
      'liv-above-neutral, group suk-neutral: 1 levels. NV ≤ EV+1 gives LIV=NV. Primarily Suk 2003, n=42, read ' +
      'through Liu. NV ≥ EV+2 gives LIV=NV−1. Primarily Suk 2003, n=42, read through Liu. NV−1 when NV ≥ EV+2; ' +
      'Suk 2003 reported through Liu.',
    population:
      'liu-2020: Lenke 1A/2A thoracic-only fusions. Sample: 6 studies; 732 patients; 150 with adding-on; ' +
      'per-reference denominators retained in the table. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Liu CW, Lenke LG, Tan LA, Oh T, Chao KH, Lin SD, Pan RY. 2020. Selection of the Lowest Instrumented ' +
          'Vertebra and Relative Odds Ratio of Distal Adding-on for Lenke Type 1A and 2A Curves in Adolescent ' +
          'Idiopathic Scoliosis: A Systematic Review and Meta-analysis. Neurospine 17(4):902-909.',
        pmid: '33401869',
        doi: '10.14245/ns.2040234.117',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-level-rule-thresholds-bmc-level-review-lenke-ab',
    feature: 'Level selection and curve criteria — lenke ab',
    publishedNumber:
      'key liv-above-stable, group lenke-ab: 1 to 2 levels. Published SV−1 or SV−2. SYNTHESIS chooses SV−1; that ' +
      'choice is not made here.',
    population:
      'bmc-level-review: AIS level-selection review; primary populations differ by rule. Sample: Review; ' +
      'aggregate n not supplied. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Chen R, Xi Y, Wang T, Wang A, Ma Z, Liang M, Yuan S, Zang L, Fan N. 2026. Advances in fusion level ' +
          'selection and surgical approaches for adolescent idiopathic scoliosis based on the Lenke classification ' +
          'system: a narrative review. BMC Surg 26(1):102.',
        pmid: '41492130',
        doi: '10.1186/s12893-025-03481-9',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-level-rule-thresholds-chen-2025-liv-index',
    feature: 'Level selection and curve criteria — liv index',
    publishedNumber:
      'key liv-index-max, group liv-index: 0 levels. Strictly less than 0; a published zero threshold, not a ' +
      'substitute for a missing value.',
    population:
      'chen-2025: Lenke 1–2 with LIV at or below LTV/LSTV; derivation cohort only. Sample: 112 patients in the ' +
      'full cohort; 10 with adding-on; under-15 subgroup n not supplied. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Chen IH, Chen CW, Huang CC, Hsu JY, Wang PY, Lee YF, Yeh YC, Lai PL, Hu MH, Yang SH. 2026. Lowest ' +
          'Instrumented Vertebra (LIV) Index as a Supplemental Indicator to Lastly Touched Vertebra for Selection of ' +
          'the Fusion Level in Main Thoracic Adolescent Idiopathic Scoliosis. Global Spine J 16(2):956-964.',
        pmid: '40820306',
        doi: '10.1177/21925682251366988',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-level-rule-thresholds-liu-2020-deviated-vertebra',
    feature: 'Level selection and curve criteria — deviated vertebra',
    publishedNumber:
      'key first-cephalad-deviation-min, group deviated-vertebra: 10 mm. Strictly greater than 10 mm from the ' +
      'CSVL. Choose as LIV the first vertebra encountered in the cephalad direction from the sacrum with that ' +
      'deviation, n = 45. This is the deviated-vertebra lower-end rule; the threshold belongs to the proposed ' +
      'LIV itself, not LIV+1, and is not a check on a candidate pair.',
    population:
      'liu-2020: Lenke 1A/2A thoracic-only fusions. Sample: 6 studies; 732 patients; 150 with adding-on; ' +
      'per-reference denominators retained in the table. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Liu CW, Lenke LG, Tan LA, Oh T, Chao KH, Lin SD, Pan RY. 2020. Selection of the Lowest Instrumented ' +
          'Vertebra and Relative Odds Ratio of Distal Adding-on for Lenke Type 1A and 2A Curves in Adolescent ' +
          'Idiopathic Scoliosis: A Systematic Review and Meta-analysis. Neurospine 17(4):902-909.',
        pmid: '33401869',
        doi: '10.14245/ns.2040234.117',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-level-rule-thresholds-bmc-level-review-lenke-l3',
    feature: 'Level selection and curve criteria — lenke l3',
    publishedNumber:
      'key rotation-max, group lenke-l3: 1.5 nash-moe-grade. key apex-at-or-above, group lenke-l3: L1/L2 levels. ' +
      'Nash–Moe grade ≤1.5, not radians or degrees. Grade mapping absent. At L1–2disc or cephalad; other ' +
      'disc/rotation criteria are required.',
    population:
      'bmc-level-review: AIS level-selection review; primary populations differ by rule. Sample: Review; ' +
      'aggregate n not supplied. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Chen R, Xi Y, Wang T, Wang A, Ma Z, Liang M, Yuan S, Zang L, Fan N. 2026. Advances in fusion level ' +
          'selection and surgical approaches for adolescent idiopathic scoliosis based on the Lenke classification ' +
          'system: a narrative review. BMC Surg 26(1):102.',
        pmid: '41492130',
        doi: '10.1186/s12893-025-03481-9',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-level-rule-thresholds-bmc-level-review-lenke-l4',
    feature: 'Level selection and curve criteria — lenke l4',
    publishedNumber:
      'key rotation-min, group lenke-l4: 1 nash-moe-grade. key apex-at-or-below, group lenke-l4: L2 to L2 ' +
      'levels. Nash–Moe grade ≥I, not radians or degrees. Grade mapping absent. At L2 or caudad; point level ' +
      'expressed without a numerical level-index conversion.',
    population:
      'bmc-level-review: AIS level-selection review; primary populations differ by rule. Sample: Review; ' +
      'aggregate n not supplied. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Chen R, Xi Y, Wang T, Wang A, Ma Z, Liang M, Yuan S, Zang L, Fan N. 2026. Advances in fusion level ' +
          'selection and surgical approaches for adolescent idiopathic scoliosis based on the Lenke classification ' +
          'system: a narrative review. BMC Surg 26(1):102.',
        pmid: '41492130',
        doi: '10.1186/s12893-025-03481-9',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-level-rule-thresholds-bmc-level-review-lev-plus-one-5c',
    feature: 'Level selection and curve criteria — lev plus one 5c',
    publishedNumber:
      'key tll-cobb-min, group lev-plus-one-5c: 60 degrees. key bending-disc-angle, group lev-plus-one-5c: 0 ' +
      'degrees. key rotation-min, group lev-plus-one-5c: 2 nash-moe-grade. key liv-below-end, group ' +
      'lev-plus-one-5c: 1 levels. Strictly greater than. Disc cannot reach 0 degrees on bending; not an angular ' +
      'tolerance. Li criterion: strictly greater than Nash–Moe grade 2; primary not opened. LEV+1 when published ' +
      'criteria hold; Yang/Li primaries not opened.',
    population:
      'bmc-level-review: AIS level-selection review; primary populations differ by rule. Sample: Review; ' +
      'aggregate n not supplied. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Chen R, Xi Y, Wang T, Wang A, Ma Z, Liang M, Yuan S, Zang L, Fan N. 2026. Advances in fusion level ' +
          'selection and surgical approaches for adolescent idiopathic scoliosis based on the Lenke classification ' +
          'system: a narrative review. BMC Surg 26(1):102.',
        pmid: '41492130',
        doi: '10.1186/s12893-025-03481-9',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-level-rule-thresholds-liu-2020-end-plus-one',
    feature: 'Level selection and curve criteria — end plus one',
    publishedNumber:
      'key liv-below-end, group end-plus-one: 1 levels. EV+1 reference landmark; no candidate or preference ' +
      'chosen.',
    population:
      'liu-2020: Lenke 1A/2A thoracic-only fusions. Sample: 6 studies; 732 patients; 150 with adding-on; ' +
      'per-reference denominators retained in the table. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Liu CW, Lenke LG, Tan LA, Oh T, Chao KH, Lin SD, Pan RY. 2020. Selection of the Lowest Instrumented ' +
          'Vertebra and Relative Odds Ratio of Distal Adding-on for Lenke Type 1A and 2A Curves in Adolescent ' +
          'Idiopathic Scoliosis: A Systematic Review and Meta-analysis. Neurospine 17(4):902-909.',
        pmid: '33401869',
        doi: '10.14245/ns.2040234.117',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-level-rule-thresholds-liu-2020-substantially-touched',
    feature: 'Level selection and curve criteria — substantially touched',
    publishedNumber:
      'key liv-below-nstv, group substantially-touched: 1 levels. nSTV+1; Qin 2016 reported through Liu, n = ' +
      '104.',
    population:
      'liu-2020: Lenke 1A/2A thoracic-only fusions. Sample: 6 studies; 732 patients; 150 with adding-on; ' +
      'per-reference denominators retained in the table. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Liu CW, Lenke LG, Tan LA, Oh T, Chao KH, Lin SD, Pan RY. 2020. Selection of the Lowest Instrumented ' +
          'Vertebra and Relative Odds Ratio of Distal Adding-on for Lenke Type 1A and 2A Curves in Adolescent ' +
          'Idiopathic Scoliosis: A Systematic Review and Meta-analysis. Neurospine 17(4):902-909.',
        pmid: '33401869',
        doi: '10.14245/ns.2040234.117',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-level-rule-thresholds-bmc-level-review-uev-5c',
    feature: 'Level selection and curve criteria — uev 5c',
    publishedNumber:
      'key uiv-above-uev, group uev-5c: 1 levels. Posterior approach allows UEV or one above; no preference ' +
      'selected.',
    population:
      'bmc-level-review: AIS level-selection review; primary populations differ by rule. Sample: Review; ' +
      'aggregate n not supplied. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Chen R, Xi Y, Wang T, Wang A, Ma Z, Liang M, Yuan S, Zang L, Fan N. 2026. Advances in fusion level ' +
          'selection and surgical approaches for adolescent idiopathic scoliosis based on the Lenke classification ' +
          'system: a narrative review. BMC Surg 26(1):102.',
        pmid: '41492130',
        doi: '10.1186/s12893-025-03481-9',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-level-rule-thresholds-bmc-level-review-kaya-selective-thoracic',
    feature: 'Level selection and curve criteria — kaya selective thoracic',
    publishedNumber:
      'key lumbar-flexibility-min, group kaya-selective-thoracic: 70 percent. key lumbar-rotation-max, group ' +
      'kaya-selective-thoracic: 2 nash-moe-grade. Strictly greater than 70 percent; lumbar bending flexibility. ' +
      'Read through the BMC review; Kaya 2022 primary was not opened, and primary sample size is not supplied. ' +
      'Less than or equal to Nash–Moe grade 2; lumbar AVR. Paired with lumbar bending flexibility >70 percent. ' +
      'Kaya 2022 primary was not opened; no degree-to-grade mapping is inferred.',
    population:
      'bmc-level-review: AIS level-selection review; primary populations differ by rule. Sample: Review; ' +
      'aggregate n not supplied. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Chen R, Xi Y, Wang T, Wang A, Ma Z, Liang M, Yuan S, Zang L, Fan N. 2026. Advances in fusion level ' +
          'selection and surgical approaches for adolescent idiopathic scoliosis based on the Lenke classification ' +
          'system: a narrative review. BMC Surg 26(1):102.',
        pmid: '41492130',
        doi: '10.1186/s12893-025-03481-9',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-level-rule-thresholds-s1-liv-42811138-adding-on',
    feature: 'Level selection and curve criteria — adding on',
    publishedNumber:
      'key s1-liv-angle-min, group adding-on: 9.5 degrees. Strictly greater than 9.5 degrees; Lenke 1–2, A/B ' +
      'modifiers. Derivation n=177, external n=289. The abstract does not supply the angle geometry; this record ' +
      'cannot define a measurement algorithm.',
    population:
      's1-liv-42811138: Lenke 1–2, lumbar modifiers A/B; adding-on analysis. Sample: Derivation 177 patients; ' +
      'external validation 289 patients. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Ohrt-Nissen S, Cheung JPY, Cheung PWH, Kuang XT, Ragborg L, Heegaard M, Andersen TB, Gehrchen M, Dahl B. ' +
          '2026. The S1-LIV angle is essential for fusion selection in thoracic adolescent idiopathic scoliosis - a ' +
          'dual-center machine learning study with external validation. Eur Spine J',
        pmid: '42811138',
        doi: '10.1007/s00586-026-10407-3',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-level-rule-thresholds-ssv-40103372-djk',
    feature: 'Level selection and curve criteria — djk',
    publishedNumber:
      'key liv-above-ssv-max, group djk: 1 levels. LIV no more proximal than SSV−1: at most one level above SSV, ' +
      'and below preoperative distal junctional kyphosis. Lenke 1–2, LIV≤L1; n=1034. Offset direction is ' +
      'proximal, not a rule implementation.',
    population:
      'ssv-40103372: Lenke 1–2 with LIV ≤ L1; DJK and sagittal stable vertebra analysis. Sample: 1,034 patients; ' +
      'multicentre. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Hori Y, Matsumura A, Namikawa T, Isogai N, Almeida da Silva LC, Kaymaz B, Yorgova PK, Gabos PG, Fletcher ' +
          'ND, Kelly MP, Shufflebarger HL, Newton PO, Yaszay B, Sponseller PD, Lonner BS, Samdani AF, Miyanji F, ' +
          'Shah SA. 2025. Spine (Phila Pa 1976)',
        pmid: '40103372',
        doi: '10.1097/brs.0000000000005336',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-level-rule-thresholds-djk-34096362-ssv-excluded-djk',
    feature: 'Level selection and curve criteria — ssv excluded djk',
    publishedNumber:
      'key postoperative-kyphosis-t5-t12-min, group ssv-excluded-djk: 25 degrees. key ' +
      'postoperative-kyphosis-t11-l2-min, group ssv-excluded-djk: 10 degrees. Greater than or equal to 25 ' +
      'degrees, postoperative T5–T12, with SSV excluded. The published test joins the two regional criteria with ' +
      'OR. Lenke 1–2, LIV≤L1; whole cohort n=122, subgroup n=13 in the note; no patient-specific rate is ' +
      'inferred. Greater than or equal to 10 degrees, postoperative T11–L2, with SSV excluded. The published ' +
      'test joins the two regional criteria with OR. Lenke 1–2, LIV≤L1; whole cohort n=122, subgroup n=13 in the ' +
      'note; no patient-specific rate is inferred.',
    population:
      'djk-34096362: Lenke 1–2 with LIV ≤ L1; sagittal stable vertebra inclusion and postoperative kyphosis. ' +
      'Sample: 122 patients; SSV excluded 42, SSV included 80; postoperative threshold subgroup 13. ' +
      '[peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Wang PY, Chen CW, Lee YF, Hu MH, Wang TM, Lai PL, Yang SH. 2023. Distal Junctional Kyphosis after ' +
          'Posterior Spinal Fusion in Lenke 1 and 2 Adolescent Idiopathic Scoliosis-Exploring Detailed Features of ' +
          'the Sagittal Stable Vertebra Concept. Global Spine J 13(4):1112-1119.',
        pmid: '34096362',
        doi: '10.1177/21925682211019692',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-level-rule-thresholds-l3-bending-30739071-l3-on-concave-bend',
    feature: 'Level selection and curve criteria — l3 on concave bend',
    publishedNumber:
      'key l3-bending-translation-max, group l3-on-concave-bend: 10 mm. key lumbar-cobb-min, group ' +
      'l3-on-concave-bend: 60 degrees. Strictly less than 10 mm on the preoperative concave side-bending film, ' +
      'for lumbar curves >60 degrees. n=84 (24 L3, 60 L4); this is a published criterion, not a choice made by ' +
      'the reader. Strictly greater than 60 degrees; eligibility condition for the L3 concave-bending ' +
      'translation criterion, not the separate LEV+1 rule.',
    population:
      'l3-bending-30739071: Lumbar curves greater than 60°; preoperative concave side-bending L3 translation. ' +
      'Sample: 84 patients: 24 ending at L3; 60 at L4. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Qin X, He Z, Yin R, Qiu Y, Zhu Z. 2019. Where to stop distally in Lenke modifier C AIS with lumbar curve ' +
          'more than 60°: L3 or L4? Clin Neurol Neurosurg 178:77-81.',
        pmid: '30739071',
        doi: '10.1016/j.clineuro.2019.02.005',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-level-rule-thresholds-bmc-level-review-lenke-1-left-shoulder-high',
    feature: 'Level selection and curve criteria — lenke 1 left shoulder high',
    publishedNumber:
      'key uiv-options, group lenke-1-left-shoulder-high: T2 to T2 levels. Preoperative left shoulder high, ' +
      'Lenke type 1. Preserve the published named option(s), without choosing one. Secondary rule citing Lenke ' +
      '2007 / Rose & Lenke 2007; primaries not opened. Contested by Brooks 2018. Shoulders are outside this ' +
      'spine model; storing the rule does not make it readable here.',
    population:
      'bmc-level-review: AIS level-selection review; primary populations differ by rule. Sample: Review; ' +
      'aggregate n not supplied. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Chen R, Xi Y, Wang T, Wang A, Ma Z, Liang M, Yuan S, Zang L, Fan N. 2026. Advances in fusion level ' +
          'selection and surgical approaches for adolescent idiopathic scoliosis based on the Lenke classification ' +
          'system: a narrative review. BMC Surg 26(1):102.',
        pmid: '41492130',
        doi: '10.1186/s12893-025-03481-9',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-level-rule-thresholds-bmc-level-review-lenke-1-level-shoulders',
    feature: 'Level selection and curve criteria — lenke 1 level shoulders',
    publishedNumber:
      'key uiv-options, group lenke-1-level-shoulders: T3 to T4 levels. Preoperative shoulders level, Lenke type ' +
      '1. Preserve the published named option(s), without choosing one. Secondary rule citing Lenke 2007 / Rose ' +
      '& Lenke 2007; primaries not opened. Contested by Brooks 2018. Shoulders are outside this spine model; ' +
      'storing the rule does not make it readable here.',
    population:
      'bmc-level-review: AIS level-selection review; primary populations differ by rule. Sample: Review; ' +
      'aggregate n not supplied. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Chen R, Xi Y, Wang T, Wang A, Ma Z, Liang M, Yuan S, Zang L, Fan N. 2026. Advances in fusion level ' +
          'selection and surgical approaches for adolescent idiopathic scoliosis based on the Lenke classification ' +
          'system: a narrative review. BMC Surg 26(1):102.',
        pmid: '41492130',
        doi: '10.1186/s12893-025-03481-9',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-level-rule-thresholds-bmc-level-review-lenke-1-right-shoulder-high',
    feature: 'Level selection and curve criteria — lenke 1 right shoulder high',
    publishedNumber:
      'key uiv-options, group lenke-1-right-shoulder-high: T4 to T5 levels. Preoperative right shoulder high, ' +
      'Lenke type 1. Preserve the published named option(s), without choosing one. Secondary rule citing Lenke ' +
      '2007 / Rose & Lenke 2007; primaries not opened. Contested by Brooks 2018. Shoulders are outside this ' +
      'spine model; storing the rule does not make it readable here.',
    population:
      'bmc-level-review: AIS level-selection review; primary populations differ by rule. Sample: Review; ' +
      'aggregate n not supplied. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Chen R, Xi Y, Wang T, Wang A, Ma Z, Liang M, Yuan S, Zang L, Fan N. 2026. Advances in fusion level ' +
          'selection and surgical approaches for adolescent idiopathic scoliosis based on the Lenke classification ' +
          'system: a narrative review. BMC Surg 26(1):102.',
        pmid: '41492130',
        doi: '10.1186/s12893-025-03481-9',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-level-rule-thresholds-bmc-level-review-pt-behaves-structural',
    feature: 'Level selection and curve criteria — pt behaves structural',
    publishedNumber:
      'key pt-bending-cobb-min, group pt-behaves-structural: 15 degrees. Greater than or equal to 15 degrees on ' +
      'cervical supine side-bending, behaving like a structural PT curve. Review citing Chan 2016, primary not ' +
      'opened; this is distinct from the 25-degree classification criterion.',
    population:
      'bmc-level-review: AIS level-selection review; primary populations differ by rule. Sample: Review; ' +
      'aggregate n not supplied. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Chen R, Xi Y, Wang T, Wang A, Ma Z, Liang M, Yuan S, Zang L, Fan N. 2026. Advances in fusion level ' +
          'selection and surgical approaches for adolescent idiopathic scoliosis based on the Lenke classification ' +
          'system: a narrative review. BMC Surg 26(1):102.',
        pmid: '41492130',
        doi: '10.1186/s12893-025-03481-9',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-level-rule-thresholds-residual-t1-39658742-residual-t1-tilt',
    feature: 'Level selection and curve criteria — residual t1 tilt',
    publishedNumber:
      'key preoperative-pt-cobb-min, group residual-t1-tilt: 26.5 degrees. key residual-t1-tilt-min, group ' +
      'residual-t1-tilt: 5 degrees. key follow-up, group residual-t1-tilt: 2 years. Strictly greater than 26.5 ' +
      'degrees; preoperative PT Cobb is associated with T1 tilt >5 degrees at two years in Lenke 1, n=128. ' +
      'Strictly greater than 5 degrees at two years; residual T1 tilt result, not a preoperative PT Cobb ' +
      'threshold. Lenke 1, n=128. Two-year follow-up for the residual T1 tilt >5-degree observation; no ' +
      'interpolation over time.',
    population:
      'residual-t1-39658742: Lenke 1; residual T1 tilt and self-image analysis. Sample: 128 patients; 44 with T1 ' +
      'tilt greater than 5° at two years. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Banno T, Yamato Y, Hasegawa T, Yoshida G, Arima H, Oe S, Ide K, Yamada T, Kurosu K, Matsuyama Y. 2025. ' +
          'Residual T1 tilt could lead to poor self-image for patients with Lenke type 1 adolescent idiopathic ' +
          'scoliosis. Spine Deform 13(3):811-819.',
        pmid: '39658742',
        doi: '10.1007/s43390-024-01020-9',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-level-rule-thresholds-shoulder-36138576-lenke-1-shoulder-imbalance',
    feature: 'Level selection and curve criteria — lenke 1 shoulder imbalance',
    publishedNumber:
      'key preoperative-rsh-min, group lenke-1-shoulder-imbalance: -6.5 mm. key mt-correction-min, group ' +
      'lenke-1-shoulder-imbalance: 76.9 percent. Strictly greater than −6.5 mm preoperative RSH, meaning less ' +
      'right-shoulder elevation. Negative sign preserved; n=104, Lenke 1. Strictly greater than 76.9 percent ' +
      'planned main-thoracic correction; n=104, Lenke 1. result association only, not a requested correction ' +
      'target.',
    population:
      'shoulder-36138576: Lenke 1; shoulder imbalance and main-thoracic overcorrection. Sample: 104 patients; 21 ' +
      'with imbalance. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Banno T, Yamato Y, Hasegawa T, Yoshida G, Arima H, Oe S, Mihara Y, Ide K, Watanabe Y, Kurosu K, Nakai K, ' +
          'Matsuyama Y. 2023. Preoperative Less Right Shoulder Elevation Had a Higher Risk of Postoperative Shoulder ' +
          'Imbalance When Main Thoracic Curve Shows Higher Correction Regardless of the Upper Instrumented Vertebra ' +
          'Level for Patients with Adolescent Idiopathic Scoliosis Lenke Type 1. Asian Spine J 17(1):166-175.',
        pmid: '36138576',
        doi: '10.31616/asj.2022.0020',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-level-rule-thresholds-zhao-2024-pjk-definition',
    feature: 'Level selection and curve criteria — pjk definition',
    publishedNumber:
      'key absolute-pja-min, group pjk-definition: 10 degrees. key pja-increase-min, group pjk-definition: 10 ' +
      'degrees. Greater than or equal to 10 degrees for absolute PJA from the lower endplate of UIV to the upper ' +
      'endplate of UIV+2. This AND the distinct increase-from-preoperative criterion define PJK in the cited ' +
      'meta-analysis. Greater than or equal to 10 degrees increase over preoperative PJA. This AND the distinct ' +
      'absolute-angle criterion define PJK; an increase is not an absolute PJA.',
    population:
      'zhao-2024: AIS proximal junctional kyphosis meta-analysis. Sample: 24 studies; stated total 4,063; 718 ' +
      'with PJK and 3,885 without; patients per hook comparison not given. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Zhao J, Huang C, Liu Y, Liu D, Liao D. 2024. Systematic review and meta-analysis for the proximal ' +
          'junctional kyphosis in adolescent idiopathic scoliosis. Front Pediatr 12:1387841.',
        pmid: '39205666',
        doi: '10.3389/fped.2024.1387841',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-level-rule-thresholds-coronal-decompensation-4417731-coronal-decompensation',
    feature: 'Level selection and curve criteria — coronal decompensation',
    publishedNumber:
      'key c7pl-csvl-min, group coronal-decompensation: 20 mm. Strictly greater than 20 mm for C7PL−CSVL. Review ' +
      'definition; full text read in part. This is a result definition, not a substitute for a measured ' +
      'CSVL or a patient-specific result.',
    population:
      'coronal-decompensation-4417731: Selective fusion; coronal decompensation defined in the opened article by ' +
      'C7PL–CSVL >20 mm. Sample: Selective thoracic subgroup 16; selective TL/L subgroup 14; full sample not ' +
      'supplied. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Studer D, Awais A, Williams N, Antoniou G, Eardley-Harris N, Cundy P. 2015. Selective fusion in ' +
          'adolescent idiopathic scoliosis: a radiographic evaluation of risk factors for imbalance. J Child Orthop ' +
          '9(2):153-160.',
        pmid: '25845647',
        doi: '10.1007/s11832-015-0653-0',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-level-rule-thresholds-liu-2020-suk-neutral-summary',
    feature: 'Level selection and curve criteria — suk neutral summary',
    publishedNumber:
      'key nv-ev-summary-interval, group suk-neutral-summary: 0 to 1 levels. Table B prints 0–1 levels for the ' +
      'first branch (same level or one below). The detailed rule and SYNTHESIS say NV≤EV+1 and provide no ' +
      'separate lower bound; this summary interval must not exclude a more proximal NV from that detailed ' +
      'branch. Suk 2003 n=42, secondary within Liu.',
    population:
      'liu-2020: Lenke 1A/2A thoracic-only fusions. Sample: 6 studies; 732 patients; 150 with adding-on; ' +
      'per-reference denominators retained in the table. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Liu CW, Lenke LG, Tan LA, Oh T, Chao KH, Lin SD, Pan RY. 2020. Selection of the Lowest Instrumented ' +
          'Vertebra and Relative Odds Ratio of Distal Adding-on for Lenke Type 1A and 2A Curves in Adolescent ' +
          'Idiopathic Scoliosis: A Systematic Review and Meta-analysis. Neurospine 17(4):902-909.',
        pmid: '33401869',
        doi: '10.14245/ns.2040234.117',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-medial-angles-zheng-2009-transverse-angle-summary',
    feature: 'Pedicle direction from published angles — transverse angle summary',
    publishedNumber:
      'key transverse-angle-summary, group zheng-p6-t1, level T1: 31 degrees. key transverse-angle-summary, ' +
      'group zheng-p6-t2, level T2: 20 degrees. key transverse-angle-summary, group zheng-p6-t3, level T3: 13 ' +
      'degrees. key transverse-angle-summary, group zheng-p6-t4-t10: 4 to 8 degrees. key ' +
      'transverse-angle-summary, group zheng-p6-t11-t12: -6 to -4 degrees. Approximately 31° at T1, as printed ' +
      'in the research note’s P6 summary; separate from exact Table 2 sex columns. No specific age-band or sex ' +
      'allocation is supplied. Approximately 20° at T2, as printed in the research note’s P6 summary; separate ' +
      'from exact Table 2 sex columns. No specific age-band or sex allocation is supplied. Approximately 13° at ' +
      'T3, as printed in the research note’s P6 summary; separate from exact Table 2 sex columns. No specific ' +
      'age-band or sex allocation is supplied. Approximately 4–8° across T4–T10, as printed; one regional range ' +
      'with no single-level, age-band or sex allocation. Printed endpoint order: approximately −4 to −6° at ' +
      'T11–T12. Interpreting negative angles as divergent is [own inference]. Lower/upper are ordered interval ' +
      'bounds. One regional range, with no single-level, age-band or sex allocation.',
    population:
      'zheng-2009: Chinese children aged 5–14 from a hospital registry, stratified random sampling with thoracic ' +
      'abnormalities excluded; not scoliosis-specific; 2 mm axial CT. Sample: 120 children aged 5–14; number per ' +
      'age band and sex is not stated. [peer-reviewed; full-text].',
    sources: [
      {
        citation: 'Zheng C, Huang Q, Hu Y, Wang X, Chen W. 2009. Int Orthop 33(6):1663-1668.',
        pmid: '18956181',
        doi: '10.1007/s00264-008-0675-z',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-medial-angles-upendra-2010-transverse-angle-summary',
    feature: 'Pedicle direction from published angles — transverse angle summary',
    publishedNumber:
      'key transverse-angle-summary, group upendra-p6-ais: 12 to 26 degrees. Published AIS summary 12–26°, ' +
      'concave larger than convex. The summary does not allocate endpoints to a side or individual level; exact ' +
      'Table 3 columns stay separate.',
    population:
      'upendra-2010: AIS; 14 girls and 10 boys; all right thoracic or thoracolumbar curves (King II 12, III 7, ' +
      'IV 5); ages 12.3–18.3; Cobb 58–98°; 3 mm CT. Sample: 24 patients; width n 213 convex/223 concave; angle n ' +
      '217 convex/216 concave; chord n 213 on each side; per-cell pedicle counts retained. [peer-reviewed; ' +
      'full-text].',
    sources: [
      {
        citation:
          'Upendra B, Meena D, Kandwal P, Ahmed A, Chowdhury B, Jayaswal A. 2010. Pedicle morphometry in patients ' +
          'with adolescent idiopathic scoliosis. Indian J Orthop 44(2):169-176.',
        pmid: '20419004',
        doi: '10.4103/0019-5413.62084',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-medial-angles-upendra-2010-transverse-angle',
    feature: 'Pedicle direction from published angles — transverse angle',
    publishedNumber:
      'key transverse-angle, level T1, curveSide convex, group upendra-ais: 20.67 ± 4.66 SD degrees (range 16.0 ' +
      'to 28.0); n = 9; printed 20.67. key transverse-angle, level T1, curveSide concave, group upendra-ais: ' +
      '21.33 ± 5.63 SD degrees (range 15.0 to 30.0); n = 9; printed 21.33. key transverse-angle, level T2, ' +
      'curveSide convex, group upendra-ais: 17.67 ± 4.96 SD degrees (range 14.0 to 25.0); n = 11; printed 17.67. ' +
      'key transverse-angle, level T2, curveSide concave, group upendra-ais: 20.7 ± 4.36 SD degrees (range 16.0 ' +
      'to 27.0); n = 13; printed 20.7. key transverse-angle, level T3, curveSide convex, group upendra-ais: ' +
      '12.67 ± 6.63 SD degrees (range 7.0 to 21.0); n = 13; printed 12.67. key transverse-angle, level T3, ' +
      'curveSide concave, group upendra-ais: 16.0 ± 4.23 SD degrees (range 9.0 to 20.0); n = 11; printed 16. key ' +
      'transverse-angle, level T4, curveSide convex, group upendra-ais: 14.6 ± 8.14 SD degrees (range 0.0 to ' +
      '20.0); n = 16; printed 14.6. key transverse-angle, level T4, curveSide concave, group upendra-ais: 17.75 ' +
      '± 5.32 SD degrees (range 5.0 to 25.0); n = 15; printed 17.75. key transverse-angle, level T5, curveSide ' +
      'convex, group upendra-ais: 18.25 ± 5.1 SD degrees (range 10.0 to 26.0); n = 9; printed 18.25. key ' +
      'transverse-angle, level T5, curveSide concave, group upendra-ais: 19.67 ± 8.65 SD degrees (range 10.0 to ' +
      '29.0); n = 9; printed 19.67. key transverse-angle, level T6, curveSide convex, group upendra-ais: 14.5 ± ' +
      '5.96 SD degrees (range 4.0 to 24.0); n = 13; printed 14.5. key transverse-angle, level T6, curveSide ' +
      'concave, group upendra-ais: 20.75 ± 9.11 SD degrees (range 13.0 to 34.0); n = 13; printed 20.75. key ' +
      'transverse-angle, level T7, curveSide convex, group upendra-ais: 17.0 ± 2.33 SD degrees (range 15.0 to ' +
      '20.0); n = 11; printed 17. key transverse-angle, level T7, curveSide concave, group upendra-ais: 26.0 ± ' +
      '1.14 SD degrees (range 20.0 to 31.0); n = 11; printed 26.0; doubtful. key transverse-angle, level T8, ' +
      'curveSide convex, group upendra-ais: 15.0 ± 4.69 SD degrees (range 5.0 to 26.0); n = 14; printed 15. key ' +
      'transverse-angle, level T8, curveSide concave, group upendra-ais: 15.67 ± 9.68 SD degrees (range 3.0 to ' +
      '28.0); n = 17; printed 15.67. key transverse-angle, level T9, curveSide convex, group upendra-ais: 15.33 ' +
      '± 2.89 SD degrees (range 10.0 to 18.0); n = 12; printed 15.33. key transverse-angle, level T9, curveSide ' +
      'concave, group upendra-ais: 19.5 ± 3.34 SD degrees (range 14.0 to 22.5); n = 12; printed 19.5. key ' +
      'transverse-angle, level T10, curveSide convex, group upendra-ais: 12.4 ± 3.66 SD degrees (range 7.0 to ' +
      '19.0); n = 15; printed 12.4. key transverse-angle, level T10, curveSide concave, group upendra-ais: 12.8 ' +
      '± 8.24 SD degrees (range 4.0 to 24.0); n = 15; printed 12.8. key transverse-angle, level T11, curveSide ' +
      'convex, group upendra-ais: 11.0 ± 6.93 SD degrees (range 3.0 to 19.0); n = 16; printed 11. key ' +
      'transverse-angle, level T11, curveSide concave, group upendra-ais: 12.0 ± 6.52 SD degrees (range 4.0 to ' +
      '18.0); n = 16; printed 12.0. key transverse-angle, level T12, curveSide convex, group upendra-ais: 13.56 ' +
      '± 9.98 SD degrees (range 0.0 to 30.0); n = 19; printed 13.56. key transverse-angle, level T12, curveSide ' +
      'concave, group upendra-ais: 14.8 ± 6.59 SD degrees (range 5.0 to 24.0); n = 16; printed 14.8. key ' +
      'transverse-angle, level L1, curveSide convex, group upendra-ais: 16.38 ± 8.74 SD degrees (range 0.0 to ' +
      '26.0); n = 19; printed 16.38. key transverse-angle, level L1, curveSide concave, group upendra-ais: 16.6 ' +
      '± 6.56 SD degrees (range 4.0 to 26.0); n = 19; printed 16.6. key transverse-angle, level L2, curveSide ' +
      'convex, group upendra-ais: 16.75 ± 9.66 SD degrees (range 5.0 to 35.0); n = 13; printed 16.75. key ' +
      'transverse-angle, level L2, curveSide concave, group upendra-ais: 17.6 ± 5.18 SD degrees (range 4.0 to ' +
      '23.0); n = 13; printed 17.6. key transverse-angle, level L3, curveSide convex, group upendra-ais: 12.8 ± ' +
      '5.73 SD degrees (range 5.0 to 21.0); n = 11; printed 12.8. key transverse-angle, level L3, curveSide ' +
      'concave, group upendra-ais: 16.0 ± 6.21 SD degrees (range 8.0 to 23.0); n = 11; printed 16. key ' +
      'transverse-angle, level L4, curveSide convex, group upendra-ais: 17.8 ± 2.66 SD degrees (range 18.0 to ' +
      '24.0); n = 9; printed 17.8; retained only, excluded from use; impossible. key transverse-angle, level L4, ' +
      'curveSide concave, group upendra-ais: 18.7 ± 4.43 SD degrees (range 12.0 to 24.0); n = 9; printed 18.7. ' +
      'key transverse-angle, level L5, curveSide convex, group upendra-ais: 18.0 ± 2.12 SD degrees (range 15.0 ' +
      'to 22.0); n = 7; printed 18. key transverse-angle, level L5, curveSide concave, group upendra-ais: 24.0 ± ' +
      '4.98 SD degrees (range 19.0 to 28.0); n = 7; printed 24. key transverse-angle, level T10, curveSide ' +
      'concave, group upendra-table-4: 12.0 degrees; printed 12.0; retained only, excluded from use; ambiguous ' +
      'print. key transverse-angle, level L1, curveSide concave, group upendra-table-4: 16.0 degrees; printed ' +
      '16.0; retained only, excluded from use; ambiguous print. key transverse-angle, level L4, curveSide ' +
      'convex, group upendra-table-4: 21.0 degrees; printed 21; retained only, excluded from use; ambiguous ' +
      'print. key transverse-angle, level L4, curveSide concave, group upendra-table-4: 16.7 degrees; printed ' +
      '16.7; retained only, excluded from use; ambiguous print. Printed Table 2 mean, SD, per-cell pedicle count ' +
      'and range; no rounding or interpolation. [own inference] Convex is read as right and concave as left ' +
      'because all 24 patients had right thoracic/thoracolumbar curves; the paper does not explicitly equate ' +
      'convex with right. Printed Table 2 mean, SD, per-cell pedicle count and range; no rounding or ' +
      'interpolation. [own inference] Convex is read as right and concave as left because all 24 patients had ' +
      'right thoracic/thoracolumbar curves; the paper does not explicitly equate convex with right. Table 4(b) ' +
      'repeats this cell as 12.0, disagreeing with Table 2’s 12.8; this primary cell follows Table 2. Printed ' +
      'Table 2 mean, SD, per-cell pedicle count and range; no rounding or interpolation. [own inference] Convex ' +
      'is read as right and concave as left because all 24 patients had right thoracic/thoracolumbar curves; the ' +
      'paper does not explicitly equate convex with right. Table 4(b) repeats this cell as 16.0, disagreeing ' +
      'with Table 2’s 16.6; this primary cell follows Table 2. Printed Table 2 mean, SD, per-cell pedicle count ' +
      'and range; no rounding or interpolation. [own inference] Convex is read as right and concave as left ' +
      'because all 24 patients had right thoracic/thoracolumbar curves; the paper does not explicitly equate ' +
      'convex with right. Table 4(b) repeats this cell as 21, disagreeing with Table 2’s 17.8; this primary cell ' +
      'follows Table 2. Printed Table 2 mean, SD, per-cell pedicle count and range; no rounding or ' +
      'interpolation. [own inference] Convex is read as right and concave as left because all 24 patients had ' +
      'right thoracic/thoracolumbar curves; the paper does not explicitly equate convex with right. Table 4(b) ' +
      'repeats this cell as 16.7, disagreeing with Table 2’s 18.7; this primary cell follows Table 2. ' +
      'Alternative figure retained only as printed, separate from primary Tables 1–3. [own inference] Convex is ' +
      'read as right and concave as left because all 24 patients had right thoracic/thoracolumbar curves; the ' +
      'paper does not explicitly equate convex with right.',
    population:
      'upendra-2010: AIS; 14 girls and 10 boys; all right thoracic or thoracolumbar curves (King II 12, III 7, ' +
      'IV 5); ages 12.3–18.3; Cobb 58–98°; 3 mm CT. Sample: 24 patients; width n 213 convex/223 concave; angle n ' +
      '217 convex/216 concave; chord n 213 on each side; per-cell pedicle counts retained. [peer-reviewed; ' +
      'full-text].',
    sources: [
      {
        citation:
          'Upendra B, Meena D, Kandwal P, Ahmed A, Chowdhury B, Jayaswal A. 2010. Pedicle morphometry in patients ' +
          'with adolescent idiopathic scoliosis. Indian J Orthop 44(2):169-176.',
        pmid: '20419004',
        doi: '10.4103/0019-5413.62084',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-medial-angles-zheng-2009-transverse-angle',
    feature: 'Pedicle direction from published angles — transverse angle',
    publishedNumber:
      'key transverse-angle, level T1, ageBand 5-8, sex male, group zheng-children: 30.94 ± 5.25 (spread ' +
      'statistic not stated) degrees; printed 30.94 ± 5.25. key transverse-angle, level T1, ageBand 5-8, sex ' +
      'female, group zheng-children: 29.42 ± 5.77 (spread statistic not stated) degrees; printed 29.42 ± 5.77. ' +
      'key transverse-angle, level T1, ageBand 9-11, sex male, group zheng-children: 30.83 ± 4.55 (spread ' +
      'statistic not stated) degrees; printed 30.83 ± 4.55. key transverse-angle, level T1, ageBand 9-11, sex ' +
      'female, group zheng-children: 31.16 ± 3.71 (spread statistic not stated) degrees; printed 31.16 ± 3.71. ' +
      'key transverse-angle, level T1, ageBand 12-14, sex male, group zheng-children: 30.69 ± 4.59 (spread ' +
      'statistic not stated) degrees; printed 30.69 ± 4.59. key transverse-angle, level T1, ageBand 12-14, sex ' +
      'female, group zheng-children: 31.4 ± 3.92 (spread statistic not stated) degrees; printed 31.40 ± 3.92. ' +
      'key transverse-angle, level T2, ageBand 5-8, sex male, group zheng-children: 20.14 ± 4.38 (spread ' +
      'statistic not stated) degrees; printed 20.14 ± 4.38. key transverse-angle, level T2, ageBand 5-8, sex ' +
      'female, group zheng-children: 20.89 ± 5.79 (spread statistic not stated) degrees; printed 20.89 ± 5.79. ' +
      'key transverse-angle, level T2, ageBand 9-11, sex male, group zheng-children: 21.73 ± 5.76 (spread ' +
      'statistic not stated) degrees; printed 21.73 ± 5.76. key transverse-angle, level T2, ageBand 9-11, sex ' +
      'female, group zheng-children: 23.17 ± 4.11 (spread statistic not stated) degrees; printed 23.17 ± 4.11. ' +
      'key transverse-angle, level T2, ageBand 12-14, sex male, group zheng-children: 20.19 ± 4.02 (spread ' +
      'statistic not stated) degrees; printed 20.19 ± 4.02. key transverse-angle, level T2, ageBand 12-14, sex ' +
      'female, group zheng-children: 18.76 ± 6.22 (spread statistic not stated) degrees; printed 18.76 ± 6.22. ' +
      'key transverse-angle, level T3, ageBand 5-8, sex male, group zheng-children: 13.56 ± 3.15 (spread ' +
      'statistic not stated) degrees; printed 13.56 ± 3.15. key transverse-angle, level T3, ageBand 5-8, sex ' +
      'female, group zheng-children: 13.72 ± 5.73 (spread statistic not stated) degrees; printed 13.72 ± 5.73. ' +
      'key transverse-angle, level T3, ageBand 9-11, sex male, group zheng-children: 13.44 ± 5.18 (spread ' +
      'statistic not stated) degrees; printed 13.44 ± 5.18. key transverse-angle, level T3, ageBand 9-11, sex ' +
      'female, group zheng-children: 15.45 ± 6.62 (spread statistic not stated) degrees; printed 15.45 ± 6.62. ' +
      'key transverse-angle, level T3, ageBand 12-14, sex male, group zheng-children: 13.17 ± 4.3 (spread ' +
      'statistic not stated) degrees; printed 13.17 ± 4.30. key transverse-angle, level T3, ageBand 12-14, sex ' +
      'female, group zheng-children: 12.57 ± 4.57 (spread statistic not stated) degrees; printed 12.57 ± 4.57. ' +
      'key transverse-angle, level T4, ageBand 5-8, sex male, group zheng-children: 9.55 ± 2.84 (spread ' +
      'statistic not stated) degrees; printed 9.55 ± 2.84. key transverse-angle, level T4, ageBand 5-8, sex ' +
      'female, group zheng-children: 11.09 ± 5.17 (spread statistic not stated) degrees; printed 11.09 ± 5.17. ' +
      'key transverse-angle, level T4, ageBand 9-11, sex male, group zheng-children: 9.27 ± 3.95 (spread ' +
      'statistic not stated) degrees; printed 9.27 ± 3.95. key transverse-angle, level T4, ageBand 9-11, sex ' +
      'female, group zheng-children: 8.23 ± 4.38 (spread statistic not stated) degrees; printed 8.23 ± 4.38. key ' +
      'transverse-angle, level T4, ageBand 12-14, sex male, group zheng-children: 7.41 ± 3.45 (spread statistic ' +
      'not stated) degrees; printed 7.41 ± 3.45. key transverse-angle, level T4, ageBand 12-14, sex female, ' +
      'group zheng-children: 7.57 ± 3.11 (spread statistic not stated) degrees; printed 7.57 ± 3.11. key ' +
      'transverse-angle, level T5, ageBand 5-8, sex male, group zheng-children: 8.23 ± 2.86 (spread statistic ' +
      'not stated) degrees; printed 8.23 ± 2.86. key transverse-angle, level T5, ageBand 5-8, sex female, group ' +
      'zheng-children: 8.82 ± 3.81 (spread statistic not stated) degrees; printed 8.82 ± 3.81. key ' +
      'transverse-angle, level T5, ageBand 9-11, sex male, group zheng-children: 8.24 ± 3.81 (spread statistic ' +
      'not stated) degrees; printed 8.24 ± 3.81. key transverse-angle, level T5, ageBand 9-11, sex female, group ' +
      'zheng-children: 8.24 ± 3.67 (spread statistic not stated) degrees; printed 8.24 ± 3.67. key ' +
      'transverse-angle, level T5, ageBand 12-14, sex male, group zheng-children: 6.46 ± 2.99 (spread statistic ' +
      'not stated) degrees; printed 6.46 ± 2.99. key transverse-angle, level T5, ageBand 12-14, sex female, ' +
      'group zheng-children: 6.3 ± 3.45 (spread statistic not stated) degrees; printed 6.30 ± 3.45. key ' +
      'transverse-angle, level T6, ageBand 5-8, sex male, group zheng-children: 7.68 ± 2.83 (spread statistic ' +
      'not stated) degrees; printed 7.68 ± 2.83. key transverse-angle, level T6, ageBand 5-8, sex female, group ' +
      'zheng-children: 8.21 ± 2.31 (spread statistic not stated) degrees; printed 8.21 ± 2.31. key ' +
      'transverse-angle, level T6, ageBand 9-11, sex male, group zheng-children: 6.6 ± 4.36 (spread statistic ' +
      'not stated) degrees; printed 6.60 ± 4.36. key transverse-angle, level T6, ageBand 9-11, sex female, group ' +
      'zheng-children: 6.01 ± 3.5 (spread statistic not stated) degrees; printed 6.01 ± 3.50. key ' +
      'transverse-angle, level T6, ageBand 12-14, sex male, group zheng-children: 5.14 ± 3.32 (spread statistic ' +
      'not stated) degrees; printed 5.14 ± 3.32. key transverse-angle, level T6, ageBand 12-14, sex female, ' +
      'group zheng-children: 5.23 ± 1.95 (spread statistic not stated) degrees; printed 5.23 ± 1.95. key ' +
      'transverse-angle, level T7, ageBand 5-8, sex male, group zheng-children: 6.68 ± 2.47 (spread statistic ' +
      'not stated) degrees; printed 6.68 ± 2.47. key transverse-angle, level T7, ageBand 5-8, sex female, group ' +
      'zheng-children: 7.29 ± 2.71 (spread statistic not stated) degrees; printed 7.29 ± 2.71. key ' +
      'transverse-angle, level T7, ageBand 9-11, sex male, group zheng-children: 5.86 ± 2.97 (spread statistic ' +
      'not stated) degrees; printed 5.86 ± 2.97. key transverse-angle, level T7, ageBand 9-11, sex female, group ' +
      'zheng-children: 5.71 ± 3.27 (spread statistic not stated) degrees; printed 5.71 ± 3.27. key ' +
      'transverse-angle, level T7, ageBand 12-14, sex male, group zheng-children: 4.72 ± 2.38 (spread statistic ' +
      'not stated) degrees; printed 4.72 ± 2.38. key transverse-angle, level T7, ageBand 12-14, sex female, ' +
      'group zheng-children: 4.42 ± 2.05 (spread statistic not stated) degrees; printed 4.42 ± 2.05. key ' +
      'transverse-angle, level T8, ageBand 5-8, sex male, group zheng-children: 5.9 ± 2.25 (spread statistic not ' +
      'stated) degrees; printed 5.90 ± 2.25. key transverse-angle, level T8, ageBand 5-8, sex female, group ' +
      'zheng-children: 7.13 ± 2.89 (spread statistic not stated) degrees; printed 7.13 ± 2.89. key ' +
      'transverse-angle, level T8, ageBand 9-11, sex male, group zheng-children: 5.69 ± 2.52 (spread statistic ' +
      'not stated) degrees; printed 5.69 ± 2.52. key transverse-angle, level T8, ageBand 9-11, sex female, group ' +
      'zheng-children: 4.21 ± 2.9 (spread statistic not stated) degrees; printed 4.21 ± 2.90. key ' +
      'transverse-angle, level T8, ageBand 12-14, sex male, group zheng-children: 4.41 ± 2.91 (spread statistic ' +
      'not stated) degrees; printed 4.41 ± 2.91. key transverse-angle, level T8, ageBand 12-14, sex female, ' +
      'group zheng-children: 3.68 ± 1.75 (spread statistic not stated) degrees; printed 3.68 ± 1.75. key ' +
      'transverse-angle, level T9, ageBand 5-8, sex male, group zheng-children: 5.93 ± 1.7 (spread statistic not ' +
      'stated) degrees; printed 5.93 ± 1.70. key transverse-angle, level T9, ageBand 5-8, sex female, group ' +
      'zheng-children: 6.81 ± 2.25 (spread statistic not stated) degrees; printed 6.81 ± 2.25. key ' +
      'transverse-angle, level T9, ageBand 9-11, sex male, group zheng-children: 5.28 ± 2.68 (spread statistic ' +
      'not stated) degrees; printed 5.28 ± 2.68. key transverse-angle, level T9, ageBand 9-11, sex female, group ' +
      'zheng-children: 5.23 ± 3.61 (spread statistic not stated) degrees; printed 5.23 ± 3.61. key ' +
      'transverse-angle, level T9, ageBand 12-14, sex male, group zheng-children: 4.24 ± 2.91 (spread statistic ' +
      'not stated) degrees; printed 4.24 ± 2.91. key transverse-angle, level T9, ageBand 12-14, sex female, ' +
      'group zheng-children: 4.02 ± 2.07 (spread statistic not stated) degrees; printed 4.02 ± 2.07. key ' +
      'transverse-angle, level T10, ageBand 5-8, sex male, group zheng-children: 6.1 ± 2.33 (spread statistic ' +
      'not stated) degrees; printed 6.10 ± 2.33. key transverse-angle, level T10, ageBand 5-8, sex female, group ' +
      'zheng-children: 6.45 ± 3.35 (spread statistic not stated) degrees; printed 6.45±3.35; probable misprint. ' +
      'key transverse-angle, level T10, ageBand 9-11, sex male, group zheng-children: 5.07 ± 2.72 (spread ' +
      'statistic not stated) degrees; printed 5.07 ± 2.72. key transverse-angle, level T10, ageBand 9-11, sex ' +
      'female, group zheng-children: 6.06 ± 4.01 (spread statistic not stated) degrees; printed 6.06 ± 4.01. key ' +
      'transverse-angle, level T10, ageBand 12-14, sex male, group zheng-children: 3.94 ± 2.93 (spread statistic ' +
      'not stated) degrees; printed 3.94 ± 2.93. key transverse-angle, level T10, ageBand 12-14, sex female, ' +
      'group zheng-children: 4.77 ± 3.09 (spread statistic not stated) degrees; printed 4.77 ± 3.09. key ' +
      'transverse-angle, level T11, ageBand 5-8, sex male, group zheng-children: -5.34 ± 2.93 (spread statistic ' +
      'not stated) degrees; printed −5.34 ± 2.93. key transverse-angle, level T11, ageBand 5-8, sex female, ' +
      'group zheng-children: -6.37 ± 1.84 (spread statistic not stated) degrees; printed −6.37 ± 1.84. key ' +
      'transverse-angle, level T11, ageBand 9-11, sex male, group zheng-children: -5.6 ± 4.14 (spread statistic ' +
      'not stated) degrees; printed −5.60 ± 4.14. key transverse-angle, level T11, ageBand 9-11, sex female, ' +
      'group zheng-children: -3.95 ± 2.49 (spread statistic not stated) degrees; printed −3.95 ± 2.49. key ' +
      'transverse-angle, level T11, ageBand 12-14, sex male, group zheng-children: -3.8 ± 3.0 (spread statistic ' +
      'not stated) degrees; printed −3.80 ± 3.00. key transverse-angle, level T11, ageBand 12-14, sex female, ' +
      'group zheng-children: -4.72 ± 2.21 (spread statistic not stated) degrees; printed −4.72 ± 2.21. key ' +
      'transverse-angle, level T12, ageBand 5-8, sex male, group zheng-children: -5.01 ± 2.78 (spread statistic ' +
      'not stated) degrees; printed −5.01 ± 2.78. key transverse-angle, level T12, ageBand 5-8, sex female, ' +
      'group zheng-children: -5.54 ± 4.42 (spread statistic not stated) degrees; printed −5.54 ± 4.42. key ' +
      'transverse-angle, level T12, ageBand 9-11, sex male, group zheng-children: -4.78 ± 3.01 (spread statistic ' +
      'not stated) degrees; printed −4.78 ± 3.01. key transverse-angle, level T12, ageBand 9-11, sex female, ' +
      'group zheng-children: -4.31 ± 2.98 (spread statistic not stated) degrees; printed −4.31 ± 2.98. key ' +
      'transverse-angle, level T12, ageBand 12-14, sex male, group zheng-children: -4.48 ± 3.0 (spread statistic ' +
      'not stated) degrees; printed −4.48 ± 3.00. key transverse-angle, level T12, ageBand 12-14, sex female, ' +
      'group zheng-children: -4.26 ± 2.93 (spread statistic not stated) degrees; printed −4.26 ± 2.93. Full ' +
      'text. The paper does not state what ± denotes; reading it as a standard deviation is this project\'s own ' +
      'inference. Numbers of children per age band and sex are not stated. The paper does not define the angle ' +
      'sign. Reading positive as convergent and negative as divergent is this project\'s own inference.',
    population:
      'zheng-2009: Chinese children aged 5–14 from a hospital registry, stratified random sampling with thoracic ' +
      'abnormalities excluded; not scoliosis-specific; 2 mm axial CT. Sample: 120 children aged 5–14; number per ' +
      'age band and sex is not stated. [peer-reviewed; full-text].',
    sources: [
      {
        citation: 'Zheng C, Huang Q, Hu Y, Wang X, Chen W. 2009. Int Orthop 33(6):1663-1668.',
        pmid: '18956181',
        doi: '10.1007/s00264-008-0675-z',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-pedicle-heights-zheng-2009-sagittal-diameter',
    feature: 'Pedicle heights from population tables — sagittal diameter',
    publishedNumber:
      'key sagittal-diameter, level T1, ageBand 5-8, sex male, group zheng-children: 3.4 ± 0.52 (spread ' +
      'statistic not stated) mm; printed 3.40 ± 0.52. key sagittal-diameter, level T1, ageBand 5-8, sex female, ' +
      'group zheng-children: 3.43 ± 0.62 (spread statistic not stated) mm; printed 3.43 ± 0.62. key ' +
      'sagittal-diameter, level T1, ageBand 9-11, sex male, group zheng-children: 3.84 ± 0.44 (spread statistic ' +
      'not stated) mm; printed 3.84 ± 0.44. key sagittal-diameter, level T1, ageBand 9-11, sex female, group ' +
      'zheng-children: 4.04 ± 0.41 (spread statistic not stated) mm; printed 4.04 ± 0.41. key sagittal-diameter, ' +
      'level T1, ageBand 12-14, sex male, group zheng-children: 4.14 ± 0.54 (spread statistic not stated) mm; ' +
      'printed 4.14 ± 0.54. key sagittal-diameter, level T1, ageBand 12-14, sex female, group zheng-children: ' +
      '4.12 ± 0.79 (spread statistic not stated) mm; printed 4.12 ± 0.79. key sagittal-diameter, level T2, ' +
      'ageBand 5-8, sex male, group zheng-children: 3.56 ± 0.58 (spread statistic not stated) mm; printed 3.56 ± ' +
      '0.58. key sagittal-diameter, level T2, ageBand 5-8, sex female, group zheng-children: 3.64 ± 0.76 (spread ' +
      'statistic not stated) mm; printed 3.64 ± 0.76. key sagittal-diameter, level T2, ageBand 9-11, sex male, ' +
      'group zheng-children: 4.14 ± 0.38 (spread statistic not stated) mm; printed 4.14 ± 0.38. key ' +
      'sagittal-diameter, level T2, ageBand 9-11, sex female, group zheng-children: 4.32 ± 0.44 (spread ' +
      'statistic not stated) mm; printed 4.32 ± 0.44. key sagittal-diameter, level T2, ageBand 12-14, sex male, ' +
      'group zheng-children: 4.43 ± 0.58 (spread statistic not stated) mm; printed 4.43 ± 0.58. key ' +
      'sagittal-diameter, level T2, ageBand 12-14, sex female, group zheng-children: 4.45 ± 0.86 (spread ' +
      'statistic not stated) mm; printed 4.45 ± 0.86. key sagittal-diameter, level T3, ageBand 5-8, sex male, ' +
      'group zheng-children: 3.97 ± 0.74 (spread statistic not stated) mm; printed 3.97 ± 0.74. key ' +
      'sagittal-diameter, level T3, ageBand 5-8, sex female, group zheng-children: 4.07 ± 0.83 (spread statistic ' +
      'not stated) mm; printed 4.07 ± 0.83. key sagittal-diameter, level T3, ageBand 9-11, sex male, group ' +
      'zheng-children: 4.49 ± 0.63 (spread statistic not stated) mm; printed 4.49 ± 0.63. key sagittal-diameter, ' +
      'level T3, ageBand 9-11, sex female, group zheng-children: 4.52 ± 0.61 (spread statistic not stated) mm; ' +
      'printed 4.52 ± 0.61. key sagittal-diameter, level T3, ageBand 12-14, sex male, group zheng-children: 4.79 ' +
      '± 0.71 (spread statistic not stated) mm; printed 4.79 ± 0.71. key sagittal-diameter, level T3, ageBand ' +
      '12-14, sex female, group zheng-children: 4.93 ± 0.88 (spread statistic not stated) mm; printed 4.93 ± ' +
      '0.88. key sagittal-diameter, level T4, ageBand 5-8, sex male, group zheng-children: 4.23 ± 0.63 (spread ' +
      'statistic not stated) mm; printed 4.23 ± 0.63. key sagittal-diameter, level T4, ageBand 5-8, sex female, ' +
      'group zheng-children: 4.38 ± 0.8 (spread statistic not stated) mm; printed 4.38 ± 0.80. key ' +
      'sagittal-diameter, level T4, ageBand 9-11, sex male, group zheng-children: 5.14 ± 0.89 (spread statistic ' +
      'not stated) mm; printed 5.14 ± 0.89. key sagittal-diameter, level T4, ageBand 9-11, sex female, group ' +
      'zheng-children: 4.75 ± 0.55 (spread statistic not stated) mm; printed 4.75 ± 0.55. key sagittal-diameter, ' +
      'level T4, ageBand 12-14, sex male, group zheng-children: 5.27 ± 0.87 (spread statistic not stated) mm; ' +
      'printed 5.27 ± 0.87. key sagittal-diameter, level T4, ageBand 12-14, sex female, group zheng-children: ' +
      '5.34 ± 0.78 (spread statistic not stated) mm; printed 5.34 ± 0.78. key sagittal-diameter, level T5, ' +
      'ageBand 5-8, sex male, group zheng-children: 4.66 ± 0.7 (spread statistic not stated) mm; printed 4.66 ± ' +
      '0.70. key sagittal-diameter, level T5, ageBand 5-8, sex female, group zheng-children: 4.64 ± 0.89 (spread ' +
      'statistic not stated) mm; printed 4.64 ± 0.89. key sagittal-diameter, level T5, ageBand 9-11, sex male, ' +
      'group zheng-children: 5.66 ± 1.0 (spread statistic not stated) mm; printed 5.66 ± 1.00. key ' +
      'sagittal-diameter, level T5, ageBand 9-11, sex female, group zheng-children: 5.23 ± 0.83 (spread ' +
      'statistic not stated) mm; printed 5.23 ± 0.83. key sagittal-diameter, level T5, ageBand 12-14, sex male, ' +
      'group zheng-children: 5.82 ± 0.82 (spread statistic not stated) mm; printed 5.82 ± 0.82. key ' +
      'sagittal-diameter, level T5, ageBand 12-14, sex female, group zheng-children: 5.73 ± 1.08 (spread ' +
      'statistic not stated) mm; printed 5.73 ± 1.08. key sagittal-diameter, level T6, ageBand 5-8, sex male, ' +
      'group zheng-children: 4.93 ± 0.81 (spread statistic not stated) mm; printed 4.93 ± 0.81. key ' +
      'sagittal-diameter, level T6, ageBand 5-8, sex female, group zheng-children: 5.16 ± 0.91 (spread statistic ' +
      'not stated) mm; printed 5.16 ± 0.91. key sagittal-diameter, level T6, ageBand 9-11, sex male, group ' +
      'zheng-children: 6.04 ± 0.94 (spread statistic not stated) mm; printed 6.04 ± 0.94. key sagittal-diameter, ' +
      'level T6, ageBand 9-11, sex female, group zheng-children: 5.72 ± 0.79 (spread statistic not stated) mm; ' +
      'printed 5.72 ± 0.79. key sagittal-diameter, level T6, ageBand 12-14, sex male, group zheng-children: 6.43 ' +
      '± 1.02 (spread statistic not stated) mm; printed 6.43 ± 1.02. key sagittal-diameter, level T6, ageBand ' +
      '12-14, sex female, group zheng-children: 6.43 ± 1.0 (spread statistic not stated) mm; printed 6.43 ± ' +
      '1.00. key sagittal-diameter, level T7, ageBand 5-8, sex male, group zheng-children: 5.66 ± 0.93 (spread ' +
      'statistic not stated) mm; printed 5.66 ± 0.93. key sagittal-diameter, level T7, ageBand 5-8, sex female, ' +
      'group zheng-children: 5.67 ± 1.02 (spread statistic not stated) mm; printed 5.67 ± 1.02. key ' +
      'sagittal-diameter, level T7, ageBand 9-11, sex male, group zheng-children: 7.03 ± 0.86 (spread statistic ' +
      'not stated) mm; printed 7.03 ± 0.86. key sagittal-diameter, level T7, ageBand 9-11, sex female, group ' +
      'zheng-children: 6.49 ± 0.98 (spread statistic not stated) mm; printed 6.49 ± 0.98. key sagittal-diameter, ' +
      'level T7, ageBand 12-14, sex male, group zheng-children: 6.92 ± 1.01 (spread statistic not stated) mm; ' +
      'printed 6.92 ± 1.01. key sagittal-diameter, level T7, ageBand 12-14, sex female, group zheng-children: ' +
      '7.21 ± 1.16 (spread statistic not stated) mm; printed 7.21 ± 1.16. key sagittal-diameter, level T8, ' +
      'ageBand 5-8, sex male, group zheng-children: 6.13 ± 1.0 (spread statistic not stated) mm; printed 6.13 ± ' +
      '1.00. key sagittal-diameter, level T8, ageBand 5-8, sex female, group zheng-children: 6.36 ± 0.87 (spread ' +
      'statistic not stated) mm; printed 6.36 ± 0.87. key sagittal-diameter, level T8, ageBand 9-11, sex male, ' +
      'group zheng-children: 7.43 ± 1.07 (spread statistic not stated) mm; printed 7.43 ± 1.07. key ' +
      'sagittal-diameter, level T8, ageBand 9-11, sex female, group zheng-children: 7.1 ± 0.67 (spread statistic ' +
      'not stated) mm; printed 7.10 ± 0.67. key sagittal-diameter, level T8, ageBand 12-14, sex male, group ' +
      'zheng-children: 7.91 ± 1.1 (spread statistic not stated) mm; printed 7.91 ± 1.10. key sagittal-diameter, ' +
      'level T8, ageBand 12-14, sex female, group zheng-children: 7.71 ± 0.96 (spread statistic not stated) mm; ' +
      'printed 7.71 ± 0.96. key sagittal-diameter, level T9, ageBand 5-8, sex male, group zheng-children: 6.91 ± ' +
      '1.03 (spread statistic not stated) mm; printed 6.91 ± 1.03. key sagittal-diameter, level T9, ageBand 5-8, ' +
      'sex female, group zheng-children: 7.11 ± 0.84 (spread statistic not stated) mm; printed 7.11 ± 0.84. key ' +
      'sagittal-diameter, level T9, ageBand 9-11, sex male, group zheng-children: 7.98 ± 1.1 (spread statistic ' +
      'not stated) mm; printed 7.98 ± 1.10. key sagittal-diameter, level T9, ageBand 9-11, sex female, group ' +
      'zheng-children: 7.58 ± 0.87 (spread statistic not stated) mm; printed 7.58 ± 0.87. key sagittal-diameter, ' +
      'level T9, ageBand 12-14, sex male, group zheng-children: 8.77 ± 1.25 (spread statistic not stated) mm; ' +
      'printed 8.77 ± 1.25. key sagittal-diameter, level T9, ageBand 12-14, sex female, group zheng-children: ' +
      '8.53 ± 1.28 (spread statistic not stated) mm; printed 8.53 ± 1.28. key sagittal-diameter, level T10, ' +
      'ageBand 5-8, sex male, group zheng-children: 7.64 ± 1.03 (spread statistic not stated) mm; printed 7.64 ± ' +
      '1.03. key sagittal-diameter, level T10, ageBand 5-8, sex female, group zheng-children: 7.9 ± 0.86 (spread ' +
      'statistic not stated) mm; printed 7.90 ± 0.86. key sagittal-diameter, level T10, ageBand 9-11, sex male, ' +
      'group zheng-children: 8.76 ± 0.96 (spread statistic not stated) mm; printed 8.76 ± 0.96. key ' +
      'sagittal-diameter, level T10, ageBand 9-11, sex female, group zheng-children: 8.46 ± 0.97 (spread ' +
      'statistic not stated) mm; printed 8.46 ± 0.97. key sagittal-diameter, level T10, ageBand 12-14, sex male, ' +
      'group zheng-children: 10.02 ± 1.13 (spread statistic not stated) mm; printed 10.02 ± 1.13. key ' +
      'sagittal-diameter, level T10, ageBand 12-14, sex female, group zheng-children: 9.43 ± 1.55 (spread ' +
      'statistic not stated) mm; printed 9.43 ± 1.55. key sagittal-diameter, level T11, ageBand 5-8, sex male, ' +
      'group zheng-children: 8.34 ± 1.0 (spread statistic not stated) mm; printed 8.34 ± 1.00. key ' +
      'sagittal-diameter, level T11, ageBand 5-8, sex female, group zheng-children: 8.63 ± 1.06 (spread ' +
      'statistic not stated) mm; printed 8.63 ± 1.06. key sagittal-diameter, level T11, ageBand 9-11, sex male, ' +
      'group zheng-children: 9.73 ± 1.09 (spread statistic not stated) mm; printed 9.73 ± 1.09. key ' +
      'sagittal-diameter, level T11, ageBand 9-11, sex female, group zheng-children: 9.47 ± 1.1 (spread ' +
      'statistic not stated) mm; printed 9.47 ± 1.10. key sagittal-diameter, level T11, ageBand 12-14, sex male, ' +
      'group zheng-children: 11.21 ± 1.04 (spread statistic not stated) mm; printed 11.21 ± 1.04. key ' +
      'sagittal-diameter, level T11, ageBand 12-14, sex female, group zheng-children: 10.59 ± 1.79 (spread ' +
      'statistic not stated) mm; printed 10.59 ± 1.79. key sagittal-diameter, level T12, ageBand 5-8, sex male, ' +
      'group zheng-children: 9.39 ± 1.36 (spread statistic not stated) mm; printed 9.39 ± 1.36. key ' +
      'sagittal-diameter, level T12, ageBand 5-8, sex female, group zheng-children: 9.46 ± 1.32 (spread ' +
      'statistic not stated) mm; printed 9.46 ± 1.32. key sagittal-diameter, level T12, ageBand 9-11, sex male, ' +
      'group zheng-children: 10.51 ± 1.18 (spread statistic not stated) mm; printed 10.51 ± 1.18. key ' +
      'sagittal-diameter, level T12, ageBand 9-11, sex female, group zheng-children: 10.36 ± 1.12 (spread ' +
      'statistic not stated) mm; printed 10.36 ± 1.12. key sagittal-diameter, level T12, ageBand 12-14, sex ' +
      'male, group zheng-children: 12.35 ± 1.28 (spread statistic not stated) mm; printed 12.35 ± 1.28. key ' +
      'sagittal-diameter, level T12, ageBand 12-14, sex female, group zheng-children: 11.67 ± 1.67 (spread ' +
      'statistic not stated) mm; printed 11.67 ± 1.67. Full text. The paper does not state what ± denotes; ' +
      'reading it as a standard deviation is this project\'s own inference. Numbers of children per age band and ' +
      'sex are not stated.',
    population:
      'zheng-2009: Chinese children aged 5–14 from a hospital registry, stratified random sampling with thoracic ' +
      'abnormalities excluded; not scoliosis-specific; 2 mm axial CT. Sample: 120 children aged 5–14; number per ' +
      'age band and sex is not stated. [peer-reviewed; full-text].',
    sources: [
      {
        citation: 'Zheng C, Huang Q, Hu Y, Wang X, Chen W. 2009. Int Orthop 33(6):1663-1668.',
        pmid: '18956181',
        doi: '10.1007/s00264-008-0675-z',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-pedicle-heights-sato-2022-outer-height',
    feature: 'Pedicle heights from population tables — outer height',
    publishedNumber:
      'key outer-height, group sato-apex-one, curveSide convex: 12.4 ± 1.5 (spread statistic not stated) mm ' +
      '(range 9.1 to 16.3); printed 12.4 ± 1.5 (9.1–16.3). key outer-height, group sato-apex-one, curveSide ' +
      'concave: 10.8 ± 1.5 (spread statistic not stated) mm (range 7.3 to 15.6); printed 10.8 ± 1.5 (7.3–15.6). ' +
      'key outer-height, group sato-neutral-one, curveSide convex: 10.8 ± 1.5 (spread statistic not stated) mm ' +
      '(range 7.3 to 15.6); printed 10.8 ± 1.5 (7.3–15.6); retained only, excluded from use; probable copy. key ' +
      'outer-height, group sato-neutral-one, curveSide concave: 9.1 ± 1.0 (spread statistic not stated) mm ' +
      '(range 6.9 to 11.8); printed 9.1 ± 1.0 (6.9–11.8). Measurements are in a plane perpendicular to the ' +
      'pedicle axis through the isthmus. The table prints ± without defining its statistic. Cohort: 57 patients, ' +
      '49 female and 8 male; Lenke 1 (25) or Lenke 2 (32), 684 pedicles. Enrolled ages 12–20, Table 1 age 16.2 ± ' +
      '2.4; 10–20 is the abstract inclusion range. The paper states right is always convex and left always ' +
      'concave. Per-cell observation counts are not supplied.',
    population:
      'sato-2022: Japanese AIS, enrolled ages 12–20 (inclusion range 10–20), mean age 16.2; 49 girls and 8 boys; ' +
      'Lenke 1/2 right main thoracic curves; 0.5 mm CT. Right is always convex and left always concave. Sample: ' +
      '57 adolescents; 684 pedicles; 15 patients in the 26.3 percent group. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Sato T, Nojiri H, Okuda T, Miyagawa K, Kobayashi N, Takahashi R, Shimura A, Tamagawa S, Ohara Y, Hara T, ' +
          'Ishijima M. 2022. Three-dimensional morphological analysis of the thoracic pedicle and related ' +
          'radiographic factors in adolescent idiopathic scoliosis. BMC Musculoskelet Disord 23(1):847.',
        pmid: '36068555',
        doi: '10.1186/s12891-022-05799-4',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-pedicle-heights-sato-2022-p-value-max',
    feature: 'Pedicle heights from population tables — p value max',
    publishedNumber:
      'key p-value-max, group sato-apex-one-outer-height: 0.001 ratio; printed < 0.001*. key p-value-max, group ' +
      'sato-apex-one-inner-height: 0.001 ratio; printed < 0.001*. key p-value-max, group ' +
      'sato-neutral-one-outer-height: 0.001 ratio; printed < 0.001*. key p-value-max, group ' +
      'sato-neutral-one-inner-height: 0.001 ratio; printed < 0.001*. Right-versus-left comparison for this ' +
      'measurement and region; strictly less than the printed bound. Cohort: 57 patients, 49 female and 8 male; ' +
      'Lenke 1 (25) or Lenke 2 (32), 684 pedicles. Enrolled ages 12–20, Table 1 age 16.2 ± 2.4; 10–20 is the ' +
      'abstract inclusion range. The paper states right is always convex and left always concave. Per-cell ' +
      'observation counts are not supplied.',
    population:
      'sato-2022: Japanese AIS, enrolled ages 12–20 (inclusion range 10–20), mean age 16.2; 49 girls and 8 boys; ' +
      'Lenke 1/2 right main thoracic curves; 0.5 mm CT. Right is always convex and left always concave. Sample: ' +
      '57 adolescents; 684 pedicles; 15 patients in the 26.3 percent group. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Sato T, Nojiri H, Okuda T, Miyagawa K, Kobayashi N, Takahashi R, Shimura A, Tamagawa S, Ohara Y, Hara T, ' +
          'Ishijima M. 2022. Three-dimensional morphological analysis of the thoracic pedicle and related ' +
          'radiographic factors in adolescent idiopathic scoliosis. BMC Musculoskelet Disord 23(1):847.',
        pmid: '36068555',
        doi: '10.1186/s12891-022-05799-4',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-pedicle-heights-sato-2022-inner-height',
    feature: 'Pedicle heights from population tables — inner height',
    publishedNumber:
      'key inner-height, group sato-apex-one, curveSide convex: 7.5 ± 1.4 (spread statistic not stated) mm ' +
      '(range 4.9 to 11.4); printed 7.5 ± 1.4 (4.9–11.4). key inner-height, group sato-apex-one, curveSide ' +
      'concave: 5.9 ± 1.5 (spread statistic not stated) mm (range 1.9 to 10.1); printed 5.9 ± 1.5 (1.9–10.1). ' +
      'key inner-height, group sato-neutral-one, curveSide convex: 5.9 ± 1.5 (spread statistic not stated) mm ' +
      '(range 1.9 to 10.1); printed 5.9 ± 1.5 (1.9–10.1); retained only, excluded from use; probable copy. key ' +
      'inner-height, group sato-neutral-one, curveSide concave: 4.9 ± 1.2 (spread statistic not stated) mm ' +
      '(range 2.4 to 8.1); printed 4.9 ± 1.2 (2.4–8.1). Measurements are in a plane perpendicular to the pedicle ' +
      'axis through the isthmus. The table prints ± without defining its statistic. Cohort: 57 patients, 49 ' +
      'female and 8 male; Lenke 1 (25) or Lenke 2 (32), 684 pedicles. Enrolled ages 12–20, Table 1 age 16.2 ± ' +
      '2.4; 10–20 is the abstract inclusion range. The paper states right is always convex and left always ' +
      'concave. Per-cell observation counts are not supplied.',
    population:
      'sato-2022: Japanese AIS, enrolled ages 12–20 (inclusion range 10–20), mean age 16.2; 49 girls and 8 boys; ' +
      'Lenke 1/2 right main thoracic curves; 0.5 mm CT. Right is always convex and left always concave. Sample: ' +
      '57 adolescents; 684 pedicles; 15 patients in the 26.3 percent group. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Sato T, Nojiri H, Okuda T, Miyagawa K, Kobayashi N, Takahashi R, Shimura A, Tamagawa S, Ohara Y, Hara T, ' +
          'Ishijima M. 2022. Three-dimensional morphological analysis of the thoracic pedicle and related ' +
          'radiographic factors in adolescent idiopathic scoliosis. BMC Musculoskelet Disord 23(1):847.',
        pmid: '36068555',
        doi: '10.1186/s12891-022-05799-4',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-pedicle-lengths-gao-2017-pedicle-only-length',
    feature: 'Short pedicle lengths, distinct from full screw chords — pedicle only length',
    publishedNumber:
      'key pedicle-only-length, level T1, group gao-ais, curveSide concave: 11.0 ± 2.34 (spread statistic not ' +
      'stated) mm; printed 11 ± 2.34; printed to whole mm. key pedicle-only-length, level T1, group gao-ais, ' +
      'curveSide convex: 11.0 ± 2.17 (spread statistic not stated) mm; printed 11 ± 2.17; printed to whole mm. ' +
      'key pedicle-only-length, level T1, group gao-controls: 11.0 ± 1.57 (spread statistic not stated) mm; ' +
      'printed 11 ± 1.57; printed to whole mm. key pedicle-only-length, level T2, group gao-ais, curveSide ' +
      'concave: 13.0 ± 1.92 (spread statistic not stated) mm; printed 13 ± 1.92; printed to whole mm. key ' +
      'pedicle-only-length, level T2, group gao-ais, curveSide convex: 13.0 ± 1.75 (spread statistic not stated) ' +
      'mm; printed 13 ± 1.75; printed to whole mm. key pedicle-only-length, level T2, group gao-controls: 13.0 ± ' +
      '1.71 (spread statistic not stated) mm; printed 13 ± 1.71; printed to whole mm. key pedicle-only-length, ' +
      'level T3, group gao-ais, curveSide concave: 14.0 ± 2.25 (spread statistic not stated) mm; printed 14 ± ' +
      '2.25; printed to whole mm. key pedicle-only-length, level T3, group gao-ais, curveSide convex: 14.0 ± ' +
      '1.87 (spread statistic not stated) mm; printed 14 ± 1.87; printed to whole mm. key pedicle-only-length, ' +
      'level T3, group gao-controls: 14.0 ± 1.73 (spread statistic not stated) mm; printed 14 ± 1.73; printed to ' +
      'whole mm. key pedicle-only-length, level T4, group gao-ais, curveSide concave: 15.0 ± 2.43 (spread ' +
      'statistic not stated) mm; printed 15 ± 2.43; printed to whole mm. key pedicle-only-length, level T4, ' +
      'group gao-ais, curveSide convex: 14.0 ± 2.18 (spread statistic not stated) mm; printed 14 ± 2.18; printed ' +
      'to whole mm. key pedicle-only-length, level T4, group gao-controls: 14.0 ± 1.38 (spread statistic not ' +
      'stated) mm; printed 14 ± 1.38; printed to whole mm. key pedicle-only-length, level T5, group gao-ais, ' +
      'curveSide concave: 15.0 ± 1.9 (spread statistic not stated) mm; printed 15 ± 1.90; printed to whole mm. ' +
      'key pedicle-only-length, level T5, group gao-ais, curveSide convex: 14.0 ± 1.98 (spread statistic not ' +
      'stated) mm; printed 14 ± 1.98; printed to whole mm. key pedicle-only-length, level T5, group ' +
      'gao-controls: 14.0 ± 1.4 (spread statistic not stated) mm; printed 14 ± 1.40; printed to whole mm. key ' +
      'pedicle-only-length, level T6, group gao-ais, curveSide concave: 15.0 ± 2.46 (spread statistic not ' +
      'stated) mm; printed 15 ± 2.46*,†; printed to whole mm. key pedicle-only-length, level T6, group gao-ais, ' +
      'curveSide convex: 14.0 ± 1.9 (spread statistic not stated) mm; printed 14 ± 1.90; printed to whole mm. ' +
      'key pedicle-only-length, level T6, group gao-controls: 14.0 ± 1.47 (spread statistic not stated) mm; ' +
      'printed 14 ± 1.47; printed to whole mm. key pedicle-only-length, level T7, group gao-ais, curveSide ' +
      'concave: 15.0 ± 2.32 (spread statistic not stated) mm; printed 15 ± 2.32*,†; printed to whole mm. key ' +
      'pedicle-only-length, level T7, group gao-ais, curveSide convex: 14.0 ± 1.62 (spread statistic not stated) ' +
      'mm; printed 14 ± 1.62; printed to whole mm. key pedicle-only-length, level T7, group gao-controls: 14.0 ± ' +
      '1.36 (spread statistic not stated) mm; printed 14 ± 1.36; printed to whole mm. key pedicle-only-length, ' +
      'level T8, group gao-ais, curveSide concave: 14.0 ± 2.17 (spread statistic not stated) mm; printed 14 ± ' +
      '2.17; printed to whole mm. key pedicle-only-length, level T8, group gao-ais, curveSide convex: 14.0 ± ' +
      '1.92 (spread statistic not stated) mm; printed 14 ± 1.92; printed to whole mm. key pedicle-only-length, ' +
      'level T8, group gao-controls: 14.0 ± 1.49 (spread statistic not stated) mm; printed 14 ± 1.49; printed to ' +
      'whole mm. key pedicle-only-length, level T9, group gao-ais, curveSide concave: 14.0 ± 2.34 (spread ' +
      'statistic not stated) mm; printed 14 ± 2.34; printed to whole mm. key pedicle-only-length, level T9, ' +
      'group gao-ais, curveSide convex: 14.0 ± 2.15 (spread statistic not stated) mm; printed 14 ± 2.15; printed ' +
      'to whole mm. key pedicle-only-length, level T9, group gao-controls: 14.0 ± 1.7 (spread statistic not ' +
      'stated) mm; printed 14 ± 1.70; printed to whole mm. key pedicle-only-length, level T10, group gao-ais, ' +
      'curveSide concave: 14.0 ± 2.24 (spread statistic not stated) mm; printed 14 ± 2.24; printed to whole mm. ' +
      'key pedicle-only-length, level T10, group gao-ais, curveSide convex: 14.0 ± 2.44 (spread statistic not ' +
      'stated) mm; printed 14 ± 2.44; printed to whole mm. key pedicle-only-length, level T10, group ' +
      'gao-controls: 14.0 ± 1.67 (spread statistic not stated) mm; printed 14 ± 1.67; printed to whole mm. key ' +
      'pedicle-only-length, level T11, group gao-ais, curveSide concave: 15.0 ± 2.42 (spread statistic not ' +
      'stated) mm; printed 15 ± 2.42; printed to whole mm. key pedicle-only-length, level T11, group gao-ais, ' +
      'curveSide convex: 15.0 ± 3.14 (spread statistic not stated) mm; printed 15± 3.14; printed to whole mm. ' +
      'key pedicle-only-length, level T11, group gao-controls: 15.0 ± 1.74 (spread statistic not stated) mm; ' +
      'printed 15 ± 1.74; printed to whole mm. key pedicle-only-length, level T12, group gao-ais, curveSide ' +
      'concave: 17.0 ± 2.56 (spread statistic not stated) mm; printed 17± 2.56; printed to whole mm. key ' +
      'pedicle-only-length, level T12, group gao-ais, curveSide convex: 16.0 ± 2.55 (spread statistic not ' +
      'stated) mm; printed 16 ± 2.55; printed to whole mm. key pedicle-only-length, level T12, group ' +
      'gao-controls: 17.0 ± 1.83 (spread statistic not stated) mm; printed 17 ± 1.83; printed to whole mm. The ' +
      'paper leaves ± unlabelled; reading it as SD is this project\'s inference. Curve direction and the ' +
      'anatomical left/right mapping are not stated. Short pedicle-only length between the paper\'s guideline and ' +
      'lamina-axis intersections, not a posterior-entry-to-anterior-cortex chord. Do not substitute it for a ' +
      'chord. The paper leaves ± unlabelled; reading it as SD is this project\'s inference. Curve direction and ' +
      'the anatomical left/right mapping are not stated. Short pedicle-only length between the paper\'s guideline ' +
      'and lamina-axis intersections, not a posterior-entry-to-anterior-cortex chord. Do not substitute it for a ' +
      'chord. Printed * and † compare this column with control and convex values at p <0.05.',
    population:
      'gao-2017: Girls with Lenke 1 AIS and female controls matched for age, sex and height; age 15 ± 2.6; ' +
      'height 154 vs 155 cm. Sample: 60 AIS girls; 60 female controls; 1,322 patient pedicles and 1,396 control ' +
      'pedicles. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Gao B, Gao W, Chen C, Wang Q, Lin S, Xu C, Huang D, Su P. 2017. What is the Difference in Morphologic ' +
          'Features of the Thoracic Pedicle Between Patients With Adolescent Idiopathic Scoliosis and Healthy ' +
          'Subjects? A CT-based Case-control Study. Clin Orthop Relat Res 475(11):2765-2774.',
        pmid: '28766159',
        doi: '10.1007/s11999-017-5448-9',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-pedicle-widths-gao-2017-outer-width',
    feature: 'Pedicle widths from population tables — outer width',
    publishedNumber:
      'key outer-width, level T1, group gao-ais, curveSide concave: 7.0 ± 1.25 (spread statistic not stated) mm; ' +
      'printed 7 ± 1.25; printed to whole mm. key outer-width, level T1, group gao-ais, curveSide convex: 7.0 ± ' +
      '1.09 (spread statistic not stated) mm; printed 7 ± 1.09; printed to whole mm. key outer-width, level T1, ' +
      'group gao-controls: 7.0 ± 1.01 (spread statistic not stated) mm; printed 7 ± 1.01; printed to whole mm. ' +
      'key outer-width, level T2, group gao-ais, curveSide concave: 5.0 ± 1.24 (spread statistic not stated) mm; ' +
      'printed 5 ± 1.24*,†; printed to whole mm. key outer-width, level T2, group gao-ais, curveSide convex: 6.0 ' +
      '± 0.91 (spread statistic not stated) mm; printed 6 ± 0.91; printed to whole mm. key outer-width, level ' +
      'T2, group gao-controls: 6.0 ± 1.02 (spread statistic not stated) mm; printed 6 ± 1.02; printed to whole ' +
      'mm. key outer-width, level T3, group gao-ais, curveSide concave: 3.76 ± 1.37 (spread statistic not ' +
      'stated) mm; printed 3.76 ± 1.37*,†. key outer-width, level T3, group gao-ais, curveSide convex: 4.98 ± ' +
      '1.03 (spread statistic not stated) mm; printed 4.98 ± 1.03. key outer-width, level T3, group ' +
      'gao-controls: 4.89 ± 0.83 (spread statistic not stated) mm; printed 4.89 ± 0.83. key outer-width, level ' +
      'T4, group gao-ais, curveSide concave: 3.41 ± 0.98 (spread statistic not stated) mm; printed 3.41 ± ' +
      '0.98*,†. key outer-width, level T4, group gao-ais, curveSide convex: 4.44 ± 0.88 (spread statistic not ' +
      'stated) mm; printed 4.44 ± 0.88. key outer-width, level T4, group gao-controls: 4.46 ± 0.89 (spread ' +
      'statistic not stated) mm; printed 4.46 ± 0.89. key outer-width, level T5, group gao-ais, curveSide ' +
      'concave: 3.95 ± 1.05 (spread statistic not stated) mm; printed 3.95 ± 1.05*,†. key outer-width, level T5, ' +
      'group gao-ais, curveSide convex: 4.71 ± 1.01 (spread statistic not stated) mm; printed 4.71±1.01. key ' +
      'outer-width, level T5, group gao-controls: 4.44 ± 0.75 (spread statistic not stated) mm; printed 4.44 ± ' +
      '0.75. key outer-width, level T6, group gao-ais, curveSide concave: 4.05 ± 1.05 (spread statistic not ' +
      'stated) mm; printed 4.05 ± 1.05*,†. key outer-width, level T6, group gao-ais, curveSide convex: 4.77 ± ' +
      '1.01 (spread statistic not stated) mm; printed 4.77 ± 1.01. key outer-width, level T6, group ' +
      'gao-controls: 4.61 ± 0.79 (spread statistic not stated) mm; printed 4.61 ± 0.79. key outer-width, level ' +
      'T7, group gao-ais, curveSide concave: 3.97 ± 1.1 (spread statistic not stated) mm; printed 3.97 ± ' +
      '1.10*,†. key outer-width, level T7, group gao-ais, curveSide convex: 5.0 ± 0.88 (spread statistic not ' +
      'stated) mm; printed 5 ± 0.88; printed to whole mm. key outer-width, level T7, group gao-controls: 4.68 ± ' +
      '0.8 (spread statistic not stated) mm; printed 4.68 ± 0.80. key outer-width, level T8, group gao-ais, ' +
      'curveSide concave: 4.08 ± 1.11 (spread statistic not stated) mm; printed 4.08 ± 1.11*,†. key outer-width, ' +
      'level T8, group gao-ais, curveSide convex: 5.0 ± 1.04 (spread statistic not stated) mm; printed 5 ± 1.04; ' +
      'printed to whole mm. key outer-width, level T8, group gao-controls: 4.99 ± 0.91 (spread statistic not ' +
      'stated) mm; printed 4.99 ± 0.91. key outer-width, level T9, group gao-ais, curveSide concave: 4.92 ± 1.45 ' +
      '(spread statistic not stated) mm; printed 4.92 ± 1.45*,†. key outer-width, level T9, group gao-ais, ' +
      'curveSide convex: 5.0 ± 1.05 (spread statistic not stated) mm; printed 5 ± 1.05; printed to whole mm. key ' +
      'outer-width, level T9, group gao-controls: 5.0 ± 0.9 (spread statistic not stated) mm; printed 5 ± 0.90; ' +
      'printed to whole mm. key outer-width, level T10, group gao-ais, curveSide concave: 6.0 ± 1.62 (spread ' +
      'statistic not stated) mm; printed 6 ± 1.62; printed to whole mm. key outer-width, level T10, group ' +
      'gao-ais, curveSide convex: 6.0 ± 1.43 (spread statistic not stated) mm; printed 6 ± 1.43; printed to ' +
      'whole mm. key outer-width, level T10, group gao-controls: 6.0 ± 1.0 (spread statistic not stated) mm; ' +
      'printed 6 ± 1.00; printed to whole mm. key outer-width, level T11, group gao-ais, curveSide concave: 7.0 ' +
      '± 2.04 (spread statistic not stated) mm; printed 7 ± 2.04; printed to whole mm. key outer-width, level ' +
      'T11, group gao-ais, curveSide convex: 7.0 ± 1.81 (spread statistic not stated) mm; printed 7 ± 1.81; ' +
      'printed to whole mm. key outer-width, level T11, group gao-controls: 7.0 ± 1.73 (spread statistic not ' +
      'stated) mm; printed 7 ± 1.73; printed to whole mm. key outer-width, level T12, group gao-ais, curveSide ' +
      'concave: 7.0 ± 1.61 (spread statistic not stated) mm; printed 7 ± 1.61; printed to whole mm. key ' +
      'outer-width, level T12, group gao-ais, curveSide convex: 7.0 ± 1.83 (spread statistic not stated) mm; ' +
      'printed 7 ± 1.83; printed to whole mm. key outer-width, level T12, group gao-controls: 7.0 ± 1.57 (spread ' +
      'statistic not stated) mm; printed 7 ± 1.57; printed to whole mm. The paper leaves ± unlabelled; reading ' +
      'it as SD is this project\'s inference. Curve direction and the anatomical left/right mapping are not ' +
      'stated. The historical outer-width key does not assert an outer-cortex measurement; the paper does not ' +
      'specify outer or inner cortex. The paper leaves ± unlabelled; reading it as SD is this project\'s ' +
      'inference. Curve direction and the anatomical left/right mapping are not stated. The historical ' +
      'outer-width key does not assert an outer-cortex measurement; the paper does not specify outer or inner ' +
      'cortex. Printed * and † compare this column with control and convex values at p <0.05.',
    population:
      'gao-2017: Girls with Lenke 1 AIS and female controls matched for age, sex and height; age 15 ± 2.6; ' +
      'height 154 vs 155 cm. Sample: 60 AIS girls; 60 female controls; 1,322 patient pedicles and 1,396 control ' +
      'pedicles. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Gao B, Gao W, Chen C, Wang Q, Lin S, Xu C, Huang D, Su P. 2017. What is the Difference in Morphologic ' +
          'Features of the Thoracic Pedicle Between Patients With Adolescent Idiopathic Scoliosis and Healthy ' +
          'Subjects? A CT-based Case-control Study. Clin Orthop Relat Res 475(11):2765-2774.',
        pmid: '28766159',
        doi: '10.1007/s11999-017-5448-9',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-pedicle-widths-pluemvitayaporn-2025-outer-width',
    feature: 'Pedicle widths from population tables — outer width',
    publishedNumber:
      'key outer-width, level T1, group pluemvitayaporn-lenke-5, curveSide concave: 3.71 ± 1.08 SD mm. key ' +
      'outer-width, level T1, group pluemvitayaporn-lenke-5, curveSide convex: 4.13 ± 1.47 SD mm. key ' +
      'outer-width, level T2, group pluemvitayaporn-lenke-5, curveSide concave: 3.48 ± 1.12 SD mm. key ' +
      'outer-width, level T2, group pluemvitayaporn-lenke-5, curveSide convex: 3.68 ± 1.03 SD mm. key ' +
      'outer-width, level T3, group pluemvitayaporn-lenke-5, curveSide concave: 2.74 ± 0.93 SD mm. key ' +
      'outer-width, level T3, group pluemvitayaporn-lenke-5, curveSide convex: 3.08 ± 0.81 SD mm. key ' +
      'outer-width, level T4, group pluemvitayaporn-lenke-5, curveSide concave: 2.83 ± 0.74 SD mm. key ' +
      'outer-width, level T4, group pluemvitayaporn-lenke-5, curveSide convex: 2.79 ± 0.92 SD mm. key ' +
      'outer-width, level T5, group pluemvitayaporn-lenke-5, curveSide concave: 2.71 ± 0.91 SD mm. key ' +
      'outer-width, level T5, group pluemvitayaporn-lenke-5, curveSide convex: 2.74 ± 0.81 SD mm. key ' +
      'outer-width, level T6, group pluemvitayaporn-lenke-5, curveSide concave: 2.82 ± 0.94 SD mm. key ' +
      'outer-width, level T6, group pluemvitayaporn-lenke-5, curveSide convex: 2.83 ± 0.88 SD mm. key ' +
      'outer-width, level T7, group pluemvitayaporn-lenke-5, curveSide concave: 2.79 ± 0.76 SD mm. key ' +
      'outer-width, level T7, group pluemvitayaporn-lenke-5, curveSide convex: 2.85 ± 0.82 SD mm. key ' +
      'outer-width, level T8, group pluemvitayaporn-lenke-5, curveSide concave: 3.04 ± 0.66 SD mm. key ' +
      'outer-width, level T8, group pluemvitayaporn-lenke-5, curveSide convex: 3.15 ± 0.83 SD mm. key ' +
      'outer-width, level T9, group pluemvitayaporn-lenke-5, curveSide concave: 3.26 ± 0.82 SD mm. key ' +
      'outer-width, level T9, group pluemvitayaporn-lenke-5, curveSide convex: 3.42 ± 0.83 SD mm. key ' +
      'outer-width, level T10, group pluemvitayaporn-lenke-5, curveSide concave: 3.61 ± 1.05 SD mm. key ' +
      'outer-width, level T10, group pluemvitayaporn-lenke-5, curveSide convex: 3.76 ± 1.05 SD mm. key ' +
      'outer-width, level T11, group pluemvitayaporn-lenke-5, curveSide concave: 4.43 ± 1.09 SD mm. key ' +
      'outer-width, level T11, group pluemvitayaporn-lenke-5, curveSide convex: 4.62 ± 0.88 SD mm. key ' +
      'outer-width, level T12, group pluemvitayaporn-lenke-5, curveSide concave: 4.51 ± 1.11 SD mm. key ' +
      'outer-width, level T12, group pluemvitayaporn-lenke-5, curveSide convex: 5.26 ± 1.07 SD mm. key ' +
      'outer-width, level L1, group pluemvitayaporn-lenke-5, curveSide concave: 4.57 ± 0.98 SD mm. key ' +
      'outer-width, level L1, group pluemvitayaporn-lenke-5, curveSide convex: 4.94 ± 1.03 SD mm. key ' +
      'outer-width, level L2, group pluemvitayaporn-lenke-5, curveSide concave: 4.68 ± 1.14 SD mm. key ' +
      'outer-width, level L2, group pluemvitayaporn-lenke-5, curveSide convex: 4.97 ± 1.03 SD mm. key ' +
      'outer-width, level L3, group pluemvitayaporn-lenke-5, curveSide concave: 4.89 ± 1.26 SD mm. key ' +
      'outer-width, level L3, group pluemvitayaporn-lenke-5, curveSide convex: 5.02 ± 1.22 SD mm. key ' +
      'outer-width, level L4, group pluemvitayaporn-lenke-5, curveSide concave: 6.02 ± 1.51 SD mm. key ' +
      'outer-width, level L4, group pluemvitayaporn-lenke-5, curveSide convex: 6.27 ± 1.38 SD mm. key ' +
      'outer-width, level L5, group pluemvitayaporn-lenke-5, curveSide concave: 8.71 ± 2.21 SD mm. key ' +
      'outer-width, level L5, group pluemvitayaporn-lenke-5, curveSide convex: 9.07 ± 1.89 SD mm. Alternative ' +
      'Lenke 5 cohort; the note cautions that absolute widths are substantially smaller than in other series.',
    population:
      'pluemvitayaporn-2025: Thai patients with Lenke 5 curves; mean age 14.7 (12–22), mean Cobb 48° (36–84°). ' +
      'Sample: 112 patients; 3,808 pedicles. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Pluemvitayaporn T, Kaewmano T, Kunakornsawat S, Surapuchong S, Ratanakoosakul W, Tiracharnvut K, ' +
          'Pruttikul P, Piyasakulkaew C, Kittithamvongs P. 2025. Pedicle Morphology Analysis in Adolescents with ' +
          'Lenke Type 5 Idiopathic Scoliosis in Thai Population. Asian J Neurosurg 20(1):132-137.',
        pmid: '40041581',
        doi: '10.1055/s-0044-1800812',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-pedicle-widths-upendra-2010-outer-width',
    feature: 'Pedicle widths from population tables — outer width',
    publishedNumber:
      'key outer-width, level T1, curveSide convex, group upendra-ais: 5.1 ± 1.76 SD mm (range 3.8 to 7.1); n = ' +
      '12; printed 5.1. key outer-width, level T1, curveSide concave, group upendra-ais: 5.2 ± 1.01 SD mm (range ' +
      '3.6 to 6.8); n = 12; printed 5.2. key outer-width, level T2, curveSide convex, group upendra-ais: 5.35 ± ' +
      '1.25 SD mm (range 4.1 to 7.2); n = 10; printed 5.35. key outer-width, level T2, curveSide concave, group ' +
      'upendra-ais: 5.2 ± 1.36 SD mm (range 3.7 to 7.2); n = 11; printed 5.2. key outer-width, level T3, ' +
      'curveSide convex, group upendra-ais: 5.12 ± 1.11 SD mm (range 3.6 to 6.8); n = 13; printed 5.12. key ' +
      'outer-width, level T3, curveSide concave, group upendra-ais: 4.1 ± 0.31 SD mm (range 3.4 to 5.1); n = 13; ' +
      'printed 4.1; doubtful. key outer-width, level T4, curveSide convex, group upendra-ais: 4.92 ± 0.86 SD mm ' +
      '(range 3.0 to 5.8); n = 14; printed 4.92. key outer-width, level T4, curveSide concave, group ' +
      'upendra-ais: 4.42 ± 0.62 SD mm (range 3.6 to 5.0); n = 15; printed 4.42. key outer-width, level T5, ' +
      'curveSide convex, group upendra-ais: 5.87 ± 0.61 SD mm (range 5.1 to 7.2); n = 9; printed 5.87. key ' +
      'outer-width, level T5, curveSide concave, group upendra-ais: 5.2 ± 0.41 SD mm (range 4.1 to 6.2); n = 9; ' +
      'printed 5.2. key outer-width, level T6, curveSide convex, group upendra-ais: 6.6 ± 1.14 SD mm (range 5.4 ' +
      'to 8.1); n = 11; printed 6.6. key outer-width, level T6, curveSide concave, group upendra-ais: 4.95 ± ' +
      '0.56 SD mm (range 4.0 to 6.1); n = 13; printed 4.95. key outer-width, level T7, curveSide convex, group ' +
      'upendra-ais: 6.6 ± 1.32 SD mm (range 5.1 to 9.2); n = 9; printed 6.60. key outer-width, level T7, ' +
      'curveSide concave, group upendra-ais: 5.5 ± 0.21 SD mm (range 4.6 to 6.3); n = 11; printed 5.50; ' +
      'doubtful. key outer-width, level T8, curveSide convex, group upendra-ais: 5.35 ± 0.68 SD mm (range 4.5 to ' +
      '6.3); n = 14; printed 5.35. key outer-width, level T8, curveSide concave, group upendra-ais: 5.33 ± 0.46 ' +
      'SD mm (range 4.2 to 6.4); n = 14; printed 5.33. key outer-width, level T9, curveSide convex, group ' +
      'upendra-ais: 5.08 ± 0.89 SD mm (range 4.3 to 6.2); n = 12; printed 5.08. key outer-width, level T9, ' +
      'curveSide concave, group upendra-ais: 4.96 ± 0.65 SD mm (range 3.8 to 6.1); n = 12; printed 4.96. key ' +
      'outer-width, level T10, curveSide convex, group upendra-ais: 5.36 ± 1.12 SD mm (range 4.2 to 7.7); n = ' +
      '15; printed 5.36. key outer-width, level T10, curveSide concave, group upendra-ais: 5.21 ± 1.01 SD mm ' +
      '(range 4.1 to 7.1); n = 15; printed 5.21. key outer-width, level T11, curveSide convex, group ' +
      'upendra-ais: 6.27 ± 1.36 SD mm (range 4.8 to 8.2); n = 16; printed 6.27. key outer-width, level T11, ' +
      'curveSide concave, group upendra-ais: 5.6 ± 1.65 SD mm (range 3.6 to 8.1); n = 16; printed 5.60. key ' +
      'outer-width, level T12, curveSide convex, group upendra-ais: 6.8 ± 1.33 SD mm (range 5.0 to 9.7); n = 19; ' +
      'printed 6.8. key outer-width, level T12, curveSide concave, group upendra-ais: 5.22 ± 0.89 SD mm (range ' +
      '4.1 to 7.2); n = 22; printed 5.22. key outer-width, level L1, curveSide convex, group upendra-ais: 6.48 ± ' +
      '1.78 SD mm (range 4.6 to 10.1); n = 18; printed 6.48. key outer-width, level L1, curveSide concave, group ' +
      'upendra-ais: 6.04 ± 1.63 SD mm (range 4.0 to 9.2); n = 19; printed 6.04. key outer-width, level L2, ' +
      'curveSide convex, group upendra-ais: 5.8 ± 1.14 SD mm (range 4.5 to 9.0); n = 13; printed 5.8. key ' +
      'outer-width, level L2, curveSide concave, group upendra-ais: 6.16 ± 1.06 SD mm (range 4.6 to 9.1); n = ' +
      '13; printed 6.16. key outer-width, level L3, curveSide convex, group upendra-ais: 7.77 ± 1.37 SD mm ' +
      '(range 6.1 to 10.3); n = 11; printed 7.77. key outer-width, level L3, curveSide concave, group ' +
      'upendra-ais: 7.46 ± 1.83 SD mm (range 5.1 to 10.3); n = 11; printed 7.46. key outer-width, level L4, ' +
      'curveSide convex, group upendra-ais: 8.9 ± 1.22 SD mm (range 7.2 to 12.1); n = 9; printed 8.9. key ' +
      'outer-width, level L4, curveSide concave, group upendra-ais: 8.16 ± 1.71 SD mm (range 6.1 to 10.4); n = ' +
      '9; printed 8.16. key outer-width, level L5, curveSide convex, group upendra-ais: 12.45 ± 0.96 SD mm ' +
      '(range 11.4 to 14.8); n = 8; printed 12.45. key outer-width, level L5, curveSide concave, group ' +
      'upendra-ais: 9.85 ± 1.44 SD mm (range 8.1 to 12.0); n = 8; printed 9.85. key outer-width, level T10, ' +
      'curveSide concave, group upendra-table-4: 5.51 mm; printed 5.51; retained only, excluded from use; ' +
      'ambiguous print. Printed Table 1 mean, SD, per-cell pedicle count and range; no rounding or ' +
      'interpolation. [own inference] Convex is read as right and concave as left because all 24 patients had ' +
      'right thoracic/thoracolumbar curves; the paper does not explicitly equate convex with right. Printed ' +
      'Table 1 mean, SD, per-cell pedicle count and range; no rounding or interpolation. [own inference] Convex ' +
      'is read as right and concave as left because all 24 patients had right thoracic/thoracolumbar curves; the ' +
      'paper does not explicitly equate convex with right. Table 4(a) repeats this cell as 5.51, disagreeing ' +
      'with Table 1’s 5.21; this primary cell follows Table 1. Alternative figure retained only as printed, ' +
      'separate from primary Tables 1–3. [own inference] Convex is read as right and concave as left because all ' +
      '24 patients had right thoracic/thoracolumbar curves; the paper does not explicitly equate convex with ' +
      'right.',
    population:
      'upendra-2010: AIS; 14 girls and 10 boys; all right thoracic or thoracolumbar curves (King II 12, III 7, ' +
      'IV 5); ages 12.3–18.3; Cobb 58–98°; 3 mm CT. Sample: 24 patients; width n 213 convex/223 concave; angle n ' +
      '217 convex/216 concave; chord n 213 on each side; per-cell pedicle counts retained. [peer-reviewed; ' +
      'full-text].',
    sources: [
      {
        citation:
          'Upendra B, Meena D, Kandwal P, Ahmed A, Chowdhury B, Jayaswal A. 2010. Pedicle morphometry in patients ' +
          'with adolescent idiopathic scoliosis. Indian J Orthop 44(2):169-176.',
        pmid: '20419004',
        doi: '10.4103/0019-5413.62084',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-pedicle-widths-zheng-2009-outer-width',
    feature: 'Pedicle widths from population tables — outer width',
    publishedNumber:
      'key outer-width, level T1, ageBand 5-8, sex male, group zheng-children: 6.06 ± 0.92 (spread statistic not ' +
      'stated) mm; printed 6.06 ± 0.92. key outer-width, level T1, ageBand 5-8, sex female, group ' +
      'zheng-children: 5.73 ± 0.94 (spread statistic not stated) mm; printed 5.73 ± 0.94. key outer-width, level ' +
      'T1, ageBand 9-11, sex male, group zheng-children: 6.84 ± 0.67 (spread statistic not stated) mm; printed ' +
      '6.84 ± 0.67. key outer-width, level T1, ageBand 9-11, sex female, group zheng-children: 6.3 ± 0.68 ' +
      '(spread statistic not stated) mm; printed 6.3 ± 0.68. key outer-width, level T1, ageBand 12-14, sex male, ' +
      'group zheng-children: 7.44 ± 0.82 (spread statistic not stated) mm; printed 7.44 ± 0.82. key outer-width, ' +
      'level T1, ageBand 12-14, sex female, group zheng-children: 6.7 ± 0.89 (spread statistic not stated) mm; ' +
      'printed 6.7 ± 0.89. key outer-width, level T2, ageBand 5-8, sex male, group zheng-children: 5.38 ± 0.73 ' +
      '(spread statistic not stated) mm; printed 5.38 ± 0.73. key outer-width, level T2, ageBand 5-8, sex ' +
      'female, group zheng-children: 5.19 ± 0.94 (spread statistic not stated) mm; printed 5.19 ± 0.94. key ' +
      'outer-width, level T2, ageBand 9-11, sex male, group zheng-children: 5.82 ± 0.9 (spread statistic not ' +
      'stated) mm; printed 5.82 ± 0.90. key outer-width, level T2, ageBand 9-11, sex female, group ' +
      'zheng-children: 5.64 ± 0.71 (spread statistic not stated) mm; printed 5.64 ± 0.71. key outer-width, level ' +
      'T2, ageBand 12-14, sex male, group zheng-children: 6.26 ± 1.09 (spread statistic not stated) mm; printed ' +
      '6.26 ± 1.09. key outer-width, level T2, ageBand 12-14, sex female, group zheng-children: 5.64 ± 0.9 ' +
      '(spread statistic not stated) mm; printed 5.64 ± 0.90. key outer-width, level T3, ageBand 5-8, sex male, ' +
      'group zheng-children: 4.53 ± 0.63 (spread statistic not stated) mm; printed 4.53 ± 0.63. key outer-width, ' +
      'level T3, ageBand 5-8, sex female, group zheng-children: 4.4 ± 0.81 (spread statistic not stated) mm; ' +
      'printed 4.40 ± 0.81. key outer-width, level T3, ageBand 9-11, sex male, group zheng-children: 4.93 ± 0.83 ' +
      '(spread statistic not stated) mm; printed 4.93 ± 0.83. key outer-width, level T3, ageBand 9-11, sex ' +
      'female, group zheng-children: 4.64 ± 0.65 (spread statistic not stated) mm; printed 4.64 ± 0.65. key ' +
      'outer-width, level T3, ageBand 12-14, sex male, group zheng-children: 5.11 ± 1.24 (spread statistic not ' +
      'stated) mm; printed 5.11 ± 1.24. key outer-width, level T3, ageBand 12-14, sex female, group ' +
      'zheng-children: 4.6 ± 0.81 (spread statistic not stated) mm; printed 4.60 ± 0.81. key outer-width, level ' +
      'T4, ageBand 5-8, sex male, group zheng-children: 3.89 ± 0.63 (spread statistic not stated) mm; printed ' +
      '3.89 ± 0.63. key outer-width, level T4, ageBand 5-8, sex female, group zheng-children: 3.67 ± 0.84 ' +
      '(spread statistic not stated) mm; printed 3.67 ± 0.84. key outer-width, level T4, ageBand 9-11, sex male, ' +
      'group zheng-children: 4.1 ± 0.74 (spread statistic not stated) mm; printed 4.10 ± 0.74. key outer-width, ' +
      'level T4, ageBand 9-11, sex female, group zheng-children: 4.09 ± 0.51 (spread statistic not stated) mm; ' +
      'printed 4.09 ± 0.51. key outer-width, level T4, ageBand 12-14, sex male, group zheng-children: 4.36 ± ' +
      '0.91 (spread statistic not stated) mm; printed 4.36 ± 0.91. key outer-width, level T4, ageBand 12-14, sex ' +
      'female, group zheng-children: 3.93 ± 0.88 (spread statistic not stated) mm; printed 3.93 ± 0.88. key ' +
      'outer-width, level T5, ageBand 5-8, sex male, group zheng-children: 3.85 ± 0.56 (spread statistic not ' +
      'stated) mm; printed 3.85 ± 0.56. key outer-width, level T5, ageBand 5-8, sex female, group ' +
      'zheng-children: 3.59 ± 0.64 (spread statistic not stated) mm; printed 3.59 ± 0.64. key outer-width, level ' +
      'T5, ageBand 9-11, sex male, group zheng-children: 3.97 ± 1.0 (spread statistic not stated) mm; printed ' +
      '3.97 ± 1.00. key outer-width, level T5, ageBand 9-11, sex female, group zheng-children: 3.91 ± 0.46 ' +
      '(spread statistic not stated) mm; printed 3.91 ± 0.46. key outer-width, level T5, ageBand 12-14, sex ' +
      'male, group zheng-children: 4.39 ± 0.84 (spread statistic not stated) mm; printed 4.39 ± 0.84. key ' +
      'outer-width, level T5, ageBand 12-14, sex female, group zheng-children: 4.13 ± 0.82 (spread statistic not ' +
      'stated) mm; printed 4.13 ± 0.82. key outer-width, level T6, ageBand 5-8, sex male, group zheng-children: ' +
      '3.99 ± 0.54 (spread statistic not stated) mm; printed 3.99 ± 0.54. key outer-width, level T6, ageBand ' +
      '5-8, sex female, group zheng-children: 3.76 ± 0.68 (spread statistic not stated) mm; printed 3.76 ± 0.68. ' +
      'key outer-width, level T6, ageBand 9-11, sex male, group zheng-children: 4.27 ± 1.05 (spread statistic ' +
      'not stated) mm; printed 4.27 ± 1.05. key outer-width, level T6, ageBand 9-11, sex female, group ' +
      'zheng-children: 3.92 ± 0.55 (spread statistic not stated) mm; printed 3.92 ± 0.55. key outer-width, level ' +
      'T6, ageBand 12-14, sex male, group zheng-children: 4.56 ± 0.93 (spread statistic not stated) mm; printed ' +
      '4.56 ± 0.93. key outer-width, level T6, ageBand 12-14, sex female, group zheng-children: 4.16 ± 0.81 ' +
      '(spread statistic not stated) mm; printed 4.16 ± 0.81. key outer-width, level T7, ageBand 5-8, sex male, ' +
      'group zheng-children: 4.19 ± 0.59 (spread statistic not stated) mm; printed 4.19 ± 0.59. key outer-width, ' +
      'level T7, ageBand 5-8, sex female, group zheng-children: 3.84 ± 0.61 (spread statistic not stated) mm; ' +
      'printed 3.84 ± 0.61. key outer-width, level T7, ageBand 9-11, sex male, group zheng-children: 4.43 ± 1.02 ' +
      '(spread statistic not stated) mm; printed 4.43 ± 1.02. key outer-width, level T7, ageBand 9-11, sex ' +
      'female, group zheng-children: 3.96 ± 0.47 (spread statistic not stated) mm; printed 3.96 ± 0.47. key ' +
      'outer-width, level T7, ageBand 12-14, sex male, group zheng-children: 4.81 ± 0.99 (spread statistic not ' +
      'stated) mm; printed 4.81 ± 0.99. key outer-width, level T7, ageBand 12-14, sex female, group ' +
      'zheng-children: 4.25 ± 0.82 (spread statistic not stated) mm; printed 4.25 ± 0.82. key outer-width, level ' +
      'T8, ageBand 5-8, sex male, group zheng-children: 4.54 ± 0.64 (spread statistic not stated) mm; printed ' +
      '4.54 ± 0.64. key outer-width, level T8, ageBand 5-8, sex female, group zheng-children: 4.06 ± 0.67 ' +
      '(spread statistic not stated) mm; printed 4.06 ± 0.67. key outer-width, level T8, ageBand 9-11, sex male, ' +
      'group zheng-children: 4.78 ± 0.88 (spread statistic not stated) mm; printed 4.78 ± 0.88. key outer-width, ' +
      'level T8, ageBand 9-11, sex female, group zheng-children: 4.54 ± 0.57 (spread statistic not stated) mm; ' +
      'printed 4.54 ± 0.57. key outer-width, level T8, ageBand 12-14, sex male, group zheng-children: 5.09 ± ' +
      '1.08 (spread statistic not stated) mm; printed 5.09 ± 1.08. key outer-width, level T8, ageBand 12-14, sex ' +
      'female, group zheng-children: 4.64 ± 1.16 (spread statistic not stated) mm; printed 4.64 ± 1.16. key ' +
      'outer-width, level T9, ageBand 5-8, sex male, group zheng-children: 4.84 ± 0.74 (spread statistic not ' +
      'stated) mm; printed 4.84 ± 0.74. key outer-width, level T9, ageBand 5-8, sex female, group ' +
      'zheng-children: 4.5 ± 0.58 (spread statistic not stated) mm; printed 4.50 ± 0.58. key outer-width, level ' +
      'T9, ageBand 9-11, sex male, group zheng-children: 5.04 ± 0.73 (spread statistic not stated) mm; printed ' +
      '5.04 ± 0.73. key outer-width, level T9, ageBand 9-11, sex female, group zheng-children: 4.72 ± 0.67 ' +
      '(spread statistic not stated) mm; printed 4.72 ± 0.67. key outer-width, level T9, ageBand 12-14, sex ' +
      'male, group zheng-children: 5.7 ± 1.04 (spread statistic not stated) mm; printed 5.70 ± 1.04. key ' +
      'outer-width, level T9, ageBand 12-14, sex female, group zheng-children: 5.01 ± 0.79 (spread statistic not ' +
      'stated) mm; printed 5.01 ± 0.79. key outer-width, level T10, ageBand 5-8, sex male, group zheng-children: ' +
      '5.34 ± 0.94 (spread statistic not stated) mm; printed 5.34 ± 0.94. key outer-width, level T10, ageBand ' +
      '5-8, sex female, group zheng-children: 4.83 mm; printed 4.83 ± 072; retained only, excluded from use; ' +
      'probable misprint. key outer-width, level T10, ageBand 9-11, sex male, group zheng-children: 5.75 ± 0.95 ' +
      '(spread statistic not stated) mm; printed 5.75 ± 0.95. key outer-width, level T10, ageBand 9-11, sex ' +
      'female, group zheng-children: 5.31 ± 0.83 (spread statistic not stated) mm; printed 5.31 ± 0.83. key ' +
      'outer-width, level T10, ageBand 12-14, sex male, group zheng-children: 6.42 ± 1.41 (spread statistic not ' +
      'stated) mm; printed 6.42 ± 1.41. key outer-width, level T10, ageBand 12-14, sex female, group ' +
      'zheng-children: 5.85 ± 1.18 (spread statistic not stated) mm; printed 5.85 ± 1.18. key outer-width, level ' +
      'T11, ageBand 5-8, sex male, group zheng-children: 5.83 ± 0.93 (spread statistic not stated) mm; printed ' +
      '5.83 ± 0.93. key outer-width, level T11, ageBand 5-8, sex female, group zheng-children: 5.49 ± 0.66 ' +
      '(spread statistic not stated) mm; printed 5.49 ± 0.66. key outer-width, level T11, ageBand 9-11, sex ' +
      'male, group zheng-children: 6.36 ± 1.09 (spread statistic not stated) mm; printed 6.36 ± 1.09. key ' +
      'outer-width, level T11, ageBand 9-11, sex female, group zheng-children: 6.03 ± 0.99 (spread statistic not ' +
      'stated) mm; printed 6.03 ± 0.99. key outer-width, level T11, ageBand 12-14, sex male, group ' +
      'zheng-children: 7.22 ± 1.44 (spread statistic not stated) mm; printed 7.22 ± 1.44. key outer-width, level ' +
      'T11, ageBand 12-14, sex female, group zheng-children: 6.57 ± 1.38 (spread statistic not stated) mm; ' +
      'printed 6.57 ± 1.38. key outer-width, level T12, ageBand 5-8, sex male, group zheng-children: 6.27 ± 1.11 ' +
      '(spread statistic not stated) mm; printed 6.27 ± 1.11. key outer-width, level T12, ageBand 5-8, sex ' +
      'female, group zheng-children: 6.0 ± 0.95 (spread statistic not stated) mm; printed 6.00 ± 0.95. key ' +
      'outer-width, level T12, ageBand 9-11, sex male, group zheng-children: 6.69 ± 1.13 (spread statistic not ' +
      'stated) mm; printed 6.69 ± 1.13. key outer-width, level T12, ageBand 9-11, sex female, group ' +
      'zheng-children: 6.52 ± 1.13 (spread statistic not stated) mm; printed 6.52 ± 1.13. key outer-width, level ' +
      'T12, ageBand 12-14, sex male, group zheng-children: 7.83 ± 1.58 (spread statistic not stated) mm; printed ' +
      '7.83 ± 1.58. key outer-width, level T12, ageBand 12-14, sex female, group zheng-children: 7.39 ± 1.64 ' +
      '(spread statistic not stated) mm; printed 7.39 ± 1.64. Full text. The paper does not state what ± ' +
      'denotes; reading it as a standard deviation is this project\'s own inference. Numbers of children per age ' +
      'band and sex are not stated. The first of the two unlabelled Table 1 rows at each level is stored as ' +
      'inner width and the second as outer width; that assignment is this project\'s own inference.',
    population:
      'zheng-2009: Chinese children aged 5–14 from a hospital registry, stratified random sampling with thoracic ' +
      'abnormalities excluded; not scoliosis-specific; 2 mm axial CT. Sample: 120 children aged 5–14; number per ' +
      'age band and sex is not stated. [peer-reviewed; full-text].',
    sources: [
      {
        citation: 'Zheng C, Huang Q, Hu Y, Wang X, Chen W. 2009. Int Orthop 33(6):1663-1668.',
        pmid: '18956181',
        doi: '10.1007/s00264-008-0675-z',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-pedicle-widths-sato-2022-outer-width',
    feature: 'Pedicle widths from population tables — outer width',
    publishedNumber:
      'key outer-width, group sato-apex-one, curveSide convex: 4.9 ± 0.8 (spread statistic not stated) mm (range ' +
      '2.9 to 6.2); printed 4.9 ± 0.8 (2.9–6.2). key outer-width, group sato-apex-one, curveSide concave: 3.6 ± ' +
      '0.9 (spread statistic not stated) mm (range 0.9 to 5.7); printed 3.6 ± 0.9 (0.9–5.7). key outer-width, ' +
      'group sato-neutral-one, curveSide convex: 3.6 ± 0.9 (spread statistic not stated) mm (range 0.9 to 5.6); ' +
      'printed 3.6 ± 0.9 (0.9–5.6); retained only, excluded from use; probable copy. key outer-width, group ' +
      'sato-neutral-one, curveSide concave: 3.6 ± 0.7 (spread statistic not stated) mm (range 2.4 to 5.7); ' +
      'printed 3.6 ± 0.7 (2.4–5.7); retained only, excluded from use; probable copy. Measurements are in a plane ' +
      'perpendicular to the pedicle axis through the isthmus. The table prints ± without defining its statistic. ' +
      'Cohort: 57 patients, 49 female and 8 male; Lenke 1 (25) or Lenke 2 (32), 684 pedicles. Enrolled ages ' +
      '12–20, Table 1 age 16.2 ± 2.4; 10–20 is the abstract inclusion range. The paper states right is always ' +
      'convex and left always concave. Per-cell observation counts are not supplied.',
    population:
      'sato-2022: Japanese AIS, enrolled ages 12–20 (inclusion range 10–20), mean age 16.2; 49 girls and 8 boys; ' +
      'Lenke 1/2 right main thoracic curves; 0.5 mm CT. Right is always convex and left always concave. Sample: ' +
      '57 adolescents; 684 pedicles; 15 patients in the 26.3 percent group. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Sato T, Nojiri H, Okuda T, Miyagawa K, Kobayashi N, Takahashi R, Shimura A, Tamagawa S, Ohara Y, Hara T, ' +
          'Ishijima M. 2022. Three-dimensional morphological analysis of the thoracic pedicle and related ' +
          'radiographic factors in adolescent idiopathic scoliosis. BMC Musculoskelet Disord 23(1):847.',
        pmid: '36068555',
        doi: '10.1186/s12891-022-05799-4',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-pedicle-widths-sato-2022-p-value-max',
    feature: 'Pedicle widths from population tables — p value max',
    publishedNumber:
      'key p-value-max, group sato-apex-one-outer-width: 0.001 ratio; printed < 0.001*. Right-versus-left ' +
      'comparison for this measurement and region; strictly less than the printed bound. Cohort: 57 patients, 49 ' +
      'female and 8 male; Lenke 1 (25) or Lenke 2 (32), 684 pedicles. Enrolled ages 12–20, Table 1 age 16.2 ± ' +
      '2.4; 10–20 is the abstract inclusion range. The paper states right is always convex and left always ' +
      'concave. Per-cell observation counts are not supplied.',
    population:
      'sato-2022: Japanese AIS, enrolled ages 12–20 (inclusion range 10–20), mean age 16.2; 49 girls and 8 boys; ' +
      'Lenke 1/2 right main thoracic curves; 0.5 mm CT. Right is always convex and left always concave. Sample: ' +
      '57 adolescents; 684 pedicles; 15 patients in the 26.3 percent group. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Sato T, Nojiri H, Okuda T, Miyagawa K, Kobayashi N, Takahashi R, Shimura A, Tamagawa S, Ohara Y, Hara T, ' +
          'Ishijima M. 2022. Three-dimensional morphological analysis of the thoracic pedicle and related ' +
          'radiographic factors in adolescent idiopathic scoliosis. BMC Musculoskelet Disord 23(1):847.',
        pmid: '36068555',
        doi: '10.1186/s12891-022-05799-4',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-pedicle-widths-sato-2022-p-value',
    feature: 'Pedicle widths from population tables — p value',
    publishedNumber:
      'key p-value, group sato-neutral-one-outer-width: 0.929 ratio; printed 0.929. Right-versus-left comparison ' +
      'for this measurement and region; Exact p-value as printed. Cohort: 57 patients, 49 female and 8 male; ' +
      'Lenke 1 (25) or Lenke 2 (32), 684 pedicles. Enrolled ages 12–20, Table 1 age 16.2 ± 2.4; 10–20 is the ' +
      'abstract inclusion range. The paper states right is always convex and left always concave. Per-cell ' +
      'observation counts are not supplied.',
    population:
      'sato-2022: Japanese AIS, enrolled ages 12–20 (inclusion range 10–20), mean age 16.2; 49 girls and 8 boys; ' +
      'Lenke 1/2 right main thoracic curves; 0.5 mm CT. Right is always convex and left always concave. Sample: ' +
      '57 adolescents; 684 pedicles; 15 patients in the 26.3 percent group. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Sato T, Nojiri H, Okuda T, Miyagawa K, Kobayashi N, Takahashi R, Shimura A, Tamagawa S, Ohara Y, Hara T, ' +
          'Ishijima M. 2022. Three-dimensional morphological analysis of the thoracic pedicle and related ' +
          'radiographic factors in adolescent idiopathic scoliosis. BMC Musculoskelet Disord 23(1):847.',
        pmid: '36068555',
        doi: '10.1186/s12891-022-05799-4',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-placement-checks-chen-2025-sensitivity',
    feature: 'Published placement checks and entry observations — sensitivity',
    publishedNumber:
      'key sensitivity, group liv-index: 100 percent. Derivation cohort only. Section 2b PPV 55.5 percent ' +
      'retained rather than rounded 55 percent in summary table/SYNTHESIS.',
    population:
      'chen-2025: Lenke 1–2 with LIV at or below LTV/LSTV; derivation cohort only. Sample: 112 patients in the ' +
      'full cohort; 10 with adding-on; under-15 subgroup n not supplied. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Chen IH, Chen CW, Huang CC, Hsu JY, Wang PY, Lee YF, Yeh YC, Lai PL, Hu MH, Yang SH. 2026. Lowest ' +
          'Instrumented Vertebra (LIV) Index as a Supplemental Indicator to Lastly Touched Vertebra for Selection of ' +
          'the Fusion Level in Main Thoracic Adolescent Idiopathic Scoliosis. Global Spine J 16(2):956-964.',
        pmid: '40820306',
        doi: '10.1177/21925682251366988',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-placement-checks-chen-2025-specificity',
    feature: 'Published placement checks and entry observations — specificity',
    publishedNumber:
      'key specificity, group liv-index: 92 percent. key specificity, group liv-index-under-15: 94 percent. ' +
      'Derivation cohort only. Section 2b PPV 55.5 percent retained rather than rounded 55 percent in summary ' +
      'table/SYNTHESIS. Under 15 years: specificity 94 percent; strictly less than 15 years. Subgroup n not ' +
      'supplied. The full-cohort n = 112 must not be assigned to this subgroup; derivation cohort only, full ' +
      'text.',
    population:
      'chen-2025: Lenke 1–2 with LIV at or below LTV/LSTV; derivation cohort only. Sample: 112 patients in the ' +
      'full cohort; 10 with adding-on; under-15 subgroup n not supplied. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Chen IH, Chen CW, Huang CC, Hsu JY, Wang PY, Lee YF, Yeh YC, Lai PL, Hu MH, Yang SH. 2026. Lowest ' +
          'Instrumented Vertebra (LIV) Index as a Supplemental Indicator to Lastly Touched Vertebra for Selection of ' +
          'the Fusion Level in Main Thoracic Adolescent Idiopathic Scoliosis. Global Spine J 16(2):956-964.',
        pmid: '40820306',
        doi: '10.1177/21925682251366988',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-placement-checks-chen-2025-positive-test-value',
    feature: 'Published placement checks and entry observations — positive test value',
    publishedNumber:
      'key positive-test-value, group liv-index: 55.5 percent. key positive-test-value, group ' +
      'liv-index-under-15: 77 percent. Derivation cohort only. Section 2b PPV 55.5 percent retained rather than ' +
      'rounded 55 percent in summary table/SYNTHESIS. Under 15 years: PPV 77 percent; strictly less than 15 ' +
      'years. Subgroup n not supplied. The full-cohort n = 112 must not be assigned to this subgroup; derivation ' +
      'cohort only, full text.',
    population:
      'chen-2025: Lenke 1–2 with LIV at or below LTV/LSTV; derivation cohort only. Sample: 112 patients in the ' +
      'full cohort; 10 with adding-on; under-15 subgroup n not supplied. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Chen IH, Chen CW, Huang CC, Hsu JY, Wang PY, Lee YF, Yeh YC, Lai PL, Hu MH, Yang SH. 2026. Lowest ' +
          'Instrumented Vertebra (LIV) Index as a Supplemental Indicator to Lastly Touched Vertebra for Selection of ' +
          'the Fusion Level in Main Thoracic Adolescent Idiopathic Scoliosis. Global Spine J 16(2):956-964.',
        pmid: '40820306',
        doi: '10.1177/21925682251366988',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-placement-checks-chen-2025-negative-test-value',
    feature: 'Published placement checks and entry observations — negative test value',
    publishedNumber:
      'key negative-test-value, group liv-index: 100 percent. Derivation cohort only. Section 2b PPV 55.5 ' +
      'percent retained rather than rounded 55 percent in summary table/SYNTHESIS.',
    population:
      'chen-2025: Lenke 1–2 with LIV at or below LTV/LSTV; derivation cohort only. Sample: 112 patients in the ' +
      'full cohort; 10 with adding-on; under-15 subgroup n not supplied. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Chen IH, Chen CW, Huang CC, Hsu JY, Wang PY, Lee YF, Yeh YC, Lai PL, Hu MH, Yang SH. 2026. Lowest ' +
          'Instrumented Vertebra (LIV) Index as a Supplemental Indicator to Lastly Touched Vertebra for Selection of ' +
          'the Fusion Level in Main Thoracic Adolescent Idiopathic Scoliosis. Global Spine J 16(2):956-964.',
        pmid: '40820306',
        doi: '10.1177/21925682251366988',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-placement-checks-uiv-tilt-40020691-t1-uiv-tilt-correlation',
    feature: 'Published placement checks and entry observations — t1 uiv tilt correlation',
    publishedNumber:
      'key t1-uiv-tilt-correlation, group t1-tilt-proxy: 0.708 correlation. Correlation coefficient r = 0.708 ' +
      'between postoperative UIV tilt and postoperative T1 tilt, not a ratio of angles. n = 151 (107 Lenke 1, 44 ' +
      'Lenke 2).',
    population:
      'uiv-tilt-40020691: 151 Lenke 1–2 AIS patients, 107 Lenke 1 and 44 Lenke 2; postoperative UIV tilt ' +
      'correlated with T1 tilt. Sample: 151 patients. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Wei Chan CY, Hong NC, Chandirasegaran S, Chiu CK, Kwan MK. 2025. The Influence of Relative Curve ' +
          'Correction and Upper Instrumented Vertebra (UIV) Tilt Angle on Post-operative Shoulder Balance Following ' +
          'Posterior Spinal Fusion (PSF) in Lenke Type 1 and 2 Adolescent Idiopathic Scoliosis (AIS) Patients. ' +
          'Global Spine J 15(8):3601-3611.',
        pmid: '40020691',
        doi: '10.1177/21925682251325171',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-placement-checks-brooks-2018-shoulder-imbalance',
    feature: 'Published placement checks and entry observations — shoulder imbalance',
    publishedNumber:
      'key shoulder-imbalance, group uiv-t2: 45 percent. key shoulder-imbalance, group uiv-t3: 48 percent. key ' +
      'shoulder-imbalance, group uiv-t4: 34 percent. Shoulder imbalance >1cm; result regardless of preoperative ' +
      'elevated shoulder.',
    population:
      'brooks-2018: Multicentre main thoracic fusion cohort; shoulder imbalance >1 cm. Sample: 626 main thoracic ' +
      'fusions. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Brooks JT, Bastrom TP, Bartley CE, Lonner BS, Shah SA, Miyanji F, Asghar J, Newton PO, Yaszay B, Harms ' +
          'Study Group. 2018. In Search of the Ever-Elusive Postoperative Shoulder Balance: Is the T2 UIV the Key? ' +
          'Spine Deform 6(6):707-711.',
        pmid: '30348348',
        doi: '10.1016/j.jspd.2018.03.010',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-placement-checks-takeshita-2010-thoracic-left-pedicle-aorta-angle',
    feature: 'Published placement checks and entry observations — thoracic left pedicle aorta angle',
    publishedNumber:
      'key thoracic-left-pedicle-aorta-angle, group right-thoracic: 29.7 degrees. Thoracic mean, not ' +
      'level-specific; warning levels T4–T5 and T10–T12 remain text in this qualifier.',
    population:
      'takeshita-2010: Right thoracic curves; left pedicle–aorta relation. Sample: 24 patients; 293 vertebrae. ' +
      '[peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Takeshita K, Maruyama T, Ono T, Ogihara S, Chikuda H, Shoda N, Nakao Y, Matsudaira K, Seichi A, Nakamura ' +
          'K. 2010. New parameters to represent the position of the aorta relative to the spine for pedicle screw ' +
          'placement. Eur Spine J 19(5):815-820.',
        pmid: '20127493',
        doi: '10.1007/s00586-010-1298-9',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-placement-checks-lehman-2003-straight-forward-pull-out-gain',
    feature: 'Published placement checks and entry observations — straight forward pull out gain',
    publishedNumber:
      'key straight-forward-pull-out-gain, group straight-forward: 27 percent. 30 cadaver thoracic vertebrae, ' +
      'compared with anatomic trajectory.',
    population:
      'lehman-2003: Cadaver thoracic vertebrae; straight-forward compared with anatomic sagittal trajectory. ' +
      'Sample: 30 cadaver thoracic vertebrae. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Lehman RA, Polly DW, Kuklo TR, Cunningham B, Kirk KL, Belmont PJ. 2003. Straight-forward versus anatomic ' +
          'trajectory technique of thoracic pedicle screw fixation: a biomechanical analysis. Spine (Phila Pa 1976) ' +
          '28(18):2058-2065.',
        pmid: '14501914',
        doi: '10.1097/01.brs.0000087743.57439.4f',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-placement-checks-fennell-2014-uniform-medial-angle',
    feature: 'Published placement checks and entry observations — uniform medial angle',
    publishedNumber:
      'key uniform-medial-angle, group t1-t2: 30 degrees. key uniform-medial-angle, group t3-t12: 20 degrees. ' +
      'Approximately 30 degrees at T1–T2 and 20 degrees at T3–T12 for a uniform medial trajectory. 33 ' +
      'consecutive patients, 219 screws; not an AIS cohort and not tested in severe scoliotic spines.',
    population:
      'fennell-2014: Thoracic screw placement in 33 consecutive patients, not an AIS cohort; the technique was ' +
      'not tested in severe scoliotic spines. Sample: 33 consecutive patients; 219 screws. [peer-reviewed; ' +
      'abstract].',
    sources: [
      {
        citation: 'Fennell VS, Palejwala S, Skoch J, Stidd DA, Baaj AA. 2014. J Neurosurg Spine 21(5):778-784.',
        pmid: '25147977',
        doi: '10.3171/2014.7.spine1489',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-placement-checks-peng-2023-uniform-medial-angle',
    feature: 'Published placement checks and entry observations — uniform medial angle',
    publishedNumber:
      'key uniform-medial-angle, group peng-t1-t2: 30 degrees. key uniform-medial-angle, group peng-t3-t12: 20 ' +
      'degrees. Inward horizontal direction: 30 degrees at T1–T2 and 20 degrees at T3–T12; screws parallel to ' +
      'upper endplate. Mixed thoracic diseases (stenosis, deformity, fractures, infection and others), not an ' +
      'AIS cohort; 118 patients, 1,315 screws. Angles were checked on radiographs only.',
    population:
      'peng-2023: Mixed thoracic diseases including stenosis, deformity, fractures, infection and others; not an ' +
      'AIS cohort. Thoracic screw technique uses the 30°/20° scheme and upper-endplate-parallel aim. Sample: 118 ' +
      'patients; 1,315 screws. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Peng F, Gao M, Li Q, Jiang Z, Chen F, Sun M, Lai Y, Wang H, Wang X, Li T, Zhang W. 2023. Supraspinous ' +
          'ligament arc tangent guided freehand thoracic pedicle screw insertion technique: high parallelism between ' +
          'screws and upper endplate. Front Surg 10:1219816.',
        pmid: '37609000',
        doi: '10.3389/fsurg.2023.1219816',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-placement-checks-sato-2022-patients-too-narrow',
    feature: 'Published placement checks and entry observations — patients too narrow',
    publishedNumber:
      'key patients-too-narrow, group apex-one-4mm-expansion: 26.3 percent. 15 patients; concave apical pedicle ' +
      'cannot take a 4 mm screw even with 25 percent expansion. Distinct condition from outer width below 4 mm.',
    population:
      'sato-2022: Japanese AIS, enrolled ages 12–20 (inclusion range 10–20), mean age 16.2; 49 girls and 8 boys; ' +
      'Lenke 1/2 right main thoracic curves; 0.5 mm CT. Right is always convex and left always concave. Sample: ' +
      '57 adolescents; 684 pedicles; 15 patients in the 26.3 percent group. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Sato T, Nojiri H, Okuda T, Miyagawa K, Kobayashi N, Takahashi R, Shimura A, Tamagawa S, Ohara Y, Hara T, ' +
          'Ishijima M. 2022. Three-dimensional morphological analysis of the thoracic pedicle and related ' +
          'radiographic factors in adolescent idiopathic scoliosis. BMC Musculoskelet Disord 23(1):847.',
        pmid: '36068555',
        doi: '10.1186/s12891-022-05799-4',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-placement-checks-sato-2022-patients-under-outer-width',
    feature: 'Published placement checks and entry observations — patients under outer width',
    publishedNumber:
      'key patients-under-outer-width, group apex-one-under-4mm: 71.9 percent. Concave outer width under 4 mm; ' +
      'no expansion condition.',
    population:
      'sato-2022: Japanese AIS, enrolled ages 12–20 (inclusion range 10–20), mean age 16.2; 49 girls and 8 boys; ' +
      'Lenke 1/2 right main thoracic curves; 0.5 mm CT. Right is always convex and left always concave. Sample: ' +
      '57 adolescents; 684 pedicles; 15 patients in the 26.3 percent group. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Sato T, Nojiri H, Okuda T, Miyagawa K, Kobayashi N, Takahashi R, Shimura A, Tamagawa S, Ohara Y, Hara T, ' +
          'Ishijima M. 2022. Three-dimensional morphological analysis of the thoracic pedicle and related ' +
          'radiographic factors in adolescent idiopathic scoliosis. BMC Musculoskelet Disord 23(1):847.',
        pmid: '36068555',
        doi: '10.1186/s12891-022-05799-4',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-placement-checks-takeshita-2010-aorta-warning-levels',
    feature: 'Published placement checks and entry observations — aorta warning levels',
    publishedNumber:
      'key aorta-warning-levels, group right-thoracic-left-upper: T4 to T5 levels. key aorta-warning-levels, ' +
      'group right-thoracic-left-lower: T10 to T12 levels. Left pedicles in right thoracic curves; no mirrored ' +
      'levels given.',
    population:
      'takeshita-2010: Right thoracic curves; left pedicle–aorta relation. Sample: 24 patients; 293 vertebrae. ' +
      '[peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Takeshita K, Maruyama T, Ono T, Ogihara S, Chikuda H, Shoda N, Nakao Y, Matsudaira K, Seichi A, Nakamura ' +
          'K. 2010. New parameters to represent the position of the aorta relative to the spine for pedicle screw ' +
          'placement. Eur Spine J 19(5):815-820.',
        pmid: '20127493',
        doi: '10.1007/s00586-010-1298-9',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-placement-checks-cerny-2014-pedicle-method-visibility-limit',
    feature: 'Published placement checks and entry observations — pedicle method visibility limit',
    publishedNumber:
      'key pedicle-method-visibility-limit, group cerny-pedicle-shadow: 30 degrees. About 30 degrees, with ' +
      'concave pedicle invisible beyond this rotation. This is not a published threshold on convergence-band ' +
      'width.',
    population:
      'cerny-2014: Front-film pedicle-based rotation measurement method evaluated by the author, 2 experts and ' +
      '10 students. Sample: 5,580 evaluations; author, 2 experts and 10 students; patient/specimen count not ' +
      'supplied in the earlier scout note. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Cerny P, Marik I, Pallova I. 2014. The radiographic method for evaluation of axial vertebral rotation - ' +
          'presentation of the new method. Scoliosis 9:11.',
        pmid: '25120581',
        doi: '10.1186/1748-7161-9-11',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-placement-checks-chen-2025-age-upper-bound',
    feature: 'Published placement checks and entry observations — age upper bound',
    publishedNumber:
      'key age-upper-bound, group liv-index-under-15: 15 years. Under 15 years means strictly less than 15 ' +
      'years; subgroup boundary for specificity 94 percent and PPV 77 percent. Subgroup n not supplied; ' +
      'derivation cohort only, full text.',
    population:
      'chen-2025: Lenke 1–2 with LIV at or below LTV/LSTV; derivation cohort only. Sample: 112 patients in the ' +
      'full cohort; 10 with adding-on; under-15 subgroup n not supplied. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Chen IH, Chen CW, Huang CC, Hsu JY, Wang PY, Lee YF, Yeh YC, Lai PL, Hu MH, Yang SH. 2026. Lowest ' +
          'Instrumented Vertebra (LIV) Index as a Supplemental Indicator to Lastly Touched Vertebra for Selection of ' +
          'the Fusion Level in Main Thoracic Adolescent Idiopathic Scoliosis. Global Spine J 16(2):956-964.',
        pmid: '40820306',
        doi: '10.1177/21925682251366988',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-placement-checks-uiv-tilt-40020691-medial-shoulder-imbalance',
    feature: 'Published placement checks and entry observations — medial shoulder imbalance',
    publishedNumber:
      'key medial-shoulder-imbalance, group uiv-tilt-cohort: 27.8 percent. Medial shoulder imbalance 27.8 ' +
      'percent; n = 151, Lenke 1–2, abstract. Distinct from neck and lateral shoulder imbalance.',
    population:
      'uiv-tilt-40020691: 151 Lenke 1–2 AIS patients, 107 Lenke 1 and 44 Lenke 2; postoperative UIV tilt ' +
      'correlated with T1 tilt. Sample: 151 patients. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Wei Chan CY, Hong NC, Chandirasegaran S, Chiu CK, Kwan MK. 2025. The Influence of Relative Curve ' +
          'Correction and Upper Instrumented Vertebra (UIV) Tilt Angle on Post-operative Shoulder Balance Following ' +
          'Posterior Spinal Fusion (PSF) in Lenke Type 1 and 2 Adolescent Idiopathic Scoliosis (AIS) Patients. ' +
          'Global Spine J 15(8):3601-3611.',
        pmid: '40020691',
        doi: '10.1177/21925682251325171',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-placement-checks-uiv-tilt-40020691-neck-imbalance',
    feature: 'Published placement checks and entry observations — neck imbalance',
    publishedNumber:
      'key neck-imbalance, group uiv-tilt-cohort: 23.2 percent. Neck imbalance 23.2 percent; n = 151, Lenke 1–2, ' +
      'abstract. Distinct from medial and lateral shoulder imbalance.',
    population:
      'uiv-tilt-40020691: 151 Lenke 1–2 AIS patients, 107 Lenke 1 and 44 Lenke 2; postoperative UIV tilt ' +
      'correlated with T1 tilt. Sample: 151 patients. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Wei Chan CY, Hong NC, Chandirasegaran S, Chiu CK, Kwan MK. 2025. The Influence of Relative Curve ' +
          'Correction and Upper Instrumented Vertebra (UIV) Tilt Angle on Post-operative Shoulder Balance Following ' +
          'Posterior Spinal Fusion (PSF) in Lenke Type 1 and 2 Adolescent Idiopathic Scoliosis (AIS) Patients. ' +
          'Global Spine J 15(8):3601-3611.',
        pmid: '40020691',
        doi: '10.1177/21925682251325171',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-placement-checks-uiv-tilt-40020691-lateral-shoulder-imbalance',
    feature: 'Published placement checks and entry observations — lateral shoulder imbalance',
    publishedNumber:
      'key lateral-shoulder-imbalance, group uiv-tilt-cohort: 9.9 percent. Lateral shoulder imbalance 9.9 ' +
      'percent; n = 151, Lenke 1–2, abstract. Distinct from neck and medial shoulder imbalance.',
    population:
      'uiv-tilt-40020691: 151 Lenke 1–2 AIS patients, 107 Lenke 1 and 44 Lenke 2; postoperative UIV tilt ' +
      'correlated with T1 tilt. Sample: 151 patients. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Wei Chan CY, Hong NC, Chandirasegaran S, Chiu CK, Kwan MK. 2025. The Influence of Relative Curve ' +
          'Correction and Upper Instrumented Vertebra (UIV) Tilt Angle on Post-operative Shoulder Balance Following ' +
          'Posterior Spinal Fusion (PSF) in Lenke Type 1 and 2 Adolescent Idiopathic Scoliosis (AIS) Patients. ' +
          'Global Spine J 15(8):3601-3611.',
        pmid: '40020691',
        doi: '10.1177/21925682251325171',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-placement-checks-cui-2012-medial-wall-danger-depth',
    feature: 'Published placement checks and entry observations — medial wall danger depth',
    publishedNumber:
      'key medial-wall-danger-depth, group cui-probe: 8 to 10 mm. Probe-path depth 8–10 mm where medial wall ' +
      'perforation risk rises; 30 AIS CTs, abstract. This is along the probe path, not a patient-frame ' +
      'coordinate or cortex thickness.',
    population:
      'cui-2012: AIS CT investigation of medial-wall perforation risk along the probe path. Sample: 30 AIS ' +
      'patients; CTs. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Cui G, Watanabe K, Hosogane N, Tsuji T, Ishii K, Nakamura M, Toyama Y, Chiba K, Lenke LG, Matsumoto M. ' +
          '2012. Surg Radiol Anat 34(3):209-216.',
        pmid: '21739245',
        doi: '10.1007/s00276-011-0849-z',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-placement-checks-etemadifar-2017-entry-lateral-offset',
    feature: 'Published placement checks and entry observations — entry lateral offset',
    publishedNumber:
      'key entry-lateral-offset, group kim-t1-t10: 2 mm. About 2 mm lateral to the midpoint of the superior ' +
      'facet, at the superior border of the transverse process, for T1–T10. Kim/Lenke technique described ' +
      'through Etemadifar 2017, full text; source series 38 AIS patients and 720 screws. This is a landmark ' +
      'technique, not an entry found on a fitted template.',
    population:
      'etemadifar-2017: AIS series; screw positions checked by CT. Sample: 38 AIS patients; 720 freehand screws. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation: 'Etemadifar M, Jamalaldini M. 2017. Adv Biomed Res 6:19.',
        pmid: '28349022',
        doi: '10.4103/2277-9175.201331',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-rod-choices-wilke-2024-diameter',
    feature: 'Rod material, diameter and contour observations — diameter',
    publishedNumber: 'key diameter, group surgeon-survey: 5.5 mm.',
    population:
      'wilke-2024: Surgeons choosing AIS classification, fusion levels and implants from standing and bending ' +
      'films. Sample: 32 surgeons; 15 countries; 12 cases. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Wilke HJ, Großkinsky M, Ruf M, Schlager B. 2024. Range of international surgical strategies for ' +
          'adolescent idiopathic scoliosis: Evaluation of a multi-center survey. JOR Spine 7(2):e1324.',
        pmid: '38633662',
        doi: '10.1002/jsp2.1324',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-rod-choices-wilke-2024-diameter-choice-share',
    feature: 'Rod material, diameter and contour observations — diameter choice share',
    publishedNumber: 'key diameter-choice-share, group surgeon-survey: 56 percent.',
    population:
      'wilke-2024: Surgeons choosing AIS classification, fusion levels and implants from standing and bending ' +
      'films. Sample: 32 surgeons; 15 countries; 12 cases. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Wilke HJ, Großkinsky M, Ruf M, Schlager B. 2024. Range of international surgical strategies for ' +
          'adolescent idiopathic scoliosis: Evaluation of a multi-center survey. JOR Spine 7(2):e1324.',
        pmid: '38633662',
        doi: '10.1002/jsp2.1324',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-rod-choices-wilke-2024-material-choice-share',
    feature: 'Rod material, diameter and contour observations — material choice share',
    publishedNumber:
      'key material-choice-share, group cobalt-chrome: 53 percent. key material-choice-share, group ' +
      'titanium-alloy: 28 percent.',
    population:
      'wilke-2024: Surgeons choosing AIS classification, fusion levels and implants from standing and bending ' +
      'films. Sample: 32 surgeons; 15 countries; 12 cases. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Wilke HJ, Großkinsky M, Ruf M, Schlager B. 2024. Range of international surgical strategies for ' +
          'adolescent idiopathic scoliosis: Evaluation of a multi-center survey. JOR Spine 7(2):e1324.',
        pmid: '38633662',
        doi: '10.1002/jsp2.1324',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-rod-choices-wang-2016-diameter',
    feature: 'Rod material, diameter and contour observations — diameter',
    publishedNumber:
      'key diameter, group cobalt-chrome: 5.5, 6.0 mm. Concave/convex rod configurations were 5.5/5.5 and ' +
      '6.0/5.5 mm cobalt-chrome in 10 AIS models. The 6.0 mm rod was tested only on the concave side; no ' +
      'inferred stiffness increment is stored.',
    population:
      'wang-2016: Ten AIS patient-specific multibody simulations; cobalt-chrome concave/convex configurations ' +
      '5.5/5.5 mm and 6.0/5.5 mm. Sample: 10 patient-specific models. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Wang X, Boyer L, Le Naveaux F, Schwend RM, Aubin CE. 2016. How does differential rod contouring ' +
          'contribute to 3-dimensional correction and affect the bone-screw forces in adolescent idiopathic ' +
          'scoliosis instrumentation? Clin Biomech (Bristol) 39:115-121.',
        pmid: '27750078',
        doi: '10.1016/j.clinbiomech.2016.10.002',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-rod-choices-abe-2015-diameter',
    feature: 'Rod material, diameter and contour observations — diameter',
    publishedNumber:
      'key diameter, group ti6al7nb: 6 mm. 20 AIS patients (1 boy, 19 girls), all Lenke 1, operated June ' +
      '2009–June 2011. Very probably the same 20-patient cohort as Salmingo 2014 (same authors and identical ' +
      'concave angles); these sources are not independent.',
    population:
      'abe-2015: AIS, one boy and 19 girls, all Lenke 1; mean age 14.8; operations June 2009–June 2011; in vivo ' +
      'rod deformation followed by an elasto-plastic finite element model; 6 mm Ti6Al7Nb rods. Sample: 20 AIS ' +
      'patients. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Abe Y, Ito M, Abumi K, Sudo H, Salmingo R, Tadano S. 2015. Scoliosis corrective force estimation from the ' +
          'implanted rod deformation using 3D-FEM analysis. Scoliosis 10(Suppl 2):S2.',
        pmid: '25810754',
        doi: '10.1186/1748-7161-10-S2-S2',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-rod-choices-cidambi-2012-diameter',
    feature: 'Rod material, diameter and contour observations — diameter',
    publishedNumber: 'key diameter, group uhss-steel: 5.5 mm.',
    population:
      'cidambi-2012: 5.5 mm ultra-high-strength steel rod contour-loss series. Sample: 27 patients. ' +
      '[peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Cidambi KR, Glaser DA, Bastrom TP, Nunn TN, Ono T, Newton PO. 2012. Postoperative changes in spinal rod ' +
          'contour in adolescent idiopathic scoliosis: an in vivo deformation study. Spine (Phila Pa 1976) ' +
          '37(18):1566-1572.',
        pmid: '22426445',
        doi: '10.1097/brs.0b013e318252ccbe',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-rod-choices-cidambi-2012-overcontour',
    feature: 'Rod material, diameter and contour observations — overcontour',
    publishedNumber:
      'key overcontour, group concave: 20 degrees. About 20 degrees as reported; applying it to another material ' +
      'or patient is a later rule.',
    population:
      'cidambi-2012: 5.5 mm ultra-high-strength steel rod contour-loss series. Sample: 27 patients. ' +
      '[peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Cidambi KR, Glaser DA, Bastrom TP, Nunn TN, Ono T, Newton PO. 2012. Postoperative changes in spinal rod ' +
          'contour in adolescent idiopathic scoliosis: an in vivo deformation study. Spine (Phila Pa 1976) ' +
          '37(18):1566-1572.',
        pmid: '22426445',
        doi: '10.1097/brs.0b013e318252ccbe',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-rod-choices-cidambi-2012-concave-change',
    feature: 'Rod material, diameter and contour observations — concave change',
    publishedNumber:
      'key concave-change, group uhss-steel: -21 degrees. Minus 21 degrees as printed in the contour-loss table; ' +
      'no final-angle arithmetic or pooled 16–22 degree interval is inferred.',
    population:
      'cidambi-2012: 5.5 mm ultra-high-strength steel rod contour-loss series. Sample: 27 patients. ' +
      '[peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Cidambi KR, Glaser DA, Bastrom TP, Nunn TN, Ono T, Newton PO. 2012. Postoperative changes in spinal rod ' +
          'contour in adolescent idiopathic scoliosis: an in vivo deformation study. Spine (Phila Pa 1976) ' +
          '37(18):1566-1572.',
        pmid: '22426445',
        doi: '10.1097/brs.0b013e318252ccbe',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-rod-choices-cidambi-2012-convex-change',
    feature: 'Rod material, diameter and contour observations — convex change',
    publishedNumber: 'key convex-change, group uhss-steel: 2 degrees. Plus 2 degrees, not a symmetric ±2 distribution.',
    population:
      'cidambi-2012: 5.5 mm ultra-high-strength steel rod contour-loss series. Sample: 27 patients. ' +
      '[peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Cidambi KR, Glaser DA, Bastrom TP, Nunn TN, Ono T, Newton PO. 2012. Postoperative changes in spinal rod ' +
          'contour in adolescent idiopathic scoliosis: an in vivo deformation study. Spine (Phila Pa 1976) ' +
          '37(18):1566-1572.',
        pmid: '22426445',
        doi: '10.1097/brs.0b013e318252ccbe',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-rod-choices-borrelli-2025-screw-cantilever-length',
    feature: 'Rod material, diameter and contour observations — screw cantilever length',
    publishedNumber:
      'key screw-cantilever-length, group lumbar: 15 mm. Published lumbar screw bushing cantilever free length ' +
      'of 15 mm under complete osseointegration, modelled with CFR-PEEK screws of 6.5 mm diameter. The rods were ' +
      'modelled an average 15 mm from the posterior pedicles; this is a model offset, not a measured adolescent ' +
      'head offset. Borrowing it as head offset is a project choice.',
    population:
      'borrelli-2025: Lumbar multibody model with a 6.5 mm CFR-PEEK screw represented as a 15 mm cantilever; ' +
      'full cortical osseointegration assumed. Sample: Model sample count not supplied. [peer-reviewed; ' +
      'full-text].',
    sources: [
      {
        citation:
          'Borrelli S, Putame G, Marone S, Ferro A, Audenino AL, Terzini M. 2025. Combined Rigid-Flexible Multibody ' +
          'Analysis Reveals Reduced Pedicle Screw Loads in Short-Segment Fixation for Decompressed Lumbar Spine ' +
          'Stabilization. Ann Biomed Eng 53(5):1257-1269.',
        pmid: '40082329',
        doi: '10.1007/s10439-025-03706-1',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-rod-choices-cidambi-2012-before-angle',
    feature: 'Rod material, diameter and contour observations — before angle',
    publishedNumber:
      'key before-angle, group cidambi-concave: 45.6 degrees. key before-angle, group cidambi-convex: 31.4 ' +
      'degrees. As printed; no subtraction recomputes a loss or repairs internal inconsistencies.',
    population:
      'cidambi-2012: 5.5 mm ultra-high-strength steel rod contour-loss series. Sample: 27 patients. ' +
      '[peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Cidambi KR, Glaser DA, Bastrom TP, Nunn TN, Ono T, Newton PO. 2012. Postoperative changes in spinal rod ' +
          'contour in adolescent idiopathic scoliosis: an in vivo deformation study. Spine (Phila Pa 1976) ' +
          '37(18):1566-1572.',
        pmid: '22426445',
        doi: '10.1097/brs.0b013e318252ccbe',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-rod-choices-salmingo-2014-before-angle',
    feature: 'Rod material, diameter and contour observations — before angle',
    publishedNumber:
      'key before-angle, group salmingo-concave: 33.6 degrees. key before-angle, group salmingo-convex: 25.5 ' +
      'degrees. As printed; no subtraction recomputes a loss or repairs internal inconsistencies. Very probably ' +
      'the same 20-patient cohort as Abe 2015 (same authors and identical concave angles); these sources are not ' +
      'independent.',
    population:
      'salmingo-2014: Twenty AIS patients, mean age 14; rod contour-loss series; titanium designation reported ' +
      'secondarily through Abe. Sample: 20 patients. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Salmingo RA, Tadano S, Abe Y, Ito M. 2014. Influence of implant rod curvature on sagittal correction of ' +
          'scoliosis deformity. Spine J 14(8):1432-1439.',
        pmid: '24275616',
        doi: '10.1016/j.spinee.2013.08.042',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-rod-choices-salmingo-2014-after-angle',
    feature: 'Rod material, diameter and contour observations — after angle',
    publishedNumber:
      'key after-angle, group salmingo-concave: 17.8 degrees. key after-angle, group salmingo-convex: 23.9 ' +
      'degrees. As printed; no subtraction recomputes a loss or repairs internal inconsistencies. Very probably ' +
      'the same 20-patient cohort as Abe 2015 (same authors and identical concave angles); these sources are not ' +
      'independent.',
    population:
      'salmingo-2014: Twenty AIS patients, mean age 14; rod contour-loss series; titanium designation reported ' +
      'secondarily through Abe. Sample: 20 patients. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Salmingo RA, Tadano S, Abe Y, Ito M. 2014. Influence of implant rod curvature on sagittal correction of ' +
          'scoliosis deformity. Spine J 14(8):1432-1439.',
        pmid: '24275616',
        doi: '10.1016/j.spinee.2013.08.042',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-rod-choices-salmingo-2014-contour-loss',
    feature: 'Rod material, diameter and contour observations — contour loss',
    publishedNumber:
      'key contour-loss, group salmingo-concave: 15.8 degrees. key contour-loss, group salmingo-convex: 1.6 ' +
      'degrees. As printed; no subtraction recomputes a loss or repairs internal inconsistencies. Very probably ' +
      'the same 20-patient cohort as Abe 2015 (same authors and identical concave angles); these sources are not ' +
      'independent.',
    population:
      'salmingo-2014: Twenty AIS patients, mean age 14; rod contour-loss series; titanium designation reported ' +
      'secondarily through Abe. Sample: 20 patients. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Salmingo RA, Tadano S, Abe Y, Ito M. 2014. Influence of implant rod curvature on sagittal correction of ' +
          'scoliosis deformity. Spine J 14(8):1432-1439.',
        pmid: '24275616',
        doi: '10.1016/j.spinee.2013.08.042',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-rod-choices-sia-2019-before-angle',
    feature: 'Rod material, diameter and contour observations — before angle',
    publishedNumber:
      'key before-angle, group sia-titanium: 60.5 median (IQR 10.4) degrees. key before-angle, group ' +
      'sia-cobalt-chrome: 51 median (IQR 26) degrees. Titanium concave rod, subgroup n = 10; Median (IQR) as ' +
      'printed in Tables II and III. Reductions are medians of per-patient changes and need not equal ' +
      'differences of group medians; no reduction is recomputed. Cobalt-chrome concave rod, subgroup n = 11; ' +
      'Median (IQR) as printed in Tables II and III. Reductions are medians of per-patient changes and need not ' +
      'equal differences of group medians; no reduction is recomputed.',
    population:
      'sia-2019: AIS, median age 16 for titanium and 17 for cobalt-chrome; 5.5 mm rods; concave rods only. ' +
      'Sample: 21 patients: titanium 10; cobalt-chrome 11. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Sia U, Tan BB, Teo YY, Wong CC. 2019. Post-implantation Deformation of Titanium Rod and Cobalt Chrome Rod ' +
          'in Adolescent Idiopathic Scoliosis. Malays Orthop J 13(1):14-19.',
        pmid: '31001378',
        doi: '10.5704/moj.1903.002',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-rod-choices-sia-2019-after-angle',
    feature: 'Rod material, diameter and contour observations — after angle',
    publishedNumber:
      'key after-angle, group sia-titanium: 37 median (IQR 11.6) degrees. key after-angle, group ' +
      'sia-cobalt-chrome: 28 median (IQR 22) degrees. Titanium concave rod, subgroup n = 10; Median (IQR) as ' +
      'printed in Tables II and III. Reductions are medians of per-patient changes and need not equal ' +
      'differences of group medians; no reduction is recomputed. Cobalt-chrome concave rod, subgroup n = 11; ' +
      'Median (IQR) as printed in Tables II and III. Reductions are medians of per-patient changes and need not ' +
      'equal differences of group medians; no reduction is recomputed.',
    population:
      'sia-2019: AIS, median age 16 for titanium and 17 for cobalt-chrome; 5.5 mm rods; concave rods only. ' +
      'Sample: 21 patients: titanium 10; cobalt-chrome 11. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Sia U, Tan BB, Teo YY, Wong CC. 2019. Post-implantation Deformation of Titanium Rod and Cobalt Chrome Rod ' +
          'in Adolescent Idiopathic Scoliosis. Malays Orthop J 13(1):14-19.',
        pmid: '31001378',
        doi: '10.5704/moj.1903.002',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-rod-choices-sia-2019-contour-loss',
    feature: 'Rod material, diameter and contour observations — contour loss',
    publishedNumber:
      'key contour-loss, group sia-titanium: 22.5 median (IQR 9.9) degrees. key contour-loss, group ' +
      'sia-cobalt-chrome: 17 median (IQR 7.7) degrees. Titanium concave rod, subgroup n = 10; Median (IQR) as ' +
      'printed in Tables II and III. Reductions are medians of per-patient changes and need not equal ' +
      'differences of group medians; no reduction is recomputed. Cobalt-chrome concave rod, subgroup n = 11; ' +
      'Median (IQR) as printed in Tables II and III. Reductions are medians of per-patient changes and need not ' +
      'equal differences of group medians; no reduction is recomputed.',
    population:
      'sia-2019: AIS, median age 16 for titanium and 17 for cobalt-chrome; 5.5 mm rods; concave rods only. ' +
      'Sample: 21 patients: titanium 10; cobalt-chrome 11. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Sia U, Tan BB, Teo YY, Wong CC. 2019. Post-implantation Deformation of Titanium Rod and Cobalt Chrome Rod ' +
          'in Adolescent Idiopathic Scoliosis. Malays Orthop J 13(1):14-19.',
        pmid: '31001378',
        doi: '10.5704/moj.1903.002',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-rod-choices-abe-2015-before-angle',
    feature: 'Rod material, diameter and contour observations — before angle',
    publishedNumber:
      'key before-angle, group abe-concave: 33.6 degrees. key before-angle, group abe-convex: 25.9 degrees. As ' +
      'printed; no subtraction recomputes a loss or repairs internal inconsistencies. 20 AIS patients (1 boy, 19 ' +
      'girls), all Lenke 1, operated June 2009–June 2011. Very probably the same 20-patient cohort as Salmingo ' +
      '2014 (same authors and identical concave angles); these sources are not independent.',
    population:
      'abe-2015: AIS, one boy and 19 girls, all Lenke 1; mean age 14.8; operations June 2009–June 2011; in vivo ' +
      'rod deformation followed by an elasto-plastic finite element model; 6 mm Ti6Al7Nb rods. Sample: 20 AIS ' +
      'patients. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Abe Y, Ito M, Abumi K, Sudo H, Salmingo R, Tadano S. 2015. Scoliosis corrective force estimation from the ' +
          'implanted rod deformation using 3D-FEM analysis. Scoliosis 10(Suppl 2):S2.',
        pmid: '25810754',
        doi: '10.1186/1748-7161-10-S2-S2',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-rod-choices-abe-2015-after-angle',
    feature: 'Rod material, diameter and contour observations — after angle',
    publishedNumber:
      'key after-angle, group abe-concave: 17.8 degrees. key after-angle, group abe-convex: 23.8 degrees. As ' +
      'printed; no subtraction recomputes a loss or repairs internal inconsistencies. 20 AIS patients (1 boy, 19 ' +
      'girls), all Lenke 1, operated June 2009–June 2011. Very probably the same 20-patient cohort as Salmingo ' +
      '2014 (same authors and identical concave angles); these sources are not independent.',
    population:
      'abe-2015: AIS, one boy and 19 girls, all Lenke 1; mean age 14.8; operations June 2009–June 2011; in vivo ' +
      'rod deformation followed by an elasto-plastic finite element model; 6 mm Ti6Al7Nb rods. Sample: 20 AIS ' +
      'patients. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Abe Y, Ito M, Abumi K, Sudo H, Salmingo R, Tadano S. 2015. Scoliosis corrective force estimation from the ' +
          'implanted rod deformation using 3D-FEM analysis. Scoliosis 10(Suppl 2):S2.',
        pmid: '25810754',
        doi: '10.1186/1748-7161-10-S2-S2',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-rod-choices-cidambi-2012-deflection',
    feature: 'Rod material, diameter and contour observations — deflection',
    publishedNumber:
      'key deflection, group cidambi-concave: -13 mm. Concave UHSS steel rod deflection −13 mm, retaining the ' +
      'reported sign; n = 27, abstract.',
    population:
      'cidambi-2012: 5.5 mm ultra-high-strength steel rod contour-loss series. Sample: 27 patients. ' +
      '[peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Cidambi KR, Glaser DA, Bastrom TP, Nunn TN, Ono T, Newton PO. 2012. Postoperative changes in spinal rod ' +
          'contour in adolescent idiopathic scoliosis: an in vivo deformation study. Spine (Phila Pa 1976) ' +
          '37(18):1566-1572.',
        pmid: '22426445',
        doi: '10.1097/brs.0b013e318252ccbe',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-rod-choices-cidambi-2012-before-kyphosis',
    feature: 'Rod material, diameter and contour observations — before kyphosis',
    publishedNumber:
      'key before-kyphosis, group cidambi-t5-t12: 19 ± 14 (spread statistic not stated) degrees. Before: T5–T12 ' +
      'kyphosis 19 ± 14 degrees; the note names neither statistic. Paired cohort n = 27; abstract. Change ' +
      'reported as not significant.',
    population:
      'cidambi-2012: 5.5 mm ultra-high-strength steel rod contour-loss series. Sample: 27 patients. ' +
      '[peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Cidambi KR, Glaser DA, Bastrom TP, Nunn TN, Ono T, Newton PO. 2012. Postoperative changes in spinal rod ' +
          'contour in adolescent idiopathic scoliosis: an in vivo deformation study. Spine (Phila Pa 1976) ' +
          '37(18):1566-1572.',
        pmid: '22426445',
        doi: '10.1097/brs.0b013e318252ccbe',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-rod-choices-cidambi-2012-after-kyphosis',
    feature: 'Rod material, diameter and contour observations — after kyphosis',
    publishedNumber:
      'key after-kyphosis, group cidambi-t5-t12: 22 ± 6 (spread statistic not stated) degrees. After: T5–T12 ' +
      'kyphosis 22 ± 6 degrees; the note names neither statistic. Paired cohort n = 27; abstract. Change ' +
      'reported as not significant.',
    population:
      'cidambi-2012: 5.5 mm ultra-high-strength steel rod contour-loss series. Sample: 27 patients. ' +
      '[peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Cidambi KR, Glaser DA, Bastrom TP, Nunn TN, Ono T, Newton PO. 2012. Postoperative changes in spinal rod ' +
          'contour in adolescent idiopathic scoliosis: an in vivo deformation study. Spine (Phila Pa 1976) ' +
          '37(18):1566-1572.',
        pmid: '22426445',
        doi: '10.1097/brs.0b013e318252ccbe',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-rod-choices-salmingo-2014-curvature-correction-correlation',
    feature: 'Rod material, diameter and contour observations — curvature correction correlation',
    publishedNumber:
      'key curvature-correction-correlation, group salmingo-sagittal: 0.6 correlation. Correlation r = 0.60 ' +
      'between rod curvature and sagittal correction; n = 20, abstract. This is not a correction multiplier. ' +
      'Very probably the same 20-patient cohort as Abe 2015 (same authors and identical concave angles); these ' +
      'sources are not independent.',
    population:
      'salmingo-2014: Twenty AIS patients, mean age 14; rod contour-loss series; titanium designation reported ' +
      'secondarily through Abe. Sample: 20 patients. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Salmingo RA, Tadano S, Abe Y, Ito M. 2014. Influence of implant rod curvature on sagittal correction of ' +
          'scoliosis deformity. Spine J 14(8):1432-1439.',
        pmid: '24275616',
        doi: '10.1016/j.spinee.2013.08.042',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-rod-choices-sia-2019-diameter',
    feature: 'Rod material, diameter and contour observations — diameter',
    publishedNumber:
      'key diameter, group sia-all: 5.5 mm. Published rod diameter 5.5 mm in the n = 21 cohort: titanium n = 10 ' +
      'and cobalt-chrome n = 11; full text.',
    population:
      'sia-2019: AIS, median age 16 for titanium and 17 for cobalt-chrome; 5.5 mm rods; concave rods only. ' +
      'Sample: 21 patients: titanium 10; cobalt-chrome 11. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Sia U, Tan BB, Teo YY, Wong CC. 2019. Post-implantation Deformation of Titanium Rod and Cobalt Chrome Rod ' +
          'in Adolescent Idiopathic Scoliosis. Malays Orthop J 13(1):14-19.',
        pmid: '31001378',
        doi: '10.5704/moj.1903.002',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-rod-choices-sia-2019-before-kyphosis',
    feature: 'Rod material, diameter and contour observations — before kyphosis',
    publishedNumber:
      'key before-kyphosis, group sia-titanium: 20 median (IQR not supplied) degrees. key before-kyphosis, group ' +
      'sia-cobalt-chrome: 25 median (IQR not supplied) degrees. Titanium before kyphosis median 20 degrees; ' +
      'titanium subgroup n = 10, full text. Tables II and III label values Median (IQR), but this audit does not ' +
      'transcribe this kyphosis IQR; it remains absent pending transcription. Concave rods only. Cobalt-chrome ' +
      'before kyphosis median 25 degrees; cobalt-chrome subgroup n = 11, full text. Tables II and III label ' +
      'values Median (IQR), but this audit does not transcribe this kyphosis IQR; it remains absent pending ' +
      'transcription. Concave rods only.',
    population:
      'sia-2019: AIS, median age 16 for titanium and 17 for cobalt-chrome; 5.5 mm rods; concave rods only. ' +
      'Sample: 21 patients: titanium 10; cobalt-chrome 11. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Sia U, Tan BB, Teo YY, Wong CC. 2019. Post-implantation Deformation of Titanium Rod and Cobalt Chrome Rod ' +
          'in Adolescent Idiopathic Scoliosis. Malays Orthop J 13(1):14-19.',
        pmid: '31001378',
        doi: '10.5704/moj.1903.002',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-rod-choices-sia-2019-after-kyphosis',
    feature: 'Rod material, diameter and contour observations — after kyphosis',
    publishedNumber:
      'key after-kyphosis, group sia-titanium: 26 median (IQR not supplied) degrees. key after-kyphosis, group ' +
      'sia-cobalt-chrome: 23 median (IQR not supplied) degrees. Titanium after kyphosis median 26 degrees; ' +
      'titanium subgroup n = 10, full text. Tables II and III label values Median (IQR), but this audit does not ' +
      'transcribe this kyphosis IQR; it remains absent pending transcription. Concave rods only. Cobalt-chrome ' +
      'after kyphosis median 23 degrees; cobalt-chrome subgroup n = 11, full text. Tables II and III label ' +
      'values Median (IQR), but this audit does not transcribe this kyphosis IQR; it remains absent pending ' +
      'transcription. Concave rods only.',
    population:
      'sia-2019: AIS, median age 16 for titanium and 17 for cobalt-chrome; 5.5 mm rods; concave rods only. ' +
      'Sample: 21 patients: titanium 10; cobalt-chrome 11. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Sia U, Tan BB, Teo YY, Wong CC. 2019. Post-implantation Deformation of Titanium Rod and Cobalt Chrome Rod ' +
          'in Adolescent Idiopathic Scoliosis. Malays Orthop J 13(1):14-19.',
        pmid: '31001378',
        doi: '10.5704/moj.1903.002',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-rod-choices-boissiere-2023-spine-rod-kyphosis-disparity',
    feature: 'Rod material, diameter and contour observations — spine rod kyphosis disparity',
    publishedNumber:
      'key spine-rod-kyphosis-disparity, group boissiere-concave: 5.5 degrees. Spine-to-rod kyphosis disparity ' +
      '5.5 degrees; n = 99, abstract. The note supplies no rod diameter for this cohort.',
    population:
      'boissiere-2023: Spine-to-rod kyphosis comparison; age and cohort composition not supplied in the note. ' +
      'Sample: 99; sample unit not named in the supplied table row. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Boissiere L, Bourghli A, Guevara-Villazon F, Pellisé F, Alanay A, Kleinstück F, Pizones J, Roscop C, ' +
          'Larrieu D, Obeid I. 2023. Rod Angulation Relationship with Thoracic Kyphosis after Adolescent Idiopathic ' +
          'Scoliosis Posterior Instrumentation. Children (Basel) 11(1):29.',
        pmid: '38255344',
        doi: '10.3390/children11010029',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-rod-choices-boissiere-2023-rod-less-kyphotic-share',
    feature: 'Rod material, diameter and contour observations — rod less kyphotic share',
    publishedNumber:
      'key rod-less-kyphotic-share, group boissiere-concave: 84 percent. Concave rod less kyphotic than the ' +
      'spine in 84 percent of the n = 99 cohort; abstract. This is not the percentage of rod flattening.',
    population:
      'boissiere-2023: Spine-to-rod kyphosis comparison; age and cohort composition not supplied in the note. ' +
      'Sample: 99; sample unit not named in the supplied table row. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Boissiere L, Bourghli A, Guevara-Villazon F, Pellisé F, Alanay A, Kleinstück F, Pizones J, Roscop C, ' +
          'Larrieu D, Obeid I. 2023. Rod Angulation Relationship with Thoracic Kyphosis after Adolescent Idiopathic ' +
          'Scoliosis Posterior Instrumentation. Children (Basel) 11(1):29.',
        pmid: '38255344',
        doi: '10.3390/children11010029',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-rod-choices-sia-2019-contour-loss-p-value',
    feature: 'Rod material, diameter and contour observations — contour loss p value',
    publishedNumber:
      'key contour-loss-p-value, group sia-material-comparison: 0.173 ratio. Reported p = 0.173 alongside the ' +
      'titanium/cobalt-chrome contour observations, n = 21 (Ti 10, CoCr 11). The research note does not name the ' +
      'statistical test; no test or significance conclusion is inferred.',
    population:
      'sia-2019: AIS, median age 16 for titanium and 17 for cobalt-chrome; 5.5 mm rods; concave rods only. ' +
      'Sample: 21 patients: titanium 10; cobalt-chrome 11. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Sia U, Tan BB, Teo YY, Wong CC. 2019. Post-implantation Deformation of Titanium Rod and Cobalt Chrome Rod ' +
          'in Adolescent Idiopathic Scoliosis. Malays Orthop J 13(1):14-19.',
        pmid: '31001378',
        doi: '10.5704/moj.1903.002',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-screw-grids-ramanathan-2026-diameter',
    feature: 'Published screw diameter and length choices — diameter',
    publishedNumber:
      'key diameter, group ramanathan-small: 4.0 to 4.5 mm. key diameter, group ramanathan-large: 5.5 to 6.5 mm. ' +
      'Reported size category; no intermediate values supplied.',
    population:
      'ramanathan-2026: AIS screw size selection compared across freehand and navigation. Sample: 137 freehand ' +
      'AIS cases; 137 O-arm-navigated AIS cases; one senior surgeon. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Ramanathan R, Luck C, Dede Ö, Ward T. 2026. O-arm navigation vs. freehand screw placement in adolescent ' +
          'idiopathic scoliosis: A comparative analysis of screw size and reoperation rates. Acta Orthop Traumatol ' +
          'Turc 60(2):e25525.',
        pmid: '42364200',
        doi: '10.5152/j.aott.2026.25525',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-screw-grids-jiang-2022-diameter',
    feature: 'Published screw diameter and length choices — diameter',
    publishedNumber:
      'key diameter, group jiang-t1-t4: 4.5 to 5.5 mm. key diameter, group jiang-t5-t8: 5.5 mm. key diameter, ' +
      'group jiang-t9-t12: 5.5 to 6.5 mm.',
    population:
      'jiang-2022: AIS, 25 girls and 9 boys, mean age 15.0; 30 Lenke 1 and 4 Lenke 2; freehand thoracic screws ' +
      'checked by CT. Sample: 34 patients; 495 screws. [peer-reviewed; full-text].',
    sources: [
      {
        citation: 'Jiang J, Chen X, Qiu Y, Wang B, Zhu ZZ, Yu Y. 2022. BMC Surg 22(1):379.',
        pmid: '36333715',
        doi: '10.1186/s12893-022-01827-1',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-screw-grids-sarwahi-2023-diameter',
    feature: 'Published screw diameter and length choices — diameter',
    publishedNumber: 'key diameter, group sarwahi-standard: 5.0 to 5.5 mm. key diameter, group sarwahi-large: 6.5 mm.',
    population:
      'sarwahi-2023: AIS, comparing 6.5 mm screws at all levels with 5.0–5.5 mm screws at all levels. Sample: ' +
      '412 patients. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Sarwahi V, Hasan S, Koutsogiannis P, Visahan K, Rao H, Patil A, Lo Y, Amaral T, DiMauro JP. 2023. Spine ' +
          '(Phila Pa 1976) 48(21):1544-1551.',
        pmid: '37134132',
        doi: '10.1097/brs.0000000000004700',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-screw-grids-grabala-2023-diameter',
    feature: 'Published screw diameter and length choices — diameter',
    publishedNumber:
      'key diameter, group grabala: 5.0, 5.5, 6.0, 6.5, 7.0 mm. Sizes individually listed against expansion ' +
      'percentages. Mixed-cause cohort n = 230: idiopathic 164, congenital 25, neuromuscular 23 and syndromic ' +
      '18; not AIS only.',
    population:
      'grabala-2023: Children aged 2–18 with mixed causes: idiopathic 164, congenital 25, neuromuscular 23, ' +
      'syndromic 18; MRI before and CT after surgery; single centre. Not an AIS-only cohort. Sample: 230 ' +
      'patients; 7,954 preoperative pedicles and 5,080 postoperative screws, different sets. [peer-reviewed; ' +
      'full-text].',
    sources: [
      {
        citation:
          'Grabala P, Helenius IJ, Grabala M, Shah SA. 2023. Influences of Increasing Pedicle Screw Diameter on ' +
          'Widening Vertebral Pedicle Size during Surgery in Spinal Deformities in Children and Adolescents without ' +
          'Higher Risk of Pedicle and Vertebral Breaches. J Clin Med 12(16):5368.',
        pmid: '37629410',
        doi: '10.3390/jcm12165368',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-screw-grids-balikci-2019-length-step',
    feature: 'Published screw diameter and length choices — length step',
    publishedNumber:
      'key length-step, group balikci: 2.5, 5 mm. The original note prints steps of 2.5 and 5 mm. Table 1 gives ' +
      'the standard and mid-length catalogue values retained individually below; their groups are not combined. ' +
      'Series n = 120, ages 13–54 (mean 19.2): AIS 86, congenital 17, adult 10 and neuromuscular 7; not an ' +
      'exclusively AIS cohort.',
    population:
      'balikci-2019: Mixed scoliosis series: AIS 86, congenital 17, adult 10, neuromuscular 7; ages 13–54, mean ' +
      '19.2. Diameter chosen using insertional torque. Sample: 120 patients, 86 with AIS; 2,846 screws. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Balikci T, Kıyak G, Heydar AM, Bawaneh MK, Bezer M. 2019. Mid-length Pedicle Screws in Posterior ' +
          'Instrumentation of Scoliosis. Asian Spine J 13(5):815-822.',
        pmid: '31079434',
        doi: '10.31616/asj.2018.0177',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-screw-grids-balikci-2019-length',
    feature: 'Published screw diameter and length choices — length',
    publishedNumber:
      'key length, group balikci-mid-27.5: 27.5 mm; printed 27.5. key length, group balikci-mid-32.5: 32.5 mm; ' +
      'printed 32.5. key length, group balikci-mid-37.5: 37.5 mm; printed 37.5. key length, group ' +
      'balikci-mid-42.5: 42.5 mm; printed 42.5. key length, group balikci-mid-47.5: 47.5 mm; printed 47.5. key ' +
      'length, group balikci-mid-52.5: 52.5 mm; printed 52.5. key length, group balikci-standard-25: 25 mm; ' +
      'printed 25. key length, group balikci-standard-30: 30 mm; printed 30. key length, group ' +
      'balikci-standard-35: 35 mm; printed 35. key length, group balikci-standard-40: 40 mm; printed 40. key ' +
      'length, group balikci-standard-45: 45 mm; printed 45. key length, group balikci-standard-50: 50 mm; ' +
      'printed 50. key length, group balikci-standard-55: 55 mm; printed 55. Mid-length screws individually as ' +
      'printed; groups remain separate. 120 patients (72 women, 48 men), mean age 19.2 and range 13–54; AIS 86, ' +
      'congenital 17, adult 10 and neuromuscular 7. Counts below are screws, not patients. Standard length ' +
      'screws individually as printed; groups remain separate. 120 patients (72 women, 48 men), mean age 19.2 ' +
      'and range 13–54; AIS 86, congenital 17, adult 10 and neuromuscular 7. Counts below are screws, not ' +
      'patients.',
    population:
      'balikci-2019: Mixed scoliosis series: AIS 86, congenital 17, adult 10, neuromuscular 7; ages 13–54, mean ' +
      '19.2. Diameter chosen using insertional torque. Sample: 120 patients, 86 with AIS; 2,846 screws. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Balikci T, Kıyak G, Heydar AM, Bawaneh MK, Bezer M. 2019. Mid-length Pedicle Screws in Posterior ' +
          'Instrumentation of Scoliosis. Asian Spine J 13(5):815-822.',
        pmid: '31079434',
        doi: '10.31616/asj.2018.0177',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-screw-grids-balikci-2019-diameter',
    feature: 'Published screw diameter and length choices — diameter',
    publishedNumber:
      'key diameter, group balikci: 5 to 8 mm; printed 5–8 mm. Available diameter bounds: "Screws with a ' +
      'diameter of 5–8 mm (with 1-mm increments) were available". The bounds are retained without expanding a ' +
      'catalogue. 120 patients (72 women, 48 men), mean age 19.2 and range 13–54; AIS 86, congenital 17, adult ' +
      '10 and neuromuscular 7. Counts below are screws, not patients.',
    population:
      'balikci-2019: Mixed scoliosis series: AIS 86, congenital 17, adult 10, neuromuscular 7; ages 13–54, mean ' +
      '19.2. Diameter chosen using insertional torque. Sample: 120 patients, 86 with AIS; 2,846 screws. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Balikci T, Kıyak G, Heydar AM, Bawaneh MK, Bezer M. 2019. Mid-length Pedicle Screws in Posterior ' +
          'Instrumentation of Scoliosis. Asian Spine J 13(5):815-822.',
        pmid: '31079434',
        doi: '10.31616/asj.2018.0177',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-screw-grids-balikci-2019-diameter-step',
    feature: 'Published screw diameter and length choices — diameter step',
    publishedNumber:
      'key diameter-step, group balikci: 1 mm; printed 1-mm increments. Available diameter increment: "Screws ' +
      'with a diameter of 5–8 mm (with 1-mm increments) were available". The increment is separate from the ' +
      'bounds. 120 patients (72 women, 48 men), mean age 19.2 and range 13–54; AIS 86, congenital 17, adult 10 ' +
      'and neuromuscular 7. Counts below are screws, not patients.',
    population:
      'balikci-2019: Mixed scoliosis series: AIS 86, congenital 17, adult 10, neuromuscular 7; ages 13–54, mean ' +
      '19.2. Diameter chosen using insertional torque. Sample: 120 patients, 86 with AIS; 2,846 screws. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Balikci T, Kıyak G, Heydar AM, Bawaneh MK, Bezer M. 2019. Mid-length Pedicle Screws in Posterior ' +
          'Instrumentation of Scoliosis. Asian Spine J 13(5):815-822.',
        pmid: '31079434',
        doi: '10.31616/asj.2018.0177',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-screw-use-balikci-2019-balikci-table-2-mid-length-thoracal',
    feature: 'Published screw counts by region and curve side — balikci table 2 mid length thoracal',
    publishedNumber:
      'key count, group balikci-table-2-mid-length-thoracal: 1007 count; printed 1,007 (46.5). key percent, ' +
      'group balikci-table-2-mid-length-thoracal: 46.5 percent; printed 1,007 (46.5). Printed Mid-length screws ' +
      'distribution, thoracal region. 120 patients (72 women, 48 men), mean age 19.2 and range 13–54; AIS 86, ' +
      'congenital 17, adult 10 and neuromuscular 7. Counts below are screws, not patients.',
    population:
      'balikci-2019: Mixed scoliosis series: AIS 86, congenital 17, adult 10, neuromuscular 7; ages 13–54, mean ' +
      '19.2. Diameter chosen using insertional torque. Sample: 120 patients, 86 with AIS; 2,846 screws. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Balikci T, Kıyak G, Heydar AM, Bawaneh MK, Bezer M. 2019. Mid-length Pedicle Screws in Posterior ' +
          'Instrumentation of Scoliosis. Asian Spine J 13(5):815-822.',
        pmid: '31079434',
        doi: '10.31616/asj.2018.0177',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-screw-use-balikci-2019-balikci-table-2-mid-length-lumbar',
    feature: 'Published screw counts by region and curve side — balikci table 2 mid length lumbar',
    publishedNumber:
      'key count, group balikci-table-2-mid-length-lumbar: 264 count; printed 264 (38.7). key percent, group ' +
      'balikci-table-2-mid-length-lumbar: 38.7 percent; printed 264 (38.7). Printed Mid-length screws ' +
      'distribution, lumbar region. 120 patients (72 women, 48 men), mean age 19.2 and range 13–54; AIS 86, ' +
      'congenital 17, adult 10 and neuromuscular 7. Counts below are screws, not patients.',
    population:
      'balikci-2019: Mixed scoliosis series: AIS 86, congenital 17, adult 10, neuromuscular 7; ages 13–54, mean ' +
      '19.2. Diameter chosen using insertional torque. Sample: 120 patients, 86 with AIS; 2,846 screws. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Balikci T, Kıyak G, Heydar AM, Bawaneh MK, Bezer M. 2019. Mid-length Pedicle Screws in Posterior ' +
          'Instrumentation of Scoliosis. Asian Spine J 13(5):815-822.',
        pmid: '31079434',
        doi: '10.31616/asj.2018.0177',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-screw-use-balikci-2019-balikci-table-2-mid-length',
    feature: 'Published screw counts by region and curve side — balikci table 2 mid length',
    publishedNumber:
      'key confidence-interval, group balikci-table-2-mid-length: 0.765 to 0.813 ratio; printed 0.765–0.813. key ' +
      'p-value-max, group balikci-table-2-mid-length: 0.05 ratio; printed <0.05. 95 percent confidence interval ' +
      'as printed for the whole row; no region, side or effect parameter is inferred. 120 patients (72 women, 48 ' +
      'men), mean age 19.2 and range 13–54; AIS 86, congenital 17, adult 10 and neuromuscular 7. Counts below ' +
      'are screws, not patients. Row p-value is strictly less than 0.05 as printed. 120 patients (72 women, 48 ' +
      'men), mean age 19.2 and range 13–54; AIS 86, congenital 17, adult 10 and neuromuscular 7. Counts below ' +
      'are screws, not patients.',
    population:
      'balikci-2019: Mixed scoliosis series: AIS 86, congenital 17, adult 10, neuromuscular 7; ages 13–54, mean ' +
      '19.2. Diameter chosen using insertional torque. Sample: 120 patients, 86 with AIS; 2,846 screws. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Balikci T, Kıyak G, Heydar AM, Bawaneh MK, Bezer M. 2019. Mid-length Pedicle Screws in Posterior ' +
          'Instrumentation of Scoliosis. Asian Spine J 13(5):815-822.',
        pmid: '31079434',
        doi: '10.31616/asj.2018.0177',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-screw-use-balikci-2019-balikci-table-2-standard-length-thoracal',
    feature: 'Published screw counts by region and curve side — balikci table 2 standard length thoracal',
    publishedNumber:
      'key count, group balikci-table-2-standard-length-thoracal: 1157 count; printed 1,157 (53.5). key percent, ' +
      'group balikci-table-2-standard-length-thoracal: 53.5 percent; printed 1,157 (53.5). Printed Standard ' +
      'length screws distribution, thoracal region. 120 patients (72 women, 48 men), mean age 19.2 and range ' +
      '13–54; AIS 86, congenital 17, adult 10 and neuromuscular 7. Counts below are screws, not patients.',
    population:
      'balikci-2019: Mixed scoliosis series: AIS 86, congenital 17, adult 10, neuromuscular 7; ages 13–54, mean ' +
      '19.2. Diameter chosen using insertional torque. Sample: 120 patients, 86 with AIS; 2,846 screws. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Balikci T, Kıyak G, Heydar AM, Bawaneh MK, Bezer M. 2019. Mid-length Pedicle Screws in Posterior ' +
          'Instrumentation of Scoliosis. Asian Spine J 13(5):815-822.',
        pmid: '31079434',
        doi: '10.31616/asj.2018.0177',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-screw-use-balikci-2019-balikci-table-2-standard-length-lumbar',
    feature: 'Published screw counts by region and curve side — balikci table 2 standard length lumbar',
    publishedNumber:
      'key count, group balikci-table-2-standard-length-lumbar: 418 count; printed 418 (61.3). key percent, ' +
      'group balikci-table-2-standard-length-lumbar: 61.3 percent; printed 418 (61.3). Printed Standard length ' +
      'screws distribution, lumbar region. 120 patients (72 women, 48 men), mean age 19.2 and range 13–54; AIS ' +
      '86, congenital 17, adult 10 and neuromuscular 7. Counts below are screws, not patients.',
    population:
      'balikci-2019: Mixed scoliosis series: AIS 86, congenital 17, adult 10, neuromuscular 7; ages 13–54, mean ' +
      '19.2. Diameter chosen using insertional torque. Sample: 120 patients, 86 with AIS; 2,846 screws. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Balikci T, Kıyak G, Heydar AM, Bawaneh MK, Bezer M. 2019. Mid-length Pedicle Screws in Posterior ' +
          'Instrumentation of Scoliosis. Asian Spine J 13(5):815-822.',
        pmid: '31079434',
        doi: '10.31616/asj.2018.0177',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-screw-use-balikci-2019-balikci-table-2-standard-length',
    feature: 'Published screw counts by region and curve side — balikci table 2 standard length',
    publishedNumber:
      'key confidence-interval, group balikci-table-2-standard-length: 0.712 to 0.756 ratio; printed ' +
      '0.712–0.756. key p-value-max, group balikci-table-2-standard-length: 0.05 ratio; printed <0.05. 95 ' +
      'percent confidence interval as printed for the whole row; no region, side or effect parameter is ' +
      'inferred. 120 patients (72 women, 48 men), mean age 19.2 and range 13–54; AIS 86, congenital 17, adult 10 ' +
      'and neuromuscular 7. Counts below are screws, not patients. Row p-value is strictly less than 0.05 as ' +
      'printed. 120 patients (72 women, 48 men), mean age 19.2 and range 13–54; AIS 86, congenital 17, adult 10 ' +
      'and neuromuscular 7. Counts below are screws, not patients.',
    population:
      'balikci-2019: Mixed scoliosis series: AIS 86, congenital 17, adult 10, neuromuscular 7; ages 13–54, mean ' +
      '19.2. Diameter chosen using insertional torque. Sample: 120 patients, 86 with AIS; 2,846 screws. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Balikci T, Kıyak G, Heydar AM, Bawaneh MK, Bezer M. 2019. Mid-length Pedicle Screws in Posterior ' +
          'Instrumentation of Scoliosis. Asian Spine J 13(5):815-822.',
        pmid: '31079434',
        doi: '10.31616/asj.2018.0177',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-screw-use-balikci-2019-balikci-table-2-total-thoracal',
    feature: 'Published screw counts by region and curve side — balikci table 2 total thoracal',
    publishedNumber:
      'key count, group balikci-table-2-total-thoracal: 2164 count; printed 2,164 (100.0). key percent, group ' +
      'balikci-table-2-total-thoracal: 100.0 percent; printed 2,164 (100.0). Printed Total distribution, ' +
      'thoracal region. 120 patients (72 women, 48 men), mean age 19.2 and range 13–54; AIS 86, congenital 17, ' +
      'adult 10 and neuromuscular 7. Counts below are screws, not patients.',
    population:
      'balikci-2019: Mixed scoliosis series: AIS 86, congenital 17, adult 10, neuromuscular 7; ages 13–54, mean ' +
      '19.2. Diameter chosen using insertional torque. Sample: 120 patients, 86 with AIS; 2,846 screws. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Balikci T, Kıyak G, Heydar AM, Bawaneh MK, Bezer M. 2019. Mid-length Pedicle Screws in Posterior ' +
          'Instrumentation of Scoliosis. Asian Spine J 13(5):815-822.',
        pmid: '31079434',
        doi: '10.31616/asj.2018.0177',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-screw-use-balikci-2019-balikci-table-2-total-lumbar',
    feature: 'Published screw counts by region and curve side — balikci table 2 total lumbar',
    publishedNumber:
      'key count, group balikci-table-2-total-lumbar: 682 count; printed 682 (100.0). key percent, group ' +
      'balikci-table-2-total-lumbar: 100.0 percent; printed 682 (100.0). Printed Total distribution, lumbar ' +
      'region. 120 patients (72 women, 48 men), mean age 19.2 and range 13–54; AIS 86, congenital 17, adult 10 ' +
      'and neuromuscular 7. Counts below are screws, not patients.',
    population:
      'balikci-2019: Mixed scoliosis series: AIS 86, congenital 17, adult 10, neuromuscular 7; ages 13–54, mean ' +
      '19.2. Diameter chosen using insertional torque. Sample: 120 patients, 86 with AIS; 2,846 screws. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Balikci T, Kıyak G, Heydar AM, Bawaneh MK, Bezer M. 2019. Mid-length Pedicle Screws in Posterior ' +
          'Instrumentation of Scoliosis. Asian Spine J 13(5):815-822.',
        pmid: '31079434',
        doi: '10.31616/asj.2018.0177',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-screw-use-balikci-2019-balikci-table-2-total',
    feature: 'Published screw counts by region and curve side — balikci table 2 total',
    publishedNumber:
      'key confidence-interval, group balikci-table-2-total: 0.745 to 0.801 ratio; printed 0.745–0.801. key ' +
      'p-value-max, group balikci-table-2-total: 0.05 ratio; printed <0.05. 95 percent confidence interval as ' +
      'printed for the whole row; no region, side or effect parameter is inferred. 120 patients (72 women, 48 ' +
      'men), mean age 19.2 and range 13–54; AIS 86, congenital 17, adult 10 and neuromuscular 7. Counts below ' +
      'are screws, not patients. Row p-value is strictly less than 0.05 as printed. 120 patients (72 women, 48 ' +
      'men), mean age 19.2 and range 13–54; AIS 86, congenital 17, adult 10 and neuromuscular 7. Counts below ' +
      'are screws, not patients.',
    population:
      'balikci-2019: Mixed scoliosis series: AIS 86, congenital 17, adult 10, neuromuscular 7; ages 13–54, mean ' +
      '19.2. Diameter chosen using insertional torque. Sample: 120 patients, 86 with AIS; 2,846 screws. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Balikci T, Kıyak G, Heydar AM, Bawaneh MK, Bezer M. 2019. Mid-length Pedicle Screws in Posterior ' +
          'Instrumentation of Scoliosis. Asian Spine J 13(5):815-822.',
        pmid: '31079434',
        doi: '10.31616/asj.2018.0177',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-screw-use-balikci-2019-balikci-table-4-mid-length',
    feature: 'Published screw counts by region and curve side — balikci table 4 mid length',
    publishedNumber:
      'key count, group balikci-table-4-mid-length, curveSide convex: 651 count; printed 651 (45.7). key ' +
      'percent, group balikci-table-4-mid-length, curveSide convex: 45.7 percent; printed 651 (45.7). key count, ' +
      'group balikci-table-4-mid-length, curveSide concave: 620 count; printed 620 (43.5). key percent, group ' +
      'balikci-table-4-mid-length, curveSide concave: 43.5 percent; printed 620 (43.5). key confidence-interval, ' +
      'group balikci-table-4-mid-length: 0.474 to 0.681 ratio; printed 0.474–0.681. key p-value-max, group ' +
      'balikci-table-4-mid-length: 0.05 ratio; printed <0.05. Printed Mid-length screws distribution, convex ' +
      'side. 120 patients (72 women, 48 men), mean age 19.2 and range 13–54; AIS 86, congenital 17, adult 10 and ' +
      'neuromuscular 7. Counts below are screws, not patients. Printed Mid-length screws distribution, concave ' +
      'side. 120 patients (72 women, 48 men), mean age 19.2 and range 13–54; AIS 86, congenital 17, adult 10 and ' +
      'neuromuscular 7. Counts below are screws, not patients. §C.15 notes the printed percentages are slightly ' +
      'off: mid-length 43.5 percent versus 620/1423, and standard 56.5 percent versus 803/1423. Printed 43.5 and ' +
      '56.5 are retained; no percentage is recomputed. 95 percent confidence interval as printed for the whole ' +
      'row; no region, side or effect parameter is inferred. 120 patients (72 women, 48 men), mean age 19.2 and ' +
      'range 13–54; AIS 86, congenital 17, adult 10 and neuromuscular 7. Counts below are screws, not patients. ' +
      'Row p-value is strictly less than 0.05 as printed. 120 patients (72 women, 48 men), mean age 19.2 and ' +
      'range 13–54; AIS 86, congenital 17, adult 10 and neuromuscular 7. Counts below are screws, not patients.',
    population:
      'balikci-2019: Mixed scoliosis series: AIS 86, congenital 17, adult 10, neuromuscular 7; ages 13–54, mean ' +
      '19.2. Diameter chosen using insertional torque. Sample: 120 patients, 86 with AIS; 2,846 screws. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Balikci T, Kıyak G, Heydar AM, Bawaneh MK, Bezer M. 2019. Mid-length Pedicle Screws in Posterior ' +
          'Instrumentation of Scoliosis. Asian Spine J 13(5):815-822.',
        pmid: '31079434',
        doi: '10.31616/asj.2018.0177',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-screw-use-balikci-2019-balikci-table-4-standard-length',
    feature: 'Published screw counts by region and curve side — balikci table 4 standard length',
    publishedNumber:
      'key count, group balikci-table-4-standard-length, curveSide convex: 772 count; printed 772 (54.3). key ' +
      'percent, group balikci-table-4-standard-length, curveSide convex: 54.3 percent; printed 772 (54.3). key ' +
      'count, group balikci-table-4-standard-length, curveSide concave: 803 count; printed 803 (56.5). key ' +
      'percent, group balikci-table-4-standard-length, curveSide concave: 56.5 percent; printed 803 (56.5). key ' +
      'confidence-interval, group balikci-table-4-standard-length: 0.393 to 0.534 ratio; printed 0.393–0.534. ' +
      'key p-value-max, group balikci-table-4-standard-length: 0.05 ratio; printed <0.05. Printed Standart ' +
      'length screws distribution, convex side. 120 patients (72 women, 48 men), mean age 19.2 and range 13–54; ' +
      'AIS 86, congenital 17, adult 10 and neuromuscular 7. Counts below are screws, not patients. Printed ' +
      'Standart length screws distribution, concave side. 120 patients (72 women, 48 men), mean age 19.2 and ' +
      'range 13–54; AIS 86, congenital 17, adult 10 and neuromuscular 7. Counts below are screws, not patients. ' +
      '§C.15 notes the printed percentages are slightly off: mid-length 43.5 percent versus 620/1423, and ' +
      'standard 56.5 percent versus 803/1423. Printed 43.5 and 56.5 are retained; no percentage is recomputed. ' +
      '95 percent confidence interval as printed for the whole row; no region, side or effect parameter is ' +
      'inferred. 120 patients (72 women, 48 men), mean age 19.2 and range 13–54; AIS 86, congenital 17, adult 10 ' +
      'and neuromuscular 7. Counts below are screws, not patients. Row p-value is strictly less than 0.05 as ' +
      'printed. 120 patients (72 women, 48 men), mean age 19.2 and range 13–54; AIS 86, congenital 17, adult 10 ' +
      'and neuromuscular 7. Counts below are screws, not patients.',
    population:
      'balikci-2019: Mixed scoliosis series: AIS 86, congenital 17, adult 10, neuromuscular 7; ages 13–54, mean ' +
      '19.2. Diameter chosen using insertional torque. Sample: 120 patients, 86 with AIS; 2,846 screws. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Balikci T, Kıyak G, Heydar AM, Bawaneh MK, Bezer M. 2019. Mid-length Pedicle Screws in Posterior ' +
          'Instrumentation of Scoliosis. Asian Spine J 13(5):815-822.',
        pmid: '31079434',
        doi: '10.31616/asj.2018.0177',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-screw-use-balikci-2019-balikci-table-4-total',
    feature: 'Published screw counts by region and curve side — balikci table 4 total',
    publishedNumber:
      'key count, group balikci-table-4-total, curveSide convex: 1423 count; printed 1,423 (100.0). key percent, ' +
      'group balikci-table-4-total, curveSide convex: 100.0 percent; printed 1,423 (100.0). key count, group ' +
      'balikci-table-4-total, curveSide concave: 1423 count; printed 1,423 (100.0). key percent, group ' +
      'balikci-table-4-total, curveSide concave: 100.0 percent; printed 1,423 (100.0). key confidence-interval, ' +
      'group balikci-table-4-total: 0.456 to 0.693 ratio; printed 0.456–0.693. key p-value-max, group ' +
      'balikci-table-4-total: 0.05 ratio; printed <0.05. Printed Total distribution, convex side. 120 patients ' +
      '(72 women, 48 men), mean age 19.2 and range 13–54; AIS 86, congenital 17, adult 10 and neuromuscular 7. ' +
      'Counts below are screws, not patients. Printed Total distribution, concave side. 120 patients (72 women, ' +
      '48 men), mean age 19.2 and range 13–54; AIS 86, congenital 17, adult 10 and neuromuscular 7. Counts below ' +
      'are screws, not patients. 95 percent confidence interval as printed for the whole row; no region, side or ' +
      'effect parameter is inferred. 120 patients (72 women, 48 men), mean age 19.2 and range 13–54; AIS 86, ' +
      'congenital 17, adult 10 and neuromuscular 7. Counts below are screws, not patients. Row p-value is ' +
      'strictly less than 0.05 as printed. 120 patients (72 women, 48 men), mean age 19.2 and range 13–54; AIS ' +
      '86, congenital 17, adult 10 and neuromuscular 7. Counts below are screws, not patients.',
    population:
      'balikci-2019: Mixed scoliosis series: AIS 86, congenital 17, adult 10, neuromuscular 7; ages 13–54, mean ' +
      '19.2. Diameter chosen using insertional torque. Sample: 120 patients, 86 with AIS; 2,846 screws. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Balikci T, Kıyak G, Heydar AM, Bawaneh MK, Bezer M. 2019. Mid-length Pedicle Screws in Posterior ' +
          'Instrumentation of Scoliosis. Asian Spine J 13(5):815-822.',
        pmid: '31079434',
        doi: '10.31616/asj.2018.0177',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-screw-use-balikci-2019-balikci-overall-total',
    feature: 'Published screw counts by region and curve side — balikci overall total',
    publishedNumber:
      'key count, group balikci-overall-total: 2846 count; printed 2846. Count as printed in the quoted text; no ' +
      'subtotal is calculated. 120 patients (72 women, 48 men), mean age 19.2 and range 13–54; AIS 86, ' +
      'congenital 17, adult 10 and neuromuscular 7. Counts below are screws, not patients.',
    population:
      'balikci-2019: Mixed scoliosis series: AIS 86, congenital 17, adult 10, neuromuscular 7; ages 13–54, mean ' +
      '19.2. Diameter chosen using insertional torque. Sample: 120 patients, 86 with AIS; 2,846 screws. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Balikci T, Kıyak G, Heydar AM, Bawaneh MK, Bezer M. 2019. Mid-length Pedicle Screws in Posterior ' +
          'Instrumentation of Scoliosis. Asian Spine J 13(5):815-822.',
        pmid: '31079434',
        doi: '10.31616/asj.2018.0177',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-screw-use-balikci-2019-balikci-overall-standard-length',
    feature: 'Published screw counts by region and curve side — balikci overall standard length',
    publishedNumber:
      'key count, group balikci-overall-standard-length: 1575 count; printed 1575. key percent, group ' +
      'balikci-overall-standard-length: 55.4 percent; printed 55.4. Count as printed in the quoted text; no ' +
      'subtotal is calculated. 120 patients (72 women, 48 men), mean age 19.2 and range 13–54; AIS 86, ' +
      'congenital 17, adult 10 and neuromuscular 7. Counts below are screws, not patients. Percentage as printed ' +
      'in the quoted text; no percentage is calculated. 120 patients (72 women, 48 men), mean age 19.2 and range ' +
      '13–54; AIS 86, congenital 17, adult 10 and neuromuscular 7. Counts below are screws, not patients.',
    population:
      'balikci-2019: Mixed scoliosis series: AIS 86, congenital 17, adult 10, neuromuscular 7; ages 13–54, mean ' +
      '19.2. Diameter chosen using insertional torque. Sample: 120 patients, 86 with AIS; 2,846 screws. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Balikci T, Kıyak G, Heydar AM, Bawaneh MK, Bezer M. 2019. Mid-length Pedicle Screws in Posterior ' +
          'Instrumentation of Scoliosis. Asian Spine J 13(5):815-822.',
        pmid: '31079434',
        doi: '10.31616/asj.2018.0177',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-screw-use-balikci-2019-balikci-overall-mid-length',
    feature: 'Published screw counts by region and curve side — balikci overall mid length',
    publishedNumber:
      'key count, group balikci-overall-mid-length: 1271 count; printed 1271. key percent, group ' +
      'balikci-overall-mid-length: 44.6 percent; printed 44.6. Count as printed in the quoted text; no subtotal ' +
      'is calculated. 120 patients (72 women, 48 men), mean age 19.2 and range 13–54; AIS 86, congenital 17, ' +
      'adult 10 and neuromuscular 7. Counts below are screws, not patients. Percentage as printed in the quoted ' +
      'text; no percentage is calculated. 120 patients (72 women, 48 men), mean age 19.2 and range 13–54; AIS ' +
      '86, congenital 17, adult 10 and neuromuscular 7. Counts below are screws, not patients.',
    population:
      'balikci-2019: Mixed scoliosis series: AIS 86, congenital 17, adult 10, neuromuscular 7; ages 13–54, mean ' +
      '19.2. Diameter chosen using insertional torque. Sample: 120 patients, 86 with AIS; 2,846 screws. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Balikci T, Kıyak G, Heydar AM, Bawaneh MK, Bezer M. 2019. Mid-length Pedicle Screws in Posterior ' +
          'Instrumentation of Scoliosis. Asian Spine J 13(5):815-822.',
        pmid: '31079434',
        doi: '10.31616/asj.2018.0177',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-screw-use-balikci-2019-balikci-apical-total',
    feature: 'Published screw counts by region and curve side — balikci apical total',
    publishedNumber:
      'key count, group balikci-apical-total: 288 count; printed 288. Count as printed in the quoted text; no ' +
      'subtotal is calculated. 120 patients (72 women, 48 men), mean age 19.2 and range 13–54; AIS 86, ' +
      'congenital 17, adult 10 and neuromuscular 7. Counts below are screws, not patients.',
    population:
      'balikci-2019: Mixed scoliosis series: AIS 86, congenital 17, adult 10, neuromuscular 7; ages 13–54, mean ' +
      '19.2. Diameter chosen using insertional torque. Sample: 120 patients, 86 with AIS; 2,846 screws. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Balikci T, Kıyak G, Heydar AM, Bawaneh MK, Bezer M. 2019. Mid-length Pedicle Screws in Posterior ' +
          'Instrumentation of Scoliosis. Asian Spine J 13(5):815-822.',
        pmid: '31079434',
        doi: '10.31616/asj.2018.0177',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-screw-use-balikci-2019-balikci-apical-mid-length',
    feature: 'Published screw counts by region and curve side — balikci apical mid length',
    publishedNumber:
      'key count, group balikci-apical-mid-length: 177 count; printed 177. key percent, group ' +
      'balikci-apical-mid-length: 61.5 percent; printed 61.5. Count as printed in the quoted text; no subtotal ' +
      'is calculated. 120 patients (72 women, 48 men), mean age 19.2 and range 13–54; AIS 86, congenital 17, ' +
      'adult 10 and neuromuscular 7. Counts below are screws, not patients. Percentage as printed in the quoted ' +
      'text; no percentage is calculated. 120 patients (72 women, 48 men), mean age 19.2 and range 13–54; AIS ' +
      '86, congenital 17, adult 10 and neuromuscular 7. Counts below are screws, not patients.',
    population:
      'balikci-2019: Mixed scoliosis series: AIS 86, congenital 17, adult 10, neuromuscular 7; ages 13–54, mean ' +
      '19.2. Diameter chosen using insertional torque. Sample: 120 patients, 86 with AIS; 2,846 screws. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Balikci T, Kıyak G, Heydar AM, Bawaneh MK, Bezer M. 2019. Mid-length Pedicle Screws in Posterior ' +
          'Instrumentation of Scoliosis. Asian Spine J 13(5):815-822.',
        pmid: '31079434',
        doi: '10.31616/asj.2018.0177',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-screw-use-balikci-2019-balikci-apical-standard-length',
    feature: 'Published screw counts by region and curve side — balikci apical standard length',
    publishedNumber:
      'key count, group balikci-apical-standard-length: 111 count; printed 111. key percent, group ' +
      'balikci-apical-standard-length: 38.5 percent; printed 38.5. Count as printed in the quoted text; no ' +
      'subtotal is calculated. 120 patients (72 women, 48 men), mean age 19.2 and range 13–54; AIS 86, ' +
      'congenital 17, adult 10 and neuromuscular 7. Counts below are screws, not patients. Percentage as printed ' +
      'in the quoted text; no percentage is calculated. 120 patients (72 women, 48 men), mean age 19.2 and range ' +
      '13–54; AIS 86, congenital 17, adult 10 and neuromuscular 7. Counts below are screws, not patients.',
    population:
      'balikci-2019: Mixed scoliosis series: AIS 86, congenital 17, adult 10, neuromuscular 7; ages 13–54, mean ' +
      '19.2. Diameter chosen using insertional torque. Sample: 120 patients, 86 with AIS; 2,846 screws. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Balikci T, Kıyak G, Heydar AM, Bawaneh MK, Bezer M. 2019. Mid-length Pedicle Screws in Posterior ' +
          'Instrumentation of Scoliosis. Asian Spine J 13(5):815-822.',
        pmid: '31079434',
        doi: '10.31616/asj.2018.0177',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-screw-use-balikci-2019-balikci-apical-mid-vs-standard',
    feature: 'Published screw counts by region and curve side — balikci apical mid vs standard',
    publishedNumber:
      'key p-value-max, group balikci-apical-mid-vs-standard: 0.05 ratio; printed p<0.05. Apical mid-length ' +
      'versus standard comparison: p strictly less than 0.05 as printed.',
    population:
      'balikci-2019: Mixed scoliosis series: AIS 86, congenital 17, adult 10, neuromuscular 7; ages 13–54, mean ' +
      '19.2. Diameter chosen using insertional torque. Sample: 120 patients, 86 with AIS; 2,846 screws. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Balikci T, Kıyak G, Heydar AM, Bawaneh MK, Bezer M. 2019. Mid-length Pedicle Screws in Posterior ' +
          'Instrumentation of Scoliosis. Asian Spine J 13(5):815-822.',
        pmid: '31079434',
        doi: '10.31616/asj.2018.0177',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-stop-short-odds-liu-2020-sv-minus-one',
    feature: 'Associations when stopping above or below a named lower end — sv minus one',
    publishedNumber:
      'key studies, group sv-minus-one: 4 count. key patients, group sv-minus-one: 369 count. key proximal-odds, ' +
      'group sv-minus-one: 2.6 ratio. key distal-odds, group sv-minus-one: 0.44 ratio. key reference-adding-on, ' +
      'group sv-minus-one: 13.4 percent. key reference-adding-on-count, group sv-minus-one: 18 count. key ' +
      'reference-total, group sv-minus-one: 134 count. key proximal-confidence-interval, group sv-minus-one: ' +
      '1.44 to 4.69 ratio. key distal-confidence-interval, group sv-minus-one: 0.17 to 1.13 ratio. key ' +
      'distal-studies, group sv-minus-one: 4 count. key distal-patients, group sv-minus-one: 233 count. Proximal ' +
      'comparison: 4 studies/369 patients; distal comparison: 4 studies/233 patients. The existing ' +
      'studies/patients keys describe the proximal comparison only; reference-count cells have their own ' +
      'denominators. Pooled Lenke 1A/2A thoracic-only fusions; proximal-to-reference comparison, not a ' +
      'patient-specific probability. Proximal comparison: 4 studies/369 patients; distal comparison: 4 ' +
      'studies/233 patients. The existing studies/patients keys describe the proximal comparison only; ' +
      'reference-count cells have their own denominators. Distal comparison count only; do not inherit the ' +
      'proximal-comparison study/patient counts or reference-group denominators.',
    population:
      'liu-2020: Lenke 1A/2A thoracic-only fusions. Sample: 6 studies; 732 patients; 150 with adding-on; ' +
      'per-reference denominators retained in the table. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Liu CW, Lenke LG, Tan LA, Oh T, Chao KH, Lin SD, Pan RY. 2020. Selection of the Lowest Instrumented ' +
          'Vertebra and Relative Odds Ratio of Distal Adding-on for Lenke Type 1A and 2A Curves in Adolescent ' +
          'Idiopathic Scoliosis: A Systematic Review and Meta-analysis. Neurospine 17(4):902-909.',
        pmid: '33401869',
        doi: '10.14245/ns.2040234.117',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-stop-short-odds-liu-2020-ev-plus-one',
    feature: 'Associations when stopping above or below a named lower end — ev plus one',
    publishedNumber:
      'key studies, group ev-plus-one: 4 count. key patients, group ev-plus-one: 371 count. key proximal-odds, ' +
      'group ev-plus-one: 2.43 ratio. key distal-odds, group ev-plus-one: 0.56 ratio. key reference-adding-on, ' +
      'group ev-plus-one: 13.0 percent. key reference-adding-on-count, group ev-plus-one: 20 count. key ' +
      'reference-total, group ev-plus-one: 153 count. key proximal-confidence-interval, group ev-plus-one: 1.36 ' +
      'to 4.34 ratio. key distal-confidence-interval, group ev-plus-one: 0.23 to 1.34 ratio. key distal-studies, ' +
      'group ev-plus-one: 4 count. key distal-patients, group ev-plus-one: 250 count. Proximal comparison: 4 ' +
      'studies/371 patients; distal comparison: 4 studies/250 patients. The existing studies/patients keys ' +
      'describe the proximal comparison only; reference-count cells have their own denominators. Pooled Lenke ' +
      '1A/2A thoracic-only fusions; proximal-to-reference comparison, not a patient-specific probability. ' +
      'Proximal comparison: 4 studies/371 patients; distal comparison: 4 studies/250 patients. The existing ' +
      'studies/patients keys describe the proximal comparison only; reference-count cells have their own ' +
      'denominators. Distal comparison count only; do not inherit the proximal-comparison study/patient counts ' +
      'or reference-group denominators.',
    population:
      'liu-2020: Lenke 1A/2A thoracic-only fusions. Sample: 6 studies; 732 patients; 150 with adding-on; ' +
      'per-reference denominators retained in the table. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Liu CW, Lenke LG, Tan LA, Oh T, Chao KH, Lin SD, Pan RY. 2020. Selection of the Lowest Instrumented ' +
          'Vertebra and Relative Odds Ratio of Distal Adding-on for Lenke Type 1A and 2A Curves in Adolescent ' +
          'Idiopathic Scoliosis: A Systematic Review and Meta-analysis. Neurospine 17(4):902-909.',
        pmid: '33401869',
        doi: '10.14245/ns.2040234.117',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-stop-short-odds-liu-2020-nv',
    feature: 'Associations when stopping above or below a named lower end — nv',
    publishedNumber:
      'key studies, group nv: 5 count. key patients, group nv: 472 count. key proximal-odds, group nv: 3.05 ' +
      'ratio. key distal-odds, group nv: 0.69 ratio. key reference-adding-on, group nv: 13.1 percent. key ' +
      'reference-adding-on-count, group nv: 18 count. key reference-total, group nv: 137 count. key ' +
      'proximal-confidence-interval, group nv: 1.68 to 5.54 ratio. key distal-confidence-interval, group nv: ' +
      '0.29 to 1.65 ratio. key distal-studies, group nv: 5 count. key distal-patients, group nv: 286 count. ' +
      'Proximal comparison: 5 studies/472 patients; distal comparison: 5 studies/286 patients. The existing ' +
      'studies/patients keys describe the proximal comparison only; reference-count cells have their own ' +
      'denominators. Pooled Lenke 1A/2A thoracic-only fusions; proximal-to-reference comparison, not a ' +
      'patient-specific probability. Proximal comparison: 5 studies/472 patients; distal comparison: 5 ' +
      'studies/286 patients. The existing studies/patients keys describe the proximal comparison only; ' +
      'reference-count cells have their own denominators. Distal comparison count only; do not inherit the ' +
      'proximal-comparison study/patient counts or reference-group denominators.',
    population:
      'liu-2020: Lenke 1A/2A thoracic-only fusions. Sample: 6 studies; 732 patients; 150 with adding-on; ' +
      'per-reference denominators retained in the table. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Liu CW, Lenke LG, Tan LA, Oh T, Chao KH, Lin SD, Pan RY. 2020. Selection of the Lowest Instrumented ' +
          'Vertebra and Relative Odds Ratio of Distal Adding-on for Lenke Type 1A and 2A Curves in Adolescent ' +
          'Idiopathic Scoliosis: A Systematic Review and Meta-analysis. Neurospine 17(4):902-909.',
        pmid: '33401869',
        doi: '10.14245/ns.2040234.117',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-stop-short-odds-liu-2020-ltv',
    feature: 'Associations when stopping above or below a named lower end — ltv',
    publishedNumber:
      'key studies, group ltv: 4 count. key patients, group ltv: 398 count. key proximal-odds, group ltv: 3.4 ' +
      'ratio. key distal-odds, group ltv: 0.66 ratio. key reference-adding-on, group ltv: 14.6 percent. key ' +
      'reference-adding-on-count, group ltv: 38 count. key reference-total, group ltv: 259 count. key ' +
      'proximal-confidence-interval, group ltv: 1.32 to 8.73 ratio. key distal-confidence-interval, group ltv: ' +
      '0.36 to 1.21 ratio. key i-squared, group ltv: 66 percent. key distal-studies, group ltv: 4 count. key ' +
      'distal-patients, group ltv: 441 count. Proximal comparison: 4 studies/398 patients; distal comparison: 4 ' +
      'studies/441 patients. The existing studies/patients keys describe the proximal comparison only; ' +
      'reference-count cells have their own denominators. Pooled Lenke 1A/2A thoracic-only fusions; ' +
      'proximal-to-reference comparison, not a patient-specific probability. Proximal comparison: 4 studies/398 ' +
      'patients; distal comparison: 4 studies/441 patients. The existing studies/patients keys describe the ' +
      'proximal comparison only; reference-count cells have their own denominators. Distal comparison count ' +
      'only; do not inherit the proximal-comparison study/patient counts or reference-group denominators.',
    population:
      'liu-2020: Lenke 1A/2A thoracic-only fusions. Sample: 6 studies; 732 patients; 150 with adding-on; ' +
      'per-reference denominators retained in the table. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Liu CW, Lenke LG, Tan LA, Oh T, Chao KH, Lin SD, Pan RY. 2020. Selection of the Lowest Instrumented ' +
          'Vertebra and Relative Odds Ratio of Distal Adding-on for Lenke Type 1A and 2A Curves in Adolescent ' +
          'Idiopathic Scoliosis: A Systematic Review and Meta-analysis. Neurospine 17(4):902-909.',
        pmid: '33401869',
        doi: '10.14245/ns.2040234.117',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-stop-short-odds-liu-2020-stv-or-nstv-plus-one',
    feature: 'Associations when stopping above or below a named lower end — stv or nstv plus one',
    publishedNumber:
      'key studies, group stv-or-nstv-plus-one: 3 count. key patients, group stv-or-nstv-plus-one: 426 count. ' +
      'key proximal-odds, group stv-or-nstv-plus-one: 4.52 ratio. key distal-odds, group stv-or-nstv-plus-one: ' +
      '0.5 ratio. key reference-adding-on, group stv-or-nstv-plus-one: 11.3 percent. key ' +
      'reference-adding-on-count, group stv-or-nstv-plus-one: 24 count. key reference-total, group ' +
      'stv-or-nstv-plus-one: 211 count. key proximal-confidence-interval, group stv-or-nstv-plus-one: 1.25 to ' +
      '16.28 ratio. key distal-confidence-interval, group stv-or-nstv-plus-one: 0.09 to 2.92 ratio. key ' +
      'i-squared, group stv-or-nstv-plus-one: 80 percent. key distal-studies, group stv-or-nstv-plus-one: 2 ' +
      'count. key distal-patients, group stv-or-nstv-plus-one: 158 count. Proximal comparison: 3 studies/426 ' +
      'patients; distal comparison: 2 studies/158 patients. The existing studies/patients keys describe the ' +
      'proximal comparison only; reference-count cells have their own denominators. Pooled Lenke 1A/2A ' +
      'thoracic-only fusions; proximal-to-reference comparison, not a patient-specific probability. Proximal ' +
      'comparison: 3 studies/426 patients; distal comparison: 2 studies/158 patients. The existing ' +
      'studies/patients keys describe the proximal comparison only; reference-count cells have their own ' +
      'denominators. Distal comparison count only; do not inherit the proximal-comparison study/patient counts ' +
      'or reference-group denominators.',
    population:
      'liu-2020: Lenke 1A/2A thoracic-only fusions. Sample: 6 studies; 732 patients; 150 with adding-on; ' +
      'per-reference denominators retained in the table. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Liu CW, Lenke LG, Tan LA, Oh T, Chao KH, Lin SD, Pan RY. 2020. Selection of the Lowest Instrumented ' +
          'Vertebra and Relative Odds Ratio of Distal Adding-on for Lenke Type 1A and 2A Curves in Adolescent ' +
          'Idiopathic Scoliosis: A Systematic Review and Meta-analysis. Neurospine 17(4):902-909.',
        pmid: '33401869',
        doi: '10.14245/ns.2040234.117',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-layout-source-ueno-2025',
    feature: 'Additional source context — ueno 2025',
    publishedNumber:
      'No separate per-level numerical table is supplied by this source record. Recorded sample: 465 screws; ' +
      'patient count not supplied. Robot or navigation study of grade-4 cortical channels; further cohort ' +
      'details not supplied in the note.',
    population:
      'ueno-2025: Robot or navigation study of grade-4 cortical channels; further cohort details not supplied in ' +
      'the note. Sample: 465 screws; patient count not supplied. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Ueno J, Torii Y, Iinuma M, Yoshida A, Tomochika K, Hideshima T, Ito M, Akazawa T. 2025. Factors causing ' +
          'deviation of robot-assisted pedicle screw placement for adolescent idiopathic scoliosis: why does ' +
          'deviation still occur even with robotics? Eur Spine J 34(7):2714-2722.',
        pmid: '40299013',
        doi: '10.1007/s00586-025-08882-1',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-anchor-strength-thoracic-anatomic',
    feature: 'Adult thoracic screw, anatomic',
    publishedNumber:
      'mean: 481 N [published; lehman-2003]; se: 54 N [published; lehman-2003]. Lower endpoint of bench T1–T8 ' +
      'range; T1–T8 assignment is own inference, not reported levels. Table use: bench.',
    population:
      'lehman-2003: Adult thoracic cadaver vertebrae; straight-forward and anatomic trajectories. Sample: 30 ' +
      'vertebrae, paired. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Lehman RA, Polly DW, Kuklo TR, Cunningham B, Kirk KL, Belmont PJ. 2003. Straight-forward versus anatomic ' +
          'trajectory technique of thoracic pedicle screw fixation: a biomechanical analysis. Spine (Phila Pa 1976) ' +
          '28(18):2058-2065.',
        pmid: '14501914',
        doi: '10.1097/01.brs.0000087743.57439.4f',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-anchor-strength-thoracic-straight-forward',
    feature: 'Adult thoracic screw, straight-forward',
    publishedNumber:
      'mean: 611 N [published; lehman-2003]; se: 50 N [published; lehman-2003]. Upper endpoint of bench T1–T8 ' +
      'range; paired trajectories, not paediatric strength. Table use: bench.',
    population:
      'lehman-2003: Adult thoracic cadaver vertebrae; straight-forward and anatomic trajectories. Sample: 30 ' +
      'vertebrae, paired. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Lehman RA, Polly DW, Kuklo TR, Cunningham B, Kirk KL, Belmont PJ. 2003. Straight-forward versus anatomic ' +
          'trajectory technique of thoracic pedicle screw fixation: a biomechanical analysis. Spine (Phila Pa 1976) ' +
          '28(18):2058-2065.',
        pmid: '14501914',
        doi: '10.1097/01.brs.0000087743.57439.4f',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-anchor-strength-thoracic-sampling',
    feature: 'Thoracic screw prior endpoints',
    publishedNumber:
      'lower: 481 N [published; lehman-2003]; upper: 611 N [published; lehman-2003]. Own inference: sample ' +
      'uniformly between trajectory means at T1–T8; do not combine SEs or scale by screw size. Table use: bench.',
    population:
      'lehman-2003: Adult thoracic cadaver vertebrae; straight-forward and anatomic trajectories. Sample: 30 ' +
      'vertebrae, paired. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Lehman RA, Polly DW, Kuklo TR, Cunningham B, Kirk KL, Belmont PJ. 2003. Straight-forward versus anatomic ' +
          'trajectory technique of thoracic pedicle screw fixation: a biomechanical analysis. Spine (Phila Pa 1976) ' +
          '28(18):2058-2065.',
        pmid: '14501914',
        doi: '10.1097/01.brs.0000087743.57439.4f',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-anchor-strength-t4-t8-screw',
    feature: 'Adult thoracic reference',
    publishedNumber:
      'mean: 531 N [published; liljenqvist-2001]. Published screw mean in the named adult thoracic region; no ' +
      'reference screw dimensions supplied. Table use: beside.',
    population:
      'liljenqvist-2001: Adult thoracic spines; T4–T8 andT9–T12 screws and hooks. Sample: 9 spines;90 tests. ' +
      '[peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Liljenqvist U, Hackenberg L, Link T, Halm H. 2001. Pullout strength of pedicle screws versus pedicle and ' +
          'laminar hooks in the thoracic spine. Acta Orthop Belg 67(2):157-163.',
        pmid: '11383294',
        doi: null,
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-anchor-strength-t9-t12-screw',
    feature: 'Adult thoracic reference',
    publishedNumber:
      'mean: 807 N [published; liljenqvist-2001]. Published screw mean in the named adult thoracic region; no ' +
      'reference screw dimensions supplied. Table use: bench.',
    population:
      'liljenqvist-2001: Adult thoracic spines; T4–T8 andT9–T12 screws and hooks. Sample: 9 spines;90 tests. ' +
      '[peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Liljenqvist U, Hackenberg L, Link T, Halm H. 2001. Pullout strength of pedicle screws versus pedicle and ' +
          'laminar hooks in the thoracic spine. Acta Orthop Belg 67(2):157-163.',
        pmid: '11383294',
        doi: null,
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-anchor-strength-t4-t8-pedicle-hook',
    feature: 'Adult thoracic reference',
    publishedNumber:
      'mean: 321 N [published; liljenqvist-2001]. Applying T4–T8 hook value at other levels is own borrowing, ' +
      'not a per-level hook table. Table use: bench.',
    population:
      'liljenqvist-2001: Adult thoracic spines; T4–T8 andT9–T12 screws and hooks. Sample: 9 spines;90 tests. ' +
      '[peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Liljenqvist U, Hackenberg L, Link T, Halm H. 2001. Pullout strength of pedicle screws versus pedicle and ' +
          'laminar hooks in the thoracic spine. Acta Orthop Belg 67(2):157-163.',
        pmid: '11383294',
        doi: null,
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-anchor-strength-laminar-hook',
    feature: 'T9–T12 supralaminar hook reference',
    publishedNumber:
      'mean: 600 N [published; liljenqvist-2001]. Supralaminar hooks used atT9–T12;600N. Confirmed in audited ' +
      'abstract. Table use: beside.',
    population:
      'liljenqvist-2001: Adult thoracic spines; T4–T8 andT9–T12 screws and hooks. Sample: 9 spines;90 tests. ' +
      '[peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Liljenqvist U, Hackenberg L, Link T, Halm H. 2001. Pullout strength of pedicle screws versus pedicle and ' +
          'laminar hooks in the thoracic spine. Acta Orthop Belg 67(2):157-163.',
        pmid: '11383294',
        doi: null,
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-anchor-strength-lumbar-screw',
    feature: 'Cadaveric lumbar7.0mm screw reference',
    publishedNumber:
      'normal: 1056.4 N [published; soshi-1991]; osteoporosis grade i: 495.6 N [published; soshi-1991]; ' +
      'osteoporosis grade ii: 269.5 N [published; soshi-1991]; diameter: 7.0 mm [published; soshi-1991]. ' +
      'Abstract does not state adulthood. Bench maps normal value toL1–L5 as own inference;reference length ' +
      'absent;no diameter/length scaling. Table use: bench.',
    population:
      'soshi-1991: Cadaveric lumbar vertebrae, normal and osteoporosis gradesI/II;7.0mm screws. Adulthood not ' +
      'stated in the abstract. Sample: not supplied. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Soshi S, Shiba R, Kondo H, Murota K. 1991. An experimental study on transpedicular screw fixation in ' +
          'relation to osteoporosis of the lumbar spine. Spine (Phila Pa 1976) 16(11):1335-1341.',
        pmid: '1750009',
        doi: '10.1097/00007632-199111000-00015',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-anchor-strength-bone-density-multipliers',
    feature: 'Own rounded osteoporosis/normal ratios',
    publishedNumber:
      'grade i: about 0.47 ratio [own-inference; project-assumptions]; grade ii: about 0.26 ratio ' +
      '[own-inference; project-assumptions]; normal force: 1056.4 N [published; soshi-1991]; grade i force: ' +
      '495.6 N [published; soshi-1991]; grade ii force: 269.5 N [published; soshi-1991]. Own ' +
      'arithmetic:grade-i=495.6/1056.4 rounded to0.47;grade-ii=269.5/1056.4 rounded to0.26. Ratios not printed ' +
      'in abstract. Published inputs retained here beside the derived ratios. Table use: input.',
    population:
      'project-assumptions: No measured population; explicit choices and calculations in the project notes. ' +
      'Sample: not supplied. [own-inference; secondary]. soshi-1991: Cadaveric lumbar vertebrae, normal and ' +
      'osteoporosis gradesI/II;7.0mm screws. Adulthood not stated in the abstract. Sample: not supplied. ' +
      '[peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Soshi S, Shiba R, Kondo H, Murota K. 1991. An experimental study on transpedicular screw fixation in ' +
          'relation to osteoporosis of the lumbar spine. Spine (Phila Pa 1976) 16(11):1335-1341.',
        pmid: '1750009',
        doi: '10.1097/00007632-199111000-00015',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-anchor-strength-ais-bone-prevalence',
    feature: 'Inputs behind bone-factor mixture',
    publishedNumber:
      'low abmd: 38 percent [published; cheng-2000]; low vbmd: 36 percent [published; cheng-2000]; osteopenia: ' +
      '32.9 percent [published; yip-2016]; osteopenic count: 169 people [published; yip-2016]; cohort count: 513 ' +
      'people [published; yip-2016]. Cheng below −1 SD; Yip femoral-neck Z ≤ −1; neither measures screw ' +
      'capacity. Table use: input.',
    population:
      'cheng-2000: AIS girls aged12–14;75AIS and94controls;38 percentaBMD/36 percentvBMD below−1SD. Sample: 75 ' +
      'AIS;94 controls. [peer-reviewed; abstract]. yip-2016: AIS girls, mean age 13.15, follow-up4.6 years; ' +
      '169/513 osteopenic. Sample: 513;HR-pQCT subgroup90. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Cheng JC, Qin L, Cheung CS, Sher AH, Lee KM, Ng SW, Guo X. 2000. Generalized low areal and volumetric ' +
          'bone mineral density in adolescent idiopathic scoliosis. J Bone Miner Res 15(8):1587-1595.',
        pmid: '10934658',
        doi: '10.1359/jbmr.2000.15.8.1587',
      },
      {
        citation: 'Yip BHK, Yu FWP, Wang Z, Hung VWY, Lam TP, Ng BKW, Zhu F, Cheng JCY. 2016. Sci Rep 6:39220.',
        pmid: '27991528',
        doi: '10.1038/srep39220',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-anchor-strength-immature-pig',
    feature: 'Immature pig context',
    publishedNumber:
      'lower: about 420 N [published; le-cann-2015]; upper: about 1341 N [published; le-cann-2015]; full text ' +
      'lower: 419.9 N [published; le-cann-2015]; full text upper: 1341.2 N [published; le-cann-2015]. Context ' +
      'only; not a human paediatric reference. Full-text unrounded group-mean extremes419.9/1341.2N retained ' +
      'separately from rounded old note420/1341N. Table use: beside.',
    population:
      'le-cann-2015: Immature pigs aged90 and140 days; mono-/bicortical, thoracic/lumbar. Sample: 80 vertebrae. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Le Cann S, Cachon T, Viguier E, Miladi L, Odent T, Rossi JM, Chabrand P. 2015. Pedicle Screw Fixation ' +
          'Study in Immature Porcine Spines to Improve Pullout Resistance during Animal Testing. PLoS One ' +
          '10(10):e0127463.',
        pmid: '26451947',
        doi: '10.1371/journal.pone.0127463',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-anchor-strength-six-week-pig',
    feature: 'Pig pull-out three months after six-week surgery',
    publishedNumber:
      'mean: 397 N [published; yilmaz-2009]; sd: 80 N [published; yilmaz-2009]; dilated mean: about 379 N ' +
      '[published; yilmaz-2009]; dilated sd: 73 N [published; yilmaz-2009]; diameter: 4.0 mm [published; ' +
      'yilmaz-2009]; length: 20 mm [published; yilmaz-2009]; dilated mean abstract: 378.7 N [published; ' +
      'yilmaz-2009]. Six weeks old at surgery;pull-out three months later;data from two animals (three ' +
      'operated,one lost),L1–L5. Original note rounded dilated mean379N;abstract precision378.7N retained ' +
      'separately. Table use: beside.',
    population:
      'yilmaz-2009: Pigs six weeks old at surgery;pull-out tested three months later after killing;L1–L5. Data ' +
      'from two surviving animals. Sample: 2 animals with data;3 operated,1 lost. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Yilmaz G, Demirkiran G, Ozkan C, Daglioglu K, Pekmezci M, Alanay A, Yazici M. 2009. The effect of ' +
          'dilation of immature pedicles on pullout strength of the screws: Part 2: In vivo study. Spine (Phila Pa ' +
          '1976) 34(22):2378-2383.',
        pmid: '19829250',
        doi: '10.1097/brs.0b013e3181b0bb9b',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-anchor-strength-osteoporotic-lumbar',
    feature: 'Pull-out after cyclic loading',
    publishedNumber:
      'mid body: 583 N [published; karami-2015]; mid body sd: 306 N [published; karami-2015]; pericortical: 713 ' +
      'N [published; karami-2015]; pericortical sd: 321 N [published; karami-2015]; bicortical: 797 N ' +
      '[published; karami-2015]; bicortical sd: 285 N [published; karami-2015]. Table B endpoints retain the ' +
      'associated uncertainties and intermediate depth from the supplying paragraph. Table use: beside.',
    population:
      'karami-2015: Osteoporotic lumbar spines; pull-out after cyclic loading. Age not provided in the supplying ' +
      'passage. Sample: 10 spines;100 pedicles. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Karami KJ, Buckenmeyer LE, Kiapour AM, Kelkar PS, Goel VK, Demetropoulos CK, Soo TM. 2015. Biomechanical ' +
          'evaluation of the pedicle screw insertion depth effect on screw stability under cyclic loading and ' +
          'subsequent pullout. J Spinal Disord Tech 28(3):E133-9.',
        pmid: '25310387',
        doi: '10.1097/bsd.0000000000000178',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-anchor-strength-immature-calf',
    feature: 'Immature calf diameter comparison',
    publishedNumber:
      'six mm table: 853 N [published; sairyo-1998]; seven mm: 1015 N [published; sairyo-1998]; six mm body ' +
      'text: 852.5 N [published; sairyo-1998]; smaller diameter: 6 mm [published; sairyo-1998]; larger diameter: ' +
      '7 mm [published; sairyo-1998]. DISAGREE in rounding: table853 versus body852.5; preserve both, never ' +
      'round or substitute. Table use: beside.',
    population: 'sairyo-1998: Immature calf; 6 and7 mm screws. Sample: 10 vertebrae. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Sairyo K, Scifert J, Goel VK, Grosland N, Grobler LJ. 1998. Neurocentral synchondrosis fracture in ' +
          'immature spines associated with pedicle screw type fixation devices. J Spinal Disord 11(2):142-145.',
        pmid: '9588471',
        doi: '10.1097/00002517-199804000-00009',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-anchor-strength-immature-calf-hubbing',
    feature: 'Immature calf thoracic hubbing',
    publishedNumber:
      'non hubbed: 922 N [published; kang-2014]; non hubbed sd: 112 N [published; kang-2014]; hubbed: 747 N ' +
      '[published; kang-2014]; hubbed sd: 197 N [published; kang-2014]. Immature animal comparison, not human ' +
      'paediatric strength. Table use: beside.',
    population:
      'kang-2014: Immature calf thoracic vertebrae; hubbed versus non-hubbed. Sample: 22 specimens. ' +
      '[peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Kang DG, Lehman RA Jr, Bevevino AJ, Gaume RE, Purcell RL, Dmitriev AE, Lenke LG. 2014. Pedicle screw ' +
          '"hubbing" in the immature thoracic spine: a biomechanical and micro-computed tomography evaluation. J ' +
          'Pediatr Orthop 34(7):703-709.',
        pmid: '24590340',
        doi: '10.1097/bpo.0000000000000166',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-anchor-strength-foam-reference',
    feature: 'ASTM F1839 foam reference',
    publishedNumber:
      'lower: 1568 N [published; tsai-2009]; upper: 1904 N [published; tsai-2009]; diameter: 6 mm [published; ' +
      'tsai-2009]; engaged length: 30 mm [published; tsai-2009]. Not cadaver bone; do not use as a patient ' +
      'strength. Table use: beside.',
    population:
      'tsai-2009: ASTM F1839 polyurethane foam; 30 mm engaged thread. Sample: 6 commercial screws;6 trials each. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation: 'Tsai WC, Chen PQ, Lu TW, Wu SS, Shih KS, Lin SC. 2009. BMC Musculoskelet Disord 10:44.',
        pmid: '19402917',
        doi: '10.1186/1471-2474-10-44',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-anchor-strength-trajectory-multiplier',
    feature: 'Pull-out comparison multiplier',
    publishedNumber:
      'straight forward over anatomic: 1.27 ratio [published; lehman-2003]. One study; multipliers are not ' +
      'established to stack. Table use: beside.',
    population:
      'lehman-2003: Adult thoracic cadaver vertebrae; straight-forward and anatomic trajectories. Sample: 30 ' +
      'vertebrae, paired. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Lehman RA, Polly DW, Kuklo TR, Cunningham B, Kirk KL, Belmont PJ. 2003. Straight-forward versus anatomic ' +
          'trajectory technique of thoracic pedicle screw fixation: a biomechanical analysis. Spine (Phila Pa 1976) ' +
          '28(18):2058-2065.',
        pmid: '14501914',
        doi: '10.1097/01.brs.0000087743.57439.4f',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-anchor-strength-depth-multiplier',
    feature: 'Pull-out comparison multiplier',
    publishedNumber:
      'bicortical over mid body: 1.37 ratio [published; karami-2015]. One study; multipliers are not established ' +
      'to stack. Table use: beside.',
    population:
      'karami-2015: Osteoporotic lumbar spines; pull-out after cyclic loading. Age not provided in the supplying ' +
      'passage. Sample: 10 spines;100 pedicles. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Karami KJ, Buckenmeyer LE, Kiapour AM, Kelkar PS, Goel VK, Demetropoulos CK, Soo TM. 2015. Biomechanical ' +
          'evaluation of the pedicle screw insertion depth effect on screw stability under cyclic loading and ' +
          'subsequent pullout. J Spinal Disord Tech 28(3):E133-9.',
        pmid: '25310387',
        doi: '10.1097/bsd.0000000000000178',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-anchor-strength-hubbing-multiplier',
    feature: 'Pull-out comparison multiplier',
    publishedNumber:
      'factor: 0.81 ratio [published; kang-2014]. One study; multipliers are not established to stack. Table ' +
      'use: beside.',
    population:
      'kang-2014: Immature calf thoracic vertebrae; hubbed versus non-hubbed. Sample: 22 specimens. ' +
      '[peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Kang DG, Lehman RA Jr, Bevevino AJ, Gaume RE, Purcell RL, Dmitriev AE, Lenke LG. 2014. Pedicle screw ' +
          '"hubbing" in the immature thoracic spine: a biomechanical and micro-computed tomography evaluation. J ' +
          'Pediatr Orthop 34(7):703-709.',
        pmid: '24590340',
        doi: '10.1097/bpo.0000000000000166',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-anchor-strength-direct-rotation-multiplier',
    feature: 'Pull-out comparison multiplier',
    publishedNumber:
      'factor: 0.65 ratio [published; sariyilmaz-2021]. One study; multipliers are not established to stack. ' +
      'Table use: beside.',
    population:
      'sariyilmaz-2021: Adult cadaver thoracic; 5.5 mm polyaxial screws; simulated direct vertebral rotation. ' +
      'Sample: 30 pedicles. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Sariyilmaz K, Ozkunt O, Can Gemalmaz H, Cingoz T, Pehlivanoglu T, Aksoy T, Kaya O, Baydogan M, Dikici F. ' +
          '2021. Direct vertebral rotation significantly decreases the pullout strength of the pedicle screw: a ' +
          'biomechanical study in adult cadavers. J Pediatr Orthop B 30(3):230-234.',
        pmid: '32453125',
        doi: '10.1097/bpb.0000000000000751',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-anchor-strength-diameter-multiplier',
    feature: 'Diameter increment comparison',
    publishedNumber:
      'lower: 1.19 ratio [published; sairyo-1998]; upper: 1.24 ratio [published; kueny-2014]; diameter ' +
      'increment: 1 mm [published; kueny-2014]. Range joins two studies; neither comparison significant. No new ' +
      'capacity scaling. Table use: beside.',
    population:
      'sairyo-1998: Immature calf; 6 and7 mm screws. Sample: 10 vertebrae. [peer-reviewed; abstract]. ' +
      'kueny-2014: Osteoporotic lumbar vertebrae; diameter increment comparison. Age not provided in the ' +
      'supplying passage. Sample: 39 vertebrae. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Sairyo K, Scifert J, Goel VK, Grosland N, Grobler LJ. 1998. Neurocentral synchondrosis fracture in ' +
          'immature spines associated with pedicle screw type fixation devices. J Spinal Disord 11(2):142-145.',
        pmid: '9588471',
        doi: '10.1097/00002517-199804000-00009',
      },
      {
        citation:
          'Kueny RA, Kolb JP, Lehmann W, Püschel K, Morlock MM, Huber G. 2014. Influence of the screw augmentation ' +
          'technique and a diameter increase on pedicle screw fixation in the osteoporotic spine: pullout versus ' +
          'fatigue testing. Eur Spine J 23(10):2196-2202.',
        pmid: '25082759',
        doi: '10.1007/s00586-014-3476-7',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-anchor-strength-low-bmd-fatigue',
    feature: 'Low versus high BMD cyclic comparison',
    publishedNumber:
      'fatigue load: 0.6 ratio [published; weiser-2017]; cycles: 0.45 ratio [published; weiser-2017]; low bmd ' +
      'upper exclusive: 80 mg/cm³ [published; weiser-2017]; high bmd lower exclusive: 120 mg/cm³ [published; ' +
      'weiser-2017]. Low BMD <80 versus high BMD >120 mg/cm³; no slope inferred. Table use: beside.',
    population:
      'weiser-2017: QCT bone-density groups; cyclic screw failure. Age not provided in the supplying passage. ' +
      'Sample: 21 T12 vertebrae. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Weiser L, Huber G, Sellenschloh K, Viezens L, Püschel K, Morlock MM, Lehmann W. 2017. Insufficient ' +
          'stability of pedicle screws in osteoporotic vertebrae: biomechanical correlation of bone mineral density ' +
          'and pedicle screw fixation strength. Eur Spine J 26(11):2891-2897.',
        pmid: '28391382',
        doi: '10.1007/s00586-017-5091-x',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-anchor-strength-cortical-trajectory',
    feature: 'Lumbar cortical versus traditional trajectory',
    publishedNumber:
      'yield pull out: 1.3 ratio [published; santoni-2009]; p: 0.08 ratio [published; santoni-2009]. P=0.08 as ' +
      'Table C prints it; not a significant effect. Table use: beside.',
    population:
      'santoni-2009: lumbar cortical versus traditional screw trajectory. Age not provided in the supplying ' +
      'passage. Sample: not supplied. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Santoni BG, Hynes RA, McGilvray KC, Rodriguez-Canessa G, Lyons AS, Henson MA, Womack WJ, Puttlitz CM. ' +
          '2009. Cortical bone trajectory for lumbar pedicle screws. Spine J 9(5):366-373.',
        pmid: '18790684',
        doi: '10.1016/j.spinee.2008.07.008',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-head-neck',
    feature: 'Borrowed head+neck mass',
    publishedNumber:
      'fraction: 0.081 fraction-of-body-weight [published; winter-via-keenan-2015]. As printed in Keenan2015 ' +
      'equations4/5,attributed there to Winter DA, Biomechanics and Motor Control of Human Movement,4th ed,John ' +
      'Wiley & Sons,2009. Textbook not opened;no memory-based substitution. Original cadaver population eight ' +
      'males aged61–83. Table use: bench.',
    population:
      'winter-via-keenan-2015: Winter cadavers aged61–83; fractions borrowed in AIS girls with Lenke1 ' +
      'curves>40°. Sample: 8 male cadavers (Winter);20 girls (Keenan). [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Keenan BE, Pettet GJ, Izatt MT, Askin GN, Labrom RD, Pearcy MJ, Adam C. 2015. Gravity-induced coronal ' +
          'plane joint moments in adolescent idiopathic scoliosis. Scoliosis 10:35.',
        pmid: '26681978',
        doi: '10.1186/s13013-015-0060-9',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-each-arm-hand',
    feature: 'Borrowed each arm+hand mass',
    publishedNumber:
      'fraction: 0.056 fraction-of-body-weight [published; winter-via-keenan-2015]. As printed in Keenan2015 ' +
      'equations4/5,attributed there to Winter DA, Biomechanics and Motor Control of Human Movement,4th ed,John ' +
      'Wiley & Sons,2009. Textbook not opened;no memory-based substitution. Original cadaver population eight ' +
      'males aged61–83. Applying arms overT1–T4 is own inference. Table use: bench.',
    population:
      'winter-via-keenan-2015: Winter cadavers aged61–83; fractions borrowed in AIS girls with Lenke1 ' +
      'curves>40°. Sample: 8 male cadavers (Winter);20 girls (Keenan). [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Keenan BE, Pettet GJ, Izatt MT, Askin GN, Labrom RD, Pearcy MJ, Adam C. 2015. Gravity-induced coronal ' +
          'plane joint moments in adolescent idiopathic scoliosis. Scoliosis 10:35.',
        pmid: '26681978',
        doi: '10.1186/s13013-015-0060-9',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-pearsall-slices',
    feature: 'Adult published trunk slice endpoints',
    publishedNumber:
      't1: 1.1 percent [published; pearsall-1996]; l5: 2.6 percent [published; pearsall-1996]; trunk total: 41.6 ' +
      'percent [published; pearsall-1996]; t1 fraction: about 0.011 fraction-of-body-weight [published; ' +
      'pearsall-1996]; trunk total rounded fraction: about 0.42 fraction-of-body-weight [published; ' +
      'pearsall-1996]. Adult T1 differs from rounded AIS 0.010 fraction; retained separately, not converted. ' +
      'Table use: input.',
    population:
      'pearsall-1996: Adults, MeSH middle-aged/aged; CT trunk by vertebral level. Sample: 4. [peer-reviewed; ' +
      'abstract].',
    sources: [
      {
        citation:
          'Pearsall DJ, Reid JG, Livingston LA. 1996. Segmental inertial parameters of the human trunk as determined ' +
          'from computed tomography. Ann Biomed Eng 24(2):198-210.',
        pmid: '8678352',
        doi: '10.1007/bf02667349',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-keenan-slices',
    feature: 'AIS slice inputs',
    publishedNumber:
      't1: 0.6 kg [published; keenan-2014]; l5: 1.5 kg [published; keenan-2014]; body mean: 58.2 kg [published; ' +
      'keenan-2014]; body sd: 11.6 kg [published; keenan-2014]; trunk total: 27.8 percent [published; ' +
      'keenan-2014]; trunk total sd: 0.5 percent [published; keenan-2014]; trunk total fraction: 0.278 ' +
      'fraction-of-body-weight [published; keenan-2014]. No full 17-level table in note. Table use: input.',
    population:
      'keenan-2014: AIS girls, age 15.0±2.7, Cobb52±5.9°, weight58.2±11.6 kg; CT torso slices. Sample: 20. ' +
      '[peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Keenan BE, Izatt MT, Askin GN, Labrom RD, Pettet GJ, Pearcy MJ, Adam CJ. 2014. Segmental torso masses in ' +
          'adolescent idiopathic scoliosis. Clin Biomech (Bristol) 29(7):773-779.',
        pmid: '24973826',
        doi: '10.1016/j.clinbiomech.2014.06.002',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-slice-offset',
    feature: 'Own sagittal offset prior beside raw gravity-line distances',
    publishedNumber:
      'mean: 28 mm [own-inference; project-assumptions]; sd: 5.6 mm [own-inference; project-assumptions]; ' +
      'distance 3d mean: 28 mm [published; gangnet-2003]; distance 3d sd: 5.6 mm [published; gangnet-2003]; left ' +
      'lateral mean: 7.6 mm [published; gangnet-2003]; left lateral sd: 1.6 mm [published; gangnet-2003]. ' +
      'Published28mm(SD5.6)is an unsigned3D body-centre/gravity-line distance;7.6mm(SD1.6)is left lateral. ' +
      'Using28/5.6as sagittal anterior offset distribution is own inference. Frontal centroid at body centre ' +
      'ignores the published left lateralization and remains own choice. Table use: bench.',
    population:
      'project-assumptions: No measured population; explicit choices and calculations in the project notes. ' +
      'Sample: not supplied. [own-inference; secondary]. gangnet-2003: 34 healthy upright standing adults as ' +
      'described in the task;the abstract supplies no ages. Sample: 34. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Gangnet N, Pomero V, Dumas R, Skalli W, Vital JM. 2003. Variability of the spine and pelvis location with ' +
          'respect to the gravity line: a three-dimensional stereoradiographic study using a force platform. Surg ' +
          'Radiol Anat 25(5-6):424-433.',
        pmid: '13680185',
        doi: '10.1007/s00276-003-0154-6',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-walking',
    feature: 'Walking/standing endpoints',
    publishedNumber:
      'lower: 1.28 ratio [published; rohlmann-2002]; upper: 1.71 ratio [published; rohlmann-2014-gait]. DISAGREE ' +
      'in measured quantity:lower1.28is load on an internal spinal fixation device;upper1.71is resultant force ' +
      'on a vertebral body replacement. Both normalized to their own standing reference. Sampling between these ' +
      'quantities/populations is own inference. Table use: bench.',
    population:
      'rohlmann-2002: Adults with telemetered posterior internal fixators; fractures/degenerative disease, not ' +
      'scoliosis. Sample: 10. [peer-reviewed; abstract]. rohlmann-2014-gait: Adults with instrumented vertebral ' +
      'body replacements; walking/stairs. Sample: 5. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Rohlmann A, Graichen F, Bergmann G. 2002. Loads on an internal spinal fixation device during physical ' +
          'therapy. Phys Ther 82(1):44-52.',
        pmid: '11784277',
        doi: '10.1093/ptj/82.1.44',
      },
      {
        citation:
          'Rohlmann A, Dreischarf M, Zander T, Graichen F, Bergmann G. 2014. Loads on a vertebral body replacement ' +
          'during locomotion measured in vivo. Gait Posture 39(2):750-755.',
        pmid: '24211089',
        doi: '10.1016/j.gaitpost.2013.10.010',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-walking-published-percent',
    feature: 'Walking measurements in published units',
    publishedNumber:
      'fixator: 128 percent [published; rohlmann-2002]; vbr: 171 percent [published; rohlmann-2014-gait]. ' +
      'Fixator128 percentis internal fixation-device load;VBR171 percentis vertebral-body-replacement resultant ' +
      'force,each relative to standing. Not the same measured quantity. Table use: input.',
    population:
      'rohlmann-2002: Adults with telemetered posterior internal fixators; fractures/degenerative disease, not ' +
      'scoliosis. Sample: 10. [peer-reviewed; abstract]. rohlmann-2014-gait: Adults with instrumented vertebral ' +
      'body replacements; walking/stairs. Sample: 5. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Rohlmann A, Graichen F, Bergmann G. 2002. Loads on an internal spinal fixation device during physical ' +
          'therapy. Phys Ther 82(1):44-52.',
        pmid: '11784277',
        doi: '10.1093/ptj/82.1.44',
      },
      {
        citation:
          'Rohlmann A, Dreischarf M, Zander T, Graichen F, Bergmann G. 2014. Loads on a vertebral body replacement ' +
          'during locomotion measured in vivo. Gait Posture 39(2):750-755.',
        pmid: '24211089',
        doi: '10.1016/j.gaitpost.2013.10.010',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-bending-forward',
    feature: 'Standing-flexed pressure ratio',
    publishedNumber:
      'multiple: 2.2 ratio [own-inference; project-assumptions]; standing flexed: 1.1 MPa [published; ' +
      'wilke-1999]; standing relaxed: 0.5 MPa [published; wilke-1999]. Pressure ratio borrowed as passive ' +
      'standing-load multiplier; one adult; no independent flexed posture or muscle forces. Table use: bench.',
    population:
      'project-assumptions: No measured population; explicit choices and calculations in the project notes. ' +
      'Sample: not supplied. [own-inference; secondary]. wilke-1999: Man aged45, 70 kg; intradiscal pressure ' +
      'atL4–L5. Sample: 1. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Wilke HJ, Neef P, Caimi M, Hoogland T, Claes LE. 1999. New in vivo measurements of pressures in the ' +
          'intervertebral disc in daily life. Spine (Phila Pa 1976) 24(8):755-762.',
        pmid: '10222525',
        doi: '10.1097/00007632-199904150-00005',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-weight-if-absent',
    feature: 'AIS model cohort weight endpoints',
    publishedNumber:
      'lower: 29 kg [published; schmid-2020-ais]; upper: 63 kg [published; schmid-2020-ais]. Sampling missing ' +
      'body weight uniformly over cohort range is own inference. Table use: bench.',
    population:
      'schmid-2020-ais: AIS aged9–17(mean13.8), 29–63 kg, Cobb13.6–28.3°; musculoskeletal models. Sample: 24. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Schmid S, Burkhart KA, Allaire BT, Grindle D, Bassani T, Galbusera F, Anderson DE. 2020. Spinal ' +
          'Compressive Forces in Adolescent Idiopathic Scoliosis With and Without Carrying Loads: A Musculoskeletal ' +
          'Modeling Study. Front Bioeng Biotechnol 8:159.',
        pmid: '32195239',
        doi: '10.3389/fbioe.2020.00159',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-steps-very-low',
    feature: 'Adult activity band very-low',
    publishedNumber:
      'upper exclusive: 5000 steps/day [published; tudor-locke-2008]. Bench uses literal adult endpoints; ' +
      'applying to adolescents is own inference. Open end remains absent. Table use: bench.',
    population:
      'tudor-locke-2008: Adult step bands; youth cut points not established. Sample: not supplied. ' +
      '[peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Tudor-Locke C, Hatano Y, Pangrazi RP, Kang M. 2008. Revisiting "how many steps are enough?". Med Sci ' +
          'Sports Exerc 40(7 Suppl):S537-43.',
        pmid: '18562971',
        doi: '10.1249/mss.0b013e31817c7133',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-steps-low',
    feature: 'Adult activity band low',
    publishedNumber:
      'lower inclusive: 5000 steps/day [published; tudor-locke-2008]; upper inclusive: 7499 steps/day ' +
      '[published; tudor-locke-2008]. Bench uses literal adult endpoints; applying to adolescents is own ' +
      'inference. Open end remains absent. Table use: bench.',
    population:
      'tudor-locke-2008: Adult step bands; youth cut points not established. Sample: not supplied. ' +
      '[peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Tudor-Locke C, Hatano Y, Pangrazi RP, Kang M. 2008. Revisiting "how many steps are enough?". Med Sci ' +
          'Sports Exerc 40(7 Suppl):S537-43.',
        pmid: '18562971',
        doi: '10.1249/mss.0b013e31817c7133',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-steps-typical',
    feature: 'Adult activity band typical',
    publishedNumber:
      'lower inclusive: 7500 steps/day [published; tudor-locke-2008]; upper inclusive: 9999 steps/day ' +
      '[published; tudor-locke-2008]. Bench uses literal adult endpoints; applying to adolescents is own ' +
      'inference. Open end remains absent. Table use: bench.',
    population:
      'tudor-locke-2008: Adult step bands; youth cut points not established. Sample: not supplied. ' +
      '[peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Tudor-Locke C, Hatano Y, Pangrazi RP, Kang M. 2008. Revisiting "how many steps are enough?". Med Sci ' +
          'Sports Exerc 40(7 Suppl):S537-43.',
        pmid: '18562971',
        doi: '10.1249/mss.0b013e31817c7133',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-steps-active',
    feature: 'Adult activity band active',
    publishedNumber:
      'lower inclusive: 10000 steps/day [published; tudor-locke-2008]; upper inclusive: 12499 steps/day ' +
      '[published; tudor-locke-2008]. Bench uses literal adult endpoints; applying to adolescents is own ' +
      'inference. Open end remains absent. Table use: bench.',
    population:
      'tudor-locke-2008: Adult step bands; youth cut points not established. Sample: not supplied. ' +
      '[peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Tudor-Locke C, Hatano Y, Pangrazi RP, Kang M. 2008. Revisiting "how many steps are enough?". Med Sci ' +
          'Sports Exerc 40(7 Suppl):S537-43.',
        pmid: '18562971',
        doi: '10.1249/mss.0b013e31817c7133',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-steps-very-active',
    feature: 'Adult activity band very-active',
    publishedNumber:
      'lower inclusive: 12500 steps/day [published; tudor-locke-2008]. Bench uses literal adult endpoints; ' +
      'applying to adolescents is own inference. Open end remains absent. Table use: bench.',
    population:
      'tudor-locke-2008: Adult step bands; youth cut points not established. Sample: not supplied. ' +
      '[peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Tudor-Locke C, Hatano Y, Pangrazi RP, Kang M. 2008. Revisiting "how many steps are enough?". Med Sci ' +
          'Sports Exerc 40(7 Suppl):S537-43.',
        pmid: '18562971',
        doi: '10.1249/mss.0b013e31817c7133',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-child-steps',
    feature: 'Children/adolescent reference ranges',
    publishedNumber:
      'boys lower: about 12000 steps/day [published; tudor-locke-2011]; boys upper: about 16000 steps/day ' +
      '[published; tudor-locke-2011]; girls lower: about 10000 steps/day [published; tudor-locke-2011]; girls ' +
      'upper: about 13000 steps/day [published; tudor-locke-2011]; by 18 lower: about 8000 steps/day [published; ' +
      'tudor-locke-2011]; by 18 upper: about 9000 steps/day [published; tudor-locke-2011]. Review ranges; no ' +
      'established youth activity cut points. Table use: input.',
    population:
      'tudor-locke-2011: Review of children/adolescent steps per day. Sample: not supplied. [peer-reviewed; ' +
      'full-text].',
    sources: [
      {
        citation:
          'Tudor-Locke C, Craig CL, Beets MW, Belton S, Cardon GM, Duncan S, Hatano Y, Lubans DR, Olds TS, Raustorp ' +
          'A, Rowe DA, Spence JC, Tanaka S, Blair SN. 2011. How many steps/day are enough? for children and ' +
          'adolescents. Int J Behav Nutr Phys Act 8:78.',
        pmid: '21798014',
        doi: '10.1186/1479-5868-8-78',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-tissue-density',
    feature: 'AIS gravity model tissue-density input',
    publishedNumber:
      'density: 1040 kg/m³ [published; keenan-2015]. Model-assigned density, not measured from the films. Table ' +
      'use: input.',
    population:
      'keenan-2015: AIS Lenke1 curves over40°, mean mass57.5kg; gravity moment model. Sample: 20 girls. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Keenan BE, Pettet GJ, Izatt MT, Askin GN, Labrom RD, Pearcy MJ, Adam C. 2015. Gravity-induced coronal ' +
          'plane joint moments in adolescent idiopathic scoliosis. Scoliosis 10:35.',
        pmid: '26681978',
        doi: '10.1186/s13013-015-0060-9',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-lying-relaxed',
    feature: 'Lying load comparison',
    publishedNumber:
      'multiple upper exclusive: 0.3 ratio [published; rohlmann-2008-standing]; pressure ratio: 0.2 ratio ' +
      '[own-inference; project-assumptions]; lying pressure: 0.1 MPa [published; wilke-1999]; standing pressure: ' +
      '0.5 MPa [published; wilke-1999]. Under 0.3 VBR standing load; pressure ratio is a separate one-adult ' +
      'inferred comparison. Table use: beside.',
    population:
      'rohlmann-2008-standing: Adults with instrumented vertebral body replacements, one month after surgery. ' +
      'Sample: 2. [peer-reviewed; abstract]. project-assumptions: No measured population; explicit choices and ' +
      'calculations in the project notes. Sample: not supplied. [own-inference; secondary]. wilke-1999: Man ' +
      'aged45, 70 kg; intradiscal pressure atL4–L5. Sample: 1. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Rohlmann A, Graichen F, Kayser R, Bender A, Bergmann G. 2008. Loads on a telemeterized vertebral body ' +
          'replacement measured in two patients. Spine (Phila Pa 1976) 33(11):1170-1179.',
        pmid: '18469689',
        doi: '10.1097/brs.0b013e3181722d52',
      },
      {
        citation:
          'Wilke HJ, Neef P, Caimi M, Hoogland T, Claes LE. 1999. New in vivo measurements of pressures in the ' +
          'intervertebral disc in daily life. Spine (Phila Pa 1976) 24(8):755-762.',
        pmid: '10222525',
        doi: '10.1097/00007632-199904150-00005',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-sitting-unsupported',
    feature: 'Unsupported sitting pressure comparison',
    publishedNumber:
      'multiple: about 0.9 ratio [own-inference; project-assumptions]; sitting pressure: 0.46 MPa [published; ' +
      'wilke-1999]; standing pressure: 0.5 MPa [published; wilke-1999]. Note rounds pressure ratio to about 0.9; ' +
      'no new division. Table use: beside.',
    population:
      'project-assumptions: No measured population; explicit choices and calculations in the project notes. ' +
      'Sample: not supplied. [own-inference; secondary]. wilke-1999: Man aged45, 70 kg; intradiscal pressure ' +
      'atL4–L5. Sample: 1. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Wilke HJ, Neef P, Caimi M, Hoogland T, Claes LE. 1999. New in vivo measurements of pressures in the ' +
          'intervertebral disc in daily life. Spine (Phila Pa 1976) 24(8):755-762.',
        pmid: '10222525',
        doi: '10.1097/00007632-199904150-00005',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-stairs',
    feature: 'VBR stairs/standing ratios',
    publishedNumber:
      'down: 2.25 ratio [published; rohlmann-2014-gait]; up: 2.65 ratio [published; rohlmann-2014-gait]. Adult ' +
      'VBR population, not posterior rods. Table use: beside.',
    population:
      'rohlmann-2014-gait: Adults with instrumented vertebral body replacements; walking/stairs. Sample: 5. ' +
      '[peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Rohlmann A, Dreischarf M, Zander T, Graichen F, Bergmann G. 2014. Loads on a vertebral body replacement ' +
          'during locomotion measured in vivo. Gait Posture 39(2):750-755.',
        pmid: '24211089',
        doi: '10.1016/j.gaitpost.2013.10.010',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-lifting-round-back',
    feature: 'Round-back20 kg lift pressure comparison',
    publishedNumber:
      'multiple: 4.6 ratio [own-inference; project-assumptions]; lifting pressure: 2.3 MPa [published; ' +
      'wilke-1999]; standing pressure: 0.5 MPa [published; wilke-1999]; weight: 20 kg [published; wilke-1999]. ' +
      'One adult pressure input, not a prescribed paediatric load. Table use: beside.',
    population:
      'project-assumptions: No measured population; explicit choices and calculations in the project notes. ' +
      'Sample: not supplied. [own-inference; secondary]. wilke-1999: Man aged45, 70 kg; intradiscal pressure ' +
      'atL4–L5. Sample: 1. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Wilke HJ, Neef P, Caimi M, Hoogland T, Claes LE. 1999. New in vivo measurements of pressures in the ' +
          'intervertebral disc in daily life. Spine (Phila Pa 1976) 24(8):755-762.',
        pmid: '10222525',
        doi: '10.1097/00007632-199904150-00005',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-lifting-small-weights',
    feature: 'VBR lifting small weights',
    publishedNumber:
      'multiple upper inclusive: 5 ratio [published; rohlmann-2013-lifting]. Published as up to 5 times ' +
      'standing; no frequency inferred. Table use: beside.',
    population:
      'rohlmann-2013-lifting: Adults with instrumented vertebral body replacements; lifting/setting down0.01–3 ' +
      'kg. Sample: not supplied. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Rohlmann A, Zander T, Graichen F, Bergmann G. 2013. Lifting up and laying down a weight causes high ' +
          'spinal loads. J Biomech 46(3):511-514.',
        pmid: '23141957',
        doi: '10.1016/j.jbiomech.2012.10.022',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-backpack-10-percent',
    feature: 'AIS model apical load with carried weight',
    publishedNumber:
      'body weight: 10 percent [published; schmid-2020-ais]; multiple lower: 1.5 ratio [published; ' +
      'schmid-2020-ais]; multiple upper: 1.62 ratio [published; schmid-2020-ais]. Range spans carriage ' +
      'configurations; adult passive stiffness borrowed in AIS model. Table use: beside.',
    population:
      'schmid-2020-ais: AIS aged9–17(mean13.8), 29–63 kg, Cobb13.6–28.3°; musculoskeletal models. Sample: 24. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Schmid S, Burkhart KA, Allaire BT, Grindle D, Bassani T, Galbusera F, Anderson DE. 2020. Spinal ' +
          'Compressive Forces in Adolescent Idiopathic Scoliosis With and Without Carrying Loads: A Musculoskeletal ' +
          'Modeling Study. Front Bioeng Biotechnol 8:159.',
        pmid: '32195239',
        doi: '10.3389/fbioe.2020.00159',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-backpack-15-percent',
    feature: 'AIS model apical load with carried weight',
    publishedNumber:
      'body weight: 15 percent [published; schmid-2020-ais]; multiple lower: 1.77 ratio [published; ' +
      'schmid-2020-ais]; multiple upper: 1.94 ratio [published; schmid-2020-ais]. Range spans carriage ' +
      'configurations; adult passive stiffness borrowed in AIS model. Table use: beside.',
    population:
      'schmid-2020-ais: AIS aged9–17(mean13.8), 29–63 kg, Cobb13.6–28.3°; musculoskeletal models. Sample: 24. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Schmid S, Burkhart KA, Allaire BT, Grindle D, Bassani T, Galbusera F, Anderson DE. 2020. Spinal ' +
          'Compressive Forces in Adolescent Idiopathic Scoliosis With and Without Carrying Loads: A Musculoskeletal ' +
          'Modeling Study. Front Bioeng Biotechnol 8:159.',
        pmid: '32195239',
        doi: '10.3389/fbioe.2020.00159',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-backpack-20-percent',
    feature: 'AIS model apical load with carried weight',
    publishedNumber:
      'body weight: 20 percent [published; schmid-2020-ais]; multiple lower: 2.03 ratio [published; ' +
      'schmid-2020-ais]; multiple upper: 2.28 ratio [published; schmid-2020-ais]. Range spans carriage ' +
      'configurations; adult passive stiffness borrowed in AIS model. Table use: beside.',
    population:
      'schmid-2020-ais: AIS aged9–17(mean13.8), 29–63 kg, Cobb13.6–28.3°; musculoskeletal models. Sample: 24. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Schmid S, Burkhart KA, Allaire BT, Grindle D, Bassani T, Galbusera F, Anderson DE. 2020. Spinal ' +
          'Compressive Forces in Adolescent Idiopathic Scoliosis With and Without Carrying Loads: A Musculoskeletal ' +
          'Modeling Study. Front Bioeng Biotechnol 8:159.',
        pmid: '32195239',
        doi: '10.3389/fbioe.2020.00159',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-scoliosis-vs-straight',
    feature: 'Standing model geometry comparison',
    publishedNumber:
      'at apex: about 1.1 ratio [published; schmid-2020-ais]; below apex: about 1.18 ratio [published; ' +
      'schmid-2020-ais]. Same-patient undeformed model reference, not activity/standing ratio. Table use: ' +
      'beside.',
    population:
      'schmid-2020-ais: AIS aged9–17(mean13.8), 29–63 kg, Cobb13.6–28.3°; musculoskeletal models. Sample: 24. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Schmid S, Burkhart KA, Allaire BT, Grindle D, Bassani T, Galbusera F, Anderson DE. 2020. Spinal ' +
          'Compressive Forces in Adolescent Idiopathic Scoliosis With and Without Carrying Loads: A Musculoskeletal ' +
          'Modeling Study. Front Bioeng Biotechnol 8:159.',
        pmid: '32195239',
        doi: '10.3389/fbioe.2020.00159',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-adolescent-mvpa',
    feature: 'Adolescent steps associated with activity duration',
    publishedNumber:
      'lower: about 10000 steps/day [published; tudor-locke-2011]; upper: about 11700 steps/day [published; ' +
      'tudor-locke-2011]; duration: 60 min [published; tudor-locke-2011]. Association with60 minutes ' +
      'moderate-to-vigorous activity, not a youth cut-point system. Table use: beside.',
    population:
      'tudor-locke-2011: Review of children/adolescent steps per day. Sample: not supplied. [peer-reviewed; ' +
      'full-text].',
    sources: [
      {
        citation:
          'Tudor-Locke C, Craig CL, Beets MW, Belton S, Cardon GM, Duncan S, Hatano Y, Lubans DR, Olds TS, Raustorp ' +
          'A, Rowe DA, Spence JC, Tanaka S, Blair SN. 2011. How many steps/day are enough? for children and ' +
          'adolescents. Int J Behav Nutr Phys Act 8:78.',
        pmid: '21798014',
        doi: '10.1186/1479-5868-8-78',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-vbr-supine',
    feature: 'VBR supine relaxed preload',
    publishedNumber:
      'lower: 42 N [published; rohlmann-2014-activities]; upper: 96 N [published; rohlmann-2014-activities]. ' +
      'Across adult patients, not standing multiples. Table use: beside.',
    population:
      'rohlmann-2014-activities: Adults aged62–71, 60–74kg; L1/L3vertebral body replacements, posterior fixator ' +
      'shares load. Sample: 5. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Rohlmann A, Pohl D, Bender A, Graichen F, Dymke J, Schmidt H, Bergmann G. 2014. Activities of everyday ' +
          'life with high spinal loads. PLoS One 9(5):e98510.',
        pmid: '24866883',
        doi: '10.1371/journal.pone.0098510',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-vbr-month-one',
    feature: 'First-month VBR sitting and standing',
    publishedNumber:
      'lower: 150 N [published; rohlmann-2008-month-one]; upper: 450 N [published; rohlmann-2008-month-one]. ' +
      'Three patients; distinct from two-patient Spine2008source. Table use: beside.',
    population:
      'rohlmann-2008-month-one: Adults with vertebral body replacements; first postoperative month. Sample: 3. ' +
      '[peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Rohlmann A, Graichen F, Bender A, Kayser R, Bergmann G. 2008. Loads on a telemeterized vertebral body ' +
          'replacement measured in three patients within the first postoperative month. Clin Biomech (Bristol) ' +
          '23(2):147-158.',
        pmid: '17983694',
        doi: '10.1016/j.clinbiomech.2007.09.011',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-walking-body-weight',
    feature: 'Mean VBR walking peak fraction',
    publishedNumber:
      'fraction: 0.39 fraction-of-body-weight [published; damm-2017]. Fraction of body weight, not multiple of ' +
      'standing. Table use: beside.',
    population:
      'damm-2017: Adults with instrumented vertebral body replacements; mean walking peak. Sample: 5 VBR ' +
      'patients. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Damm P, Kutzner I, Bergmann G, Rohlmann A, Schmidt H. 2017. Comparison of in vivo measured loads in knee, ' +
          'hip and spinal implants during level walking. J Biomech 51:128-132.',
        pmid: '27914627',
        doi: '10.1016/j.jbiomech.2016.11.060',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-walking-with-walker',
    feature: 'Walker load comparison',
    publishedNumber:
      'vbr multiple: 0.62 ratio [published; rohlmann-2014-gait]; fixator change: about -25 percent [published; ' +
      'rohlmann-1997-walker]. Separate implant populations; no combination. Table use: beside.',
    population:
      'rohlmann-2014-gait: Adults with instrumented vertebral body replacements; walking/stairs. Sample: 5. ' +
      '[peer-reviewed; abstract]. rohlmann-1997-walker: Adults with telemetered posterior internal fixators; ' +
      'walker use. Sample: 2. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Rohlmann A, Dreischarf M, Zander T, Graichen F, Bergmann G. 2014. Loads on a vertebral body replacement ' +
          'during locomotion measured in vivo. Gait Posture 39(2):750-755.',
        pmid: '24211089',
        doi: '10.1016/j.gaitpost.2013.10.010',
      },
      {
        citation:
          'Rohlmann A, Bergmann G, Graichen F. 1997. Loads on an internal spinal fixation device during walking. J ' +
          'Biomech 30(1):41-47.',
        pmid: '8970923',
        doi: '10.1016/s0021-9290(96)00103-0',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-stand-sit-bend-twist',
    feature: 'Adult telemetry combined activities',
    publishedNumber:
      'fixator lower: 1.11 ratio [published; rohlmann-2002]; fixator upper: 1.2 ratio [published; ' +
      'rohlmann-2002]; vbr threshold exclusive: 1200 N [published; rohlmann-2014-activities]. Fixator ratios ' +
      '111–120 percent; VBRactivities include peaks strictly above 1200 N; threshold is not an upper bound. ' +
      'Table use: beside.',
    population:
      'rohlmann-2002: Adults with telemetered posterior internal fixators; fractures/degenerative disease, not ' +
      'scoliosis. Sample: 10. [peer-reviewed; abstract]. rohlmann-2014-activities: Adults aged62–71, 60–74kg; ' +
      'L1/L3vertebral body replacements, posterior fixator shares load. Sample: 5. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Rohlmann A, Graichen F, Bergmann G. 2002. Loads on an internal spinal fixation device during physical ' +
          'therapy. Phys Ther 82(1):44-52.',
        pmid: '11784277',
        doi: '10.1093/ptj/82.1.44',
      },
      {
        citation:
          'Rohlmann A, Pohl D, Bender A, Graichen F, Dymke J, Schmidt H, Bergmann G. 2014. Activities of everyday ' +
          'life with high spinal loads. PLoS One 9(5):e98510.',
        pmid: '24866883',
        doi: '10.1371/journal.pone.0098510',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-vbr-upper-body-flexion',
    feature: 'VBR flexion absolute forces',
    publishedNumber:
      'lower: about 450 N [published; rohlmann-2015-flexion]; upper: about 565 N [published; ' +
      'rohlmann-2015-flexion]. About450 N at 53 degrees and565 N on return to upright; not a within-posture ' +
      'uncertainty interval. Table use: beside.',
    population:
      'rohlmann-2015-flexion: Adult vertebral body replacement; upper-body flexion and return to upright. ' +
      'Sample: not supplied. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Dreischarf M, Albiol L, Zander T, Arshad R, Graichen F, Bergmann G, Schmidt H, Rohlmann A. 2015. In vivo ' +
          'implant forces acting on a vertebral body replacement during upper body flexion. J Biomech 48(4):560-565.',
        pmid: '25640900',
        doi: '10.1016/j.jbiomech.2015.01.007',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-vbr-lifting-peak',
    feature: 'VBR lifting peak',
    publishedNumber:
      'peak: 1650 N [published; rohlmann-2014-activities]. Peak lifting10.8 kg crate; growth note separately ' +
      'gives1649 N, not substituted. Table use: beside.',
    population:
      'rohlmann-2014-activities: Adults aged62–71, 60–74kg; L1/L3vertebral body replacements, posterior fixator ' +
      'shares load. Sample: 5. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Rohlmann A, Pohl D, Bender A, Graichen F, Dymke J, Schmidt H, Bergmann G. 2014. Activities of everyday ' +
          'life with high spinal loads. PLoS One 9(5):e98510.',
        pmid: '24866883',
        doi: '10.1371/journal.pone.0098510',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-backpack-absolute',
    feature: 'VBR10 kg backpack increment',
    publishedNumber:
      'weight: 10 kg [published; rohlmann-2014-carrying]; force increment: 35 N [published; ' +
      'rohlmann-2014-carrying]. Adult VBR telemetry, not Schmid AIS model ratios. Table use: beside.',
    population:
      'rohlmann-2014-carrying: Adult vertebral body replacement; carrying10 kg backpack or weights. Sample: not ' +
      'supplied. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Rohlmann A, Zander T, Graichen F, Schmidt H, Bergmann G. 2014. How does the way a weight is carried ' +
          'affect spinal loads? Ergonomics 57(2):262-270.',
        pmid: '24559120',
        doi: '10.1080/00140139.2014.887789',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-cycling',
    feature: 'VBR ergometer load increments',
    publishedNumber:
      'power lower: 25 W [published; rohlmann-2014-cycling]; power upper: 85 W [published; ' +
      'rohlmann-2014-cycling]; force increment lower: 73 N [published; rohlmann-2014-cycling]; force increment ' +
      'upper: 225 N [published; rohlmann-2014-cycling]. Increments from cycling power comparison, not total ' +
      'forces. Table use: beside.',
    population:
      'rohlmann-2014-cycling: Adult vertebral body replacement; ergometer cycling. Sample: not supplied. ' +
      '[peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Rohlmann A, Zander T, Graichen F, Schmidt H, Bergmann G. 2014. Spinal loads during cycling on an ' +
          'ergometer. PLoS One 9(4):e95497.',
        pmid: '24743577',
        doi: '10.1371/journal.pone.0095497',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-whole-body-vibration',
    feature: 'VBR vibration comparison',
    publishedNumber:
      'force change: 24 percent [published; rohlmann-2014-vibration]. Relative to same posture; still below ' +
      'walking peaks. Table use: beside.',
    population:
      'rohlmann-2014-vibration: Adult vertebral body replacement; whole-body vibration versus same posture. ' +
      'Sample: not supplied. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Rohlmann A, Schmidt H, Gast U, Kutzner I, Damm P, Bergmann G. 2014. In vivo measurements of the effect of ' +
          'whole body vibration on spinal loads. Eur Spine J 23(3):666-672.',
        pmid: '24201510',
        doi: '10.1007/s00586-013-3087-8',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-daily-loads-orthosis',
    feature: 'VBR orthosis comparison',
    publishedNumber:
      'average force change: -9 percent [published; rohlmann-2013-orthosis]; hyperextension force change: -19 ' +
      'percent [published; rohlmann-2013-orthosis]. Across26 activities; wide variation. Table use: beside.',
    population:
      'rohlmann-2013-orthosis: Adult vertebral body replacement; orthosis comparison over26 activities. Sample: ' +
      'not supplied. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Rohlmann A, Zander T, Graichen F, Bergmann G. 2013. Effect of an orthosis on the loads acting on a ' +
          'vertebral body replacement. Clin Biomech (Bristol) 28(5):490-494.',
        pmid: '23623494',
        doi: '10.1016/j.clinbiomech.2013.03.010',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-fatigue-titanium-alloy',
    feature: 'Construct-derived Goodman-equivalent reference',
    publishedNumber:
      'run out load: 250 N [published; vieweg-2023]; rod diameter: 5.5 mm [published; vieweg-2023]; active ' +
      'length: 76 mm [published; vieweg-2023]; screw diameter: 4.5 mm [published; vieweg-2023]; screw length: 45 ' +
      'mm [published; vieweg-2023]; run out cycles: 5000000 cycles [published; vieweg-2023]; frequency: 5 Hz ' +
      '[published; vieweg-2023]; printed r: 10 ratio [published; vieweg-2023]; assumed arm: 40 mm ' +
      '[own-inference; project-assumptions]; equally sharing rods: 2 rods [own-inference; project-assumptions]; ' +
      'per rod moment: 5.0 N·m [own-inference; project-assumptions]; maximum stress: about 306 MPa ' +
      '[own-inference; project-assumptions]; amplitude: about 138 MPa [own-inference; project-assumptions]; ' +
      'mean: about 168 MPa [own-inference; project-assumptions]; goodman equivalent: about 169 MPa ' +
      '[own-inference; project-assumptions]; goodman uts: about 900 MPa [own-inference; project-assumptions]. ' +
      'Own inference, not material endurance: 40 mm arm unverified; equal load sharing; note interprets printed ' +
      'R = 10 through stress fractions. No recalculation of rounded stress cells. Failure mode: Rod at set ' +
      'screw. Reproduction: moment=F×arm/2; stress=32M/(pi*d^3); Goodman=amplitude/(1-mean/UTS). goodman-uts is ' +
      'the historical assumed UTS used only to reproduce the copied169/191MPa calibration;The current rule now ' +
      'uses paired sampled ultimate strength for each version. Use Evidence::bench_strengths for the runtime ' +
      'pair. Table use: bench.',
    population:
      'vieweg-2023: F1717-21 vertebrectomy constructs, 76 mm active length, 5.5 mm rods, 4.5×45 mm screws. ' +
      'Sample: 6 per dynamic compression-bending group. [peer-reviewed; full-text]. project-assumptions: No ' +
      'measured population; explicit choices and calculations in the project notes. Sample: not supplied. ' +
      '[own-inference; secondary].',
    sources: [
      {
        citation:
          'Vieweg U, Keck J, Krüger S, Arabmotlagh M, Rauschmann M, Schilling C. 2023. Biomechanical comparison of ' +
          'different rod-to-rod connectors to a conventional titanium- and cobalt chromium posterior spinal fixation ' +
          'system. Brain Spine 3:101708.',
        pmid: '36685708',
        doi: '10.1016/j.bas.2022.101708',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-fatigue-cobalt-chrome',
    feature: 'Construct-derived Goodman-equivalent reference',
    publishedNumber:
      'run out load: 290 N [published; vieweg-2023]; rod diameter: 5.5 mm [published; vieweg-2023]; active ' +
      'length: 76 mm [published; vieweg-2023]; screw diameter: 4.5 mm [published; vieweg-2023]; screw length: 45 ' +
      'mm [published; vieweg-2023]; run out cycles: 5000000 cycles [published; vieweg-2023]; frequency: 5 Hz ' +
      '[published; vieweg-2023]; printed r: 10 ratio [published; vieweg-2023]; assumed arm: 40 mm ' +
      '[own-inference; project-assumptions]; equally sharing rods: 2 rods [own-inference; project-assumptions]; ' +
      'per rod moment: 5.8 N·m [own-inference; project-assumptions]; maximum stress: about 355 MPa ' +
      '[own-inference; project-assumptions]; amplitude: about 160 MPa [own-inference; project-assumptions]; ' +
      'mean: about 195 MPa [own-inference; project-assumptions]; goodman equivalent: about 191 MPa ' +
      '[own-inference; project-assumptions]; goodman uts: about 1200 MPa [own-inference; project-assumptions]. ' +
      'Own inference, not material endurance: 40 mm arm unverified; equal load sharing; note interprets printed ' +
      'R = 10 through stress fractions. No recalculation of rounded stress cells. Failure mode: Screw at block. ' +
      'Reproduction: moment=F×arm/2; stress=32M/(pi*d^3); Goodman=amplitude/(1-mean/UTS). goodman-uts is the ' +
      'historical assumed UTS used only to reproduce the copied169/191MPa calibration;The current rule now uses ' +
      'paired sampled ultimate strength for each version. Use Evidence::bench_strengths for the runtime pair. ' +
      'Table use: bench.',
    population:
      'vieweg-2023: F1717-21 vertebrectomy constructs, 76 mm active length, 5.5 mm rods, 4.5×45 mm screws. ' +
      'Sample: 6 per dynamic compression-bending group. [peer-reviewed; full-text]. project-assumptions: No ' +
      'measured population; explicit choices and calculations in the project notes. Sample: not supplied. ' +
      '[own-inference; secondary].',
    sources: [
      {
        citation:
          'Vieweg U, Keck J, Krüger S, Arabmotlagh M, Rauschmann M, Schilling C. 2023. Biomechanical comparison of ' +
          'different rod-to-rod connectors to a conventional titanium- and cobalt chromium posterior spinal fixation ' +
          'system. Brain Spine 3:101708.',
        pmid: '36685708',
        doi: '10.1016/j.bas.2022.101708',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-fatigue-contour-knock-down',
    feature: 'Contouring factor',
    publishedNumber:
      'lower: 0.6 ratio [own-inference; project-assumptions]; upper: 0.8 ratio [own-inference; ' +
      'project-assumptions]; reduction lower: 20 percent [published; slivka-2013]; reduction upper: 40 percent ' +
      '[published; slivka-2013]. Note converts 20–40 percent to 0.6–0.8; published effect Ti/steel 4.5 mm; ' +
      'uniform sampling and applying to 5.5 mm CoCr are own extrapolations. Table use: bench.',
    population:
      'project-assumptions: No measured population; explicit choices and calculations in the project notes. ' +
      'Sample: not supplied. [own-inference; secondary]. slivka-2013: Contoured/re-bent4.5 mm Ti, ' +
      'stainless-steel, CoCr rod constructs; CoCrscrews failed first. Sample: not supplied. [peer-reviewed; ' +
      'abstract].',
    sources: [
      {
        citation: 'Slivka MA, Fan YK, Eck JC. 2013. Spine Deform 1(6):395-400.',
        pmid: '27927364',
        doi: '10.1016/j.jspd.2013.08.004',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-fatigue-cocr-relative-endurance',
    feature: 'CoCr endurance comparison',
    publishedNumber:
      'increase at least: 25 percent [published; slivka-2013]. Could not fix CoCr endurance because screws ' +
      'failed first; not a construct run-out MPa. Table use: beside.',
    population:
      'slivka-2013: Contoured/re-bent4.5 mm Ti, stainless-steel, CoCr rod constructs; CoCrscrews failed first. ' +
      'Sample: not supplied. [peer-reviewed; abstract].',
    sources: [
      {
        citation: 'Slivka MA, Fan YK, Eck JC. 2013. Spine Deform 1(6):395-400.',
        pmid: '27927364',
        doi: '10.1016/j.jspd.2013.08.004',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-fatigue-textbook-titanium',
    feature: 'Textbook fatigue strength via review',
    publishedNumber:
      'lower: 300 MPa [published; ohrt-nissen-2018]; upper: 389 MPa [published; ohrt-nissen-2018]. DISAGREE with ' +
      'coupon/construct values; cycle count absent. Not used as construct level. Table use: beside.',
    population:
      'ohrt-nissen-2018: Review quoting textbook alloy ranges; fatigue cycle count not given. Sample: not ' +
      'supplied. [peer-reviewed; full-text].',
    sources: [
      {
        citation: 'Ohrt-Nissen S, Dahl B, Gehrchen M. 2018. Neurospine 15(2):123-130.',
        pmid: '29991241',
        doi: '10.14245/ns.1836050.025',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-fatigue-textbook-cocr',
    feature: 'Textbook fatigue strength via review',
    publishedNumber:
      'lower: 207 MPa [published; ohrt-nissen-2018]; upper: 950 MPa [published; ohrt-nissen-2018]. DISAGREE with ' +
      'coupon/construct values; cycle count absent. Not used as construct level. Table use: beside.',
    population:
      'ohrt-nissen-2018: Review quoting textbook alloy ranges; fatigue cycle count not given. Sample: not ' +
      'supplied. [peer-reviewed; full-text].',
    sources: [
      {
        citation: 'Ohrt-Nissen S, Dahl B, Gehrchen M. 2018. Neurospine 15(2):123-130.',
        pmid: '29991241',
        doi: '10.14245/ns.1836050.025',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-fatigue-textbook-stainless',
    feature: 'Textbook fatigue strength via review',
    publishedNumber:
      'lower: 241 MPa [published; ohrt-nissen-2018]; upper: 820 MPa [published; ohrt-nissen-2018]. DISAGREE with ' +
      'coupon/construct values; cycle count absent. Not used as construct level. Table use: beside.',
    population:
      'ohrt-nissen-2018: Review quoting textbook alloy ranges; fatigue cycle count not given. Sample: not ' +
      'supplied. [peer-reviewed; full-text].',
    sources: [
      {
        citation: 'Ohrt-Nissen S, Dahl B, Gehrchen M. 2018. Neurospine 15(2):123-130.',
        pmid: '29991241',
        doi: '10.14245/ns.1836050.025',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-fatigue-smooth-ti-coupon',
    feature: 'Smooth polishedTi ELI coupon comparison',
    publishedNumber:
      'strain amplitude: 0.005 ratio [published; carrion-2016]; strain ratio: -1 ratio [published; ' +
      'carrion-2016]; reversals more than: 2713156 reversals [published; carrion-2016]; young modulus lower: 110 ' +
      'GPa [own-inference; project-assumptions]; young modulus upper: 114 GPa [own-inference; ' +
      'project-assumptions]; stress lower: about 550 MPa [own-inference; project-assumptions]; stress upper: ' +
      'about 570 MPa [own-inference; project-assumptions]. DISAGREE: coupon strain run-out converted by note to ' +
      'about 550–570 MPa; not contoured construct strength. Reversal count and unit kept as printed; no ' +
      'conversion to cycles. Table use: beside.',
    population:
      'carrion-2016: Annealed polished wroughtTi-6Al-4VELI coupons, strain-controlled. Sample: 38 specimens. ' +
      '[peer-reviewed; full-text]. project-assumptions: No measured population; explicit choices and ' +
      'calculations in the project notes. Sample: not supplied. [own-inference; secondary].',
    sources: [
      {
        citation:
          'Carrion PE, Shamsaei N. 2016. Strain-based fatigue data for Ti-6Al-4V ELI under fully-reversed and mean ' +
          'strain loads. Data Brief 7:12-15.',
        pmid: '26952022',
        doi: '10.1016/j.dib.2016.02.014',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-fatigue-r-minus-one-strain-0-006',
    feature: 'Smooth Ti-6Al-4V ELI coupon condition',
    publishedNumber:
      'strain amplitude: 0.006 ratio [published; carrion-2016]; strain ratio: -1 ratio [published; ' +
      'carrion-2016]; failure lower: 124952 reversals [published; carrion-2016]; failure upper: 207610 reversals ' +
      '[published; carrion-2016]. Failure range, fully reversed strain control. Table use: beside.',
    population:
      'carrion-2016: Annealed polished wroughtTi-6Al-4VELI coupons, strain-controlled. Sample: 38 specimens. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Carrion PE, Shamsaei N. 2016. Strain-based fatigue data for Ti-6Al-4V ELI under fully-reversed and mean ' +
          'strain loads. Data Brief 7:12-15.',
        pmid: '26952022',
        doi: '10.1016/j.dib.2016.02.014',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-fatigue-r-minus-one-strain-0-004',
    feature: 'Smooth Ti-6Al-4V ELI coupon condition',
    publishedNumber:
      'strain amplitude: 0.004 ratio [published; carrion-2016]; strain ratio: -1 ratio [published; ' +
      'carrion-2016]; run out lower more than: 2.3 million reversals [published; carrion-2016]; run out upper ' +
      'more than: 4.4 million reversals [published; carrion-2016]. Note prints >2.3–4.4 million reversals; two ' +
      'threshold ends, not measured failures. Table use: beside.',
    population:
      'carrion-2016: Annealed polished wroughtTi-6Al-4VELI coupons, strain-controlled. Sample: 38 specimens. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Carrion PE, Shamsaei N. 2016. Strain-based fatigue data for Ti-6Al-4V ELI under fully-reversed and mean ' +
          'strain loads. Data Brief 7:12-15.',
        pmid: '26952022',
        doi: '10.1016/j.dib.2016.02.014',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-fatigue-r-zero-strain-0-004',
    feature: 'Smooth Ti-6Al-4V ELI coupon condition',
    publishedNumber:
      'strain amplitude: 0.004 ratio [published; carrion-2016]; strain ratio: 0 ratio [published; carrion-2016]; ' +
      'reversals more than: 2506668 reversals [published; carrion-2016]. Run-out >2506668 reversals at strain ' +
      'ratio 0. Table use: beside.',
    population:
      'carrion-2016: Annealed polished wroughtTi-6Al-4VELI coupons, strain-controlled. Sample: 38 specimens. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Carrion PE, Shamsaei N. 2016. Strain-based fatigue data for Ti-6Al-4V ELI under fully-reversed and mean ' +
          'strain loads. Data Brief 7:12-15.',
        pmid: '26952022',
        doi: '10.1016/j.dib.2016.02.014',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-fatigue-run-out-definition',
    feature: 'Coupon run-out definition',
    publishedNumber:
      'cycles more than: 1000000 cycles [published; carrion-2016]. Strictly more than 10^6 cycles; no conversion ' +
      'from the reversal counts. Table use: beside.',
    population:
      'carrion-2016: Annealed polished wroughtTi-6Al-4VELI coupons, strain-controlled. Sample: 38 specimens. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Carrion PE, Shamsaei N. 2016. Strain-based fatigue data for Ti-6Al-4V ELI under fully-reversed and mean ' +
          'strain loads. Data Brief 7:12-15.',
        pmid: '26952022',
        doi: '10.1016/j.dib.2016.02.014',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-joint-springs-t1-2-to-l4-5-intact',
    feature: 'Uniform T1/2–L4/5 intact joint',
    publishedNumber:
      'ap: 1392 N/mm [published; tachi-2021]; ml: 294 N/mm [published; tachi-2021]; cc: 341 N/mm [published; ' +
      'tachi-2021]; moment 1: 448 N·mm/deg [published; tachi-2021]; moment 2: 644 N·mm/deg [published; ' +
      'tachi-2021]; moment 3: 738 N·mm/deg [published; tachi-2021]. Moment-1 = AP (x; lateral bending);moment-2 ' +
      '= ML (y; flexion-extension);moment-3 = CC (z; axial rotation). Force columns AP/ML/CC. Tachi borrows ' +
      'stiffness from Argoubi and Shirazi-Adl1996,an adult lumbar source;uniform T1/2–L4/5 application is the ' +
      'planner choice. Confirmed in citation-audit-physics-tables.md §A/§B8 and the current joint-axis ' +
      'convention. Table use: bench.',
    population:
      'tachi-2021: AIS planner comparison against postoperative CT. Sample: 47. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Tachi H, Kato K, Abe Y, Kokabu T, Yamada K, Iwasaki N, Sudo H. 2021. Front Bioeng Biotechnol 9:746902.',
        pmid: '34712654',
        doi: '10.3389/fbioe.2021.746902',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-joint-springs-t1-2-to-l4-5-facetectomy',
    feature: 'Uniform joint after facetectomy',
    publishedNumber:
      'ap: 696 N/mm [published; tachi-2021]; ml: 188 N/mm [published; tachi-2021]; cc: 188 N/mm [published; ' +
      'tachi-2021]; moment 1: 224 N·mm/deg [published; tachi-2021]; moment 2: 406 N·mm/deg [published; ' +
      'tachi-2021]; moment 3: 406 N·mm/deg [published; tachi-2021]. Moment-1 = AP (x; lateral bending);moment-2 ' +
      '= ML (y; flexion-extension);moment-3 = CC (z; axial rotation). Force columns AP/ML/CC. Tachi borrows ' +
      'stiffness from Argoubi and Shirazi-Adl1996,an adult lumbar source;uniform T1/2–L4/5 application is the ' +
      'planner choice. Confirmed in citation-audit-physics-tables.md §A/§B8 and the current joint-axis ' +
      'convention. Table use: beside.',
    population:
      'tachi-2021: AIS planner comparison against postoperative CT. Sample: 47. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Tachi H, Kato K, Abe Y, Kokabu T, Yamada K, Iwasaki N, Sudo H. 2021. Front Bioeng Biotechnol 9:746902.',
        pmid: '34712654',
        doi: '10.3389/fbioe.2021.746902',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-joint-springs-l5-s1',
    feature: 'L5/S1 joint',
    publishedNumber:
      'ap: 700 N/mm [published; tachi-2021]; ml: 190 N/mm [published; tachi-2021]; cc: 190 N/mm [published; ' +
      'tachi-2021]; moment 1: 222 N·mm/deg [published; tachi-2021]; moment 2: 410 N·mm/deg [published; ' +
      'tachi-2021]; moment 3: 410 N·mm/deg [published; tachi-2021]. Moment-1 = AP (x; lateral bending);moment-2 ' +
      '= ML (y; flexion-extension);moment-3 = CC (z; axial rotation). Force columns AP/ML/CC. Tachi borrows ' +
      'stiffness from Argoubi and Shirazi-Adl1996,an adult lumbar source;uniform T1/2–L4/5 application is the ' +
      'planner choice. Confirmed in citation-audit-physics-tables.md §A/§B8 and the current joint-axis ' +
      'convention. Table use: bench.',
    population:
      'tachi-2021: AIS planner comparison against postoperative CT. Sample: 47. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Tachi H, Kato K, Abe Y, Kokabu T, Yamada K, Iwasaki N, Sudo H. 2021. Front Bioeng Biotechnol 9:746902.',
        pmid: '34712654',
        doi: '10.3389/fbioe.2021.746902',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-joint-springs-release',
    feature: 'Facet-capsule/interspinous release',
    publishedNumber:
      'axial rotation: -17 percent [published; wang-2017]; flexion: -15 percent [published; wang-2017]; coronal: ' +
      '-3.8 percent [published; wang-2017]; compression: -14 percent [published; wang-2017]. Signed percentage ' +
      'calibration targets cited by Wang from cadaver studies;not measurements made by Wang. Applied at ' +
      'instrumented joints by project choice. Table use: bench.',
    population:
      'wang-2017: AIS multibody simulations; release terms borrowed from the simulator. Sample: 10 patients; 50 ' +
      'simulations. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Wang X, Larson AN, Crandall DG, Parent S, Labelle H, Ledonio CGT, Aubin CE. 2017. Biomechanical effect of ' +
          'pedicle screw distribution in AIS instrumentation using a segmental translation technique: computer ' +
          'modeling and simulation. Scoliosis Spinal Disord 12:13.',
        pmid: '28428982',
        doi: '10.1186/s13013-017-0120-4',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-joint-springs-preload',
    feature: 'Standing preload multiplier',
    publishedNumber:
      'lower: 1.71 ratio [published; gardner-morse-2004]; upper: 2.11 ratio [published; gardner-morse-2004]; ' +
      'lower preload: 250 N [published; gardner-morse-2004]; upper preload: 500 N [published; ' +
      'gardner-morse-2004]. Published at two discrete preloads; uniform sampling between them is own inference. ' +
      'Table use: bench.',
    population:
      'gardner-morse-2004: Intact human lumbar segments with posterior elements; 250 and 500 N preload. Own ' +
      'abstract passage gives no n. Sample: not supplied. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Gardner-Morse MG, Stokes IA. 2004. Structural behavior of human lumbar spinal motion segments. J Biomech ' +
          '37(2):205-212.',
        pmid: '14706323',
        doi: '10.1016/j.jbiomech.2003.10.003',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-joint-springs-stiffness-scale',
    feature: 'Global log-uniform prior',
    publishedNumber:
      'lower: 1 ratio [own-inference; project-assumptions]; upper: 10 ratio [own-inference; ' +
      'project-assumptions]; lumbar fitted lower: 1.38 ratio [published; petit-2004]; lumbar fitted upper: 10.0 ' +
      'ratio [published; petit-2004]; right thoracic fitted lower: 1.74 ratio [published; petit-2004]; right ' +
      'thoracic fitted upper: 5.18 ratio [published; petit-2004]. Own inference: use Tachi as scale 1; Petit ' +
      'supplies scale 10 endpoint with a different baseline. No identification of absolute stiffness from films. ' +
      'Table use: bench.',
    population:
      'project-assumptions: No measured population; explicit choices and calculations in the project notes. ' +
      'Sample: not supplied. [own-inference; secondary]. petit-2004: Scoliosis bending-film personalisation of ' +
      'per-segment torsion springs. Sample: 10. [peer-reviewed; abstract].',
    sources: [
      {
        citation:
          'Petit Y, Aubin CE, Labelle H. 2004. Patient-specific mechanical properties of a flexible multi-body model ' +
          'of the scoliotic spine. Med Biol Eng Comput 42(1):55-60.',
        pmid: '14977223',
        doi: '10.1007/bf02351011',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-joint-springs-rib-watkins',
    feature: 'Intact rib-cage stability increases',
    publishedNumber:
      'flexion extension: about 40 percent [published; watkins-2005]; lateral bending: about 35 percent ' +
      '[published; watkins-2005]; axial rotation: about 31 percent [published; watkins-2005]. DISAGREE with ' +
      'Mannen/Liebsch; published as stability percentages, kept beside sampled priors. Table use: beside.',
    population:
      'watkins-2005: Cadaveric thoracic spines with and without intact rib cage. Sample: 10. [peer-reviewed; ' +
      'abstract].',
    sources: [
      {
        citation:
          'Watkins R, Watkins R, Williams L, Ahlbrand S, Garcia R, Karamanian A, Sharp L, Vo C, Hedman T. 2005. ' +
          'Stability provided by the sternum and rib cage in the thoracic spine. Spine (Phila Pa 1976) ' +
          '30(11):1283-1286.',
        pmid: '15928553',
        doi: '10.1097/01.brs.0000164257.69354.bb',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-joint-springs-rib-mannen-t1-t4',
    feature: 'Regional elastic-zone stiffness t1-t4',
    publishedNumber:
      'lb with ribs: 6.85 unstated [published; mannen-2018]; lb without ribs: 4.7 unstated [published; ' +
      'mannen-2018]; fe with ribs: 2.29 unstated [published; mannen-2018]; fe without ribs: 2.12 unstated ' +
      '[published; mannen-2018]; ar with ribs: 3.28 unstated [published; mannen-2018]; ar without ribs: 1.35 ' +
      'unstated [published; mannen-2018]. Intact means:n8;no-ribs EZS means:n7,one specimen excluded for loading ' +
      'only to±2N·m. Units not printed in paper and remain unstated. Table use: beside.',
    population:
      'mannen-2018: Whole T1–T12 cadaver spines; 400 N follower load, ±5 Nm dynamic. Sample: 8 intact;7 no-ribs ' +
      'EZS specimens. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Mannen EM, Friis EA, Sis HL, Wong BM, Cadel ES, Anderson DE. 2018. The rib cage stiffens the thoracic ' +
          'spine in a cadaveric model with body weight load under dynamic moments. J Mech Behav Biomed Mater ' +
          '84:258-264.',
        pmid: '29852313',
        doi: '10.1016/j.jmbbm.2018.05.019',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-joint-springs-rib-mannen-t4-t8',
    feature: 'Regional elastic-zone stiffness t4-t8',
    publishedNumber:
      'lb with ribs: 8.3 unstated [published; mannen-2018]; lb without ribs: 4.48 unstated [published; ' +
      'mannen-2018]; fe with ribs: 4.74 unstated [published; mannen-2018]; fe without ribs: 3.36 unstated ' +
      '[published; mannen-2018]; ar with ribs: 4.51 unstated [published; mannen-2018]; ar without ribs: 1.08 ' +
      'unstated [published; mannen-2018]; ar with ribs sd: 6.13 unstated [published; mannen-2018]. Intact ' +
      'means:n8;no-ribs EZS means:n7,one specimen excluded for loading only to±2N·m. Units not printed in paper ' +
      'and remain unstated. Table use: beside.',
    population:
      'mannen-2018: Whole T1–T12 cadaver spines; 400 N follower load, ±5 Nm dynamic. Sample: 8 intact;7 no-ribs ' +
      'EZS specimens. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Mannen EM, Friis EA, Sis HL, Wong BM, Cadel ES, Anderson DE. 2018. The rib cage stiffens the thoracic ' +
          'spine in a cadaveric model with body weight load under dynamic moments. J Mech Behav Biomed Mater ' +
          '84:258-264.',
        pmid: '29852313',
        doi: '10.1016/j.jmbbm.2018.05.019',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-joint-springs-rib-mannen-t8-t12',
    feature: 'Regional elastic-zone stiffness t8-t12',
    publishedNumber:
      'lb with ribs: 2.87 unstated [published; mannen-2018]; lb without ribs: 1.5 unstated [published; ' +
      'mannen-2018]; fe with ribs: 2.97 unstated [published; mannen-2018]; fe without ribs: 1.39 unstated ' +
      '[published; mannen-2018]; ar with ribs: 2.31 unstated [published; mannen-2018]; ar without ribs: 1.02 ' +
      'unstated [published; mannen-2018]. Intact means:n8;no-ribs EZS means:n7,one specimen excluded for loading ' +
      'only to±2N·m. Units not printed in paper and remain unstated. Table use: beside.',
    population:
      'mannen-2018: Whole T1–T12 cadaver spines; 400 N follower load, ±5 Nm dynamic. Sample: 8 intact;7 no-ribs ' +
      'EZS specimens. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Mannen EM, Friis EA, Sis HL, Wong BM, Cadel ES, Anderson DE. 2018. The rib cage stiffens the thoracic ' +
          'spine in a cadaveric model with body weight load under dynamic moments. J Mech Behav Biomed Mater ' +
          '84:258-264.',
        pmid: '29852313',
        doi: '10.1016/j.jmbbm.2018.05.019',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-joint-springs-rib-liebsch',
    feature: 'ROM increase after cutting anterior ribs',
    publishedNumber:
      'axial rotation: 72 percent [published; liebsch-2020]; lateral bending: 11 percent [published; ' +
      'liebsch-2020]; flexion extension: 9 percent [published; liebsch-2020]. ROM increases, not directly ' +
      'transcribed stiffness multipliers. Table use: beside.',
    population:
      'liebsch-2020: Intact polysegmental human thoracic specimens; ±5 Nm; anterior ribs then cut. Sample: 8. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Liebsch C, Wilke HJ. 2020. Rib Presence, Anterior Rib Cage Integrity, and Segmental Length Affect the ' +
          'Stability of the Human Thoracic Spine: An in vitro Study. Front Bioeng Biotechnol 8:46.',
        pmid: '32117927',
        doi: '10.3389/fbioe.2020.00046',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-joint-springs-lumbar-disc-0',
    feature: 'Lumbar disc comparison at 0 N',
    publishedNumber:
      'mx rx: 0.92 Nm/deg [published; stokes-gardner-morse-2016]; mx rx sd: 0.11 Nm/deg [published; ' +
      'stokes-gardner-morse-2016]; mz rz: 0.85 Nm/deg [published; stokes-gardner-morse-2016]; mz rz sd: 0.14 ' +
      'Nm/deg [published; stokes-gardner-morse-2016]; fx dx: 0.06 kN/mm [published; stokes-gardner-morse-2016]; ' +
      'fx dx sd: 0.01 kN/mm [published; stokes-gardner-morse-2016]; fy dy: 0.08 kN/mm [published; ' +
      'stokes-gardner-morse-2016]; fy dy sd: 0.01 kN/mm [published; stokes-gardner-morse-2016]; fz dz: 0.41 ' +
      'kN/mm [published; stokes-gardner-morse-2016]; fz dz sd: 0.08 kN/mm [published; ' +
      'stokes-gardner-morse-2016]. Table3 footnote: x = posterior-anterior shear/lateral bending;y = lateral ' +
      'shear/flexion-extension;z = compression-tension/axial rotation. My/Ry row absent;mapping is defined. Disc ' +
      'specimens lack posterior elements. Preload-series specimen set:L2-3/L4-5,four female donors aged17–58. ' +
      'Table use: beside.',
    population:
      'stokes-gardner-morse-2016: Lumbar discs without posterior elements. Preload series: L2-3/L4-5, four ' +
      'female donors aged17–58. Frequency series: eight segments from seven spines, five male and two female, ' +
      'mean age41;642N column measured at0.01Hz. Sample: 8 per series. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Stokes IAF, Gardner-Morse M. 2016. A database of lumbar spinal mechanical behavior for validation of ' +
          'spinal analytical models. J Biomech 49(5):780-785.',
        pmid: '26900035',
        doi: '10.1016/j.jbiomech.2016.01.035',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-joint-springs-lumbar-disc-250',
    feature: 'Lumbar disc comparison at 250 N',
    publishedNumber:
      'mx rx: 1.9 Nm/deg [published; stokes-gardner-morse-2016]; mx rx sd: 0.6 Nm/deg [published; ' +
      'stokes-gardner-morse-2016]; mz rz: 1.7 Nm/deg [published; stokes-gardner-morse-2016]; mz rz sd: 0.2 ' +
      'Nm/deg [published; stokes-gardner-morse-2016]; fx dx: 0.11 kN/mm [published; stokes-gardner-morse-2016]; ' +
      'fx dx sd: 0.01 kN/mm [published; stokes-gardner-morse-2016]; fy dy: 0.14 kN/mm [published; ' +
      'stokes-gardner-morse-2016]; fy dy sd: 0.02 kN/mm [published; stokes-gardner-morse-2016]; fz dz: 1.7 kN/mm ' +
      '[published; stokes-gardner-morse-2016]; fz dz sd: 0.1 kN/mm [published; stokes-gardner-morse-2016]. ' +
      'Table3 footnote: x = posterior-anterior shear/lateral bending;y = lateral shear/flexion-extension;z = ' +
      'compression-tension/axial rotation. My/Ry row absent;mapping is defined. Disc specimens lack posterior ' +
      'elements. Preload-series specimen set:L2-3/L4-5,four female donors aged17–58. Table use: beside.',
    population:
      'stokes-gardner-morse-2016: Lumbar discs without posterior elements. Preload series: L2-3/L4-5, four ' +
      'female donors aged17–58. Frequency series: eight segments from seven spines, five male and two female, ' +
      'mean age41;642N column measured at0.01Hz. Sample: 8 per series. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Stokes IAF, Gardner-Morse M. 2016. A database of lumbar spinal mechanical behavior for validation of ' +
          'spinal analytical models. J Biomech 49(5):780-785.',
        pmid: '26900035',
        doi: '10.1016/j.jbiomech.2016.01.035',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-joint-springs-lumbar-disc-500',
    feature: 'Lumbar disc comparison at 500 N',
    publishedNumber:
      'mx rx: 2.7 Nm/deg [published; stokes-gardner-morse-2016]; mx rx sd: 0.8 Nm/deg [published; ' +
      'stokes-gardner-morse-2016]; mz rz: 2.2 Nm/deg [published; stokes-gardner-morse-2016]; mz rz sd: 0.2 ' +
      'Nm/deg [published; stokes-gardner-morse-2016]; fx dx: 0.15 kN/mm [published; stokes-gardner-morse-2016]; ' +
      'fx dx sd: 0.02 kN/mm [published; stokes-gardner-morse-2016]; fy dy: 0.19 kN/mm [published; ' +
      'stokes-gardner-morse-2016]; fy dy sd: 0.02 kN/mm [published; stokes-gardner-morse-2016]; fz dz: 2.5 kN/mm ' +
      '[published; stokes-gardner-morse-2016]; fz dz sd: 0.2 kN/mm [published; stokes-gardner-morse-2016]. ' +
      'Table3 footnote: x = posterior-anterior shear/lateral bending;y = lateral shear/flexion-extension;z = ' +
      'compression-tension/axial rotation. My/Ry row absent;mapping is defined. Disc specimens lack posterior ' +
      'elements. Preload-series specimen set:L2-3/L4-5,four female donors aged17–58. Table use: beside.',
    population:
      'stokes-gardner-morse-2016: Lumbar discs without posterior elements. Preload series: L2-3/L4-5, four ' +
      'female donors aged17–58. Frequency series: eight segments from seven spines, five male and two female, ' +
      'mean age41;642N column measured at0.01Hz. Sample: 8 per series. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Stokes IAF, Gardner-Morse M. 2016. A database of lumbar spinal mechanical behavior for validation of ' +
          'spinal analytical models. J Biomech 49(5):780-785.',
        pmid: '26900035',
        doi: '10.1016/j.jbiomech.2016.01.035',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-joint-springs-lumbar-disc-642',
    feature: 'Lumbar disc comparison at 642 N',
    publishedNumber:
      'mx rx: 2.8 Nm/deg [published; stokes-gardner-morse-2016]; mx rx sd: 2.2 Nm/deg [published; ' +
      'stokes-gardner-morse-2016]; mz rz: 2.4 Nm/deg [published; stokes-gardner-morse-2016]; mz rz sd: 1.0 ' +
      'Nm/deg [published; stokes-gardner-morse-2016]; fx dx: 0.17 kN/mm [published; stokes-gardner-morse-2016]; ' +
      'fx dx sd: 0.07 kN/mm [published; stokes-gardner-morse-2016]; fy dy: 0.21 kN/mm [published; ' +
      'stokes-gardner-morse-2016]; fy dy sd: 0.07 kN/mm [published; stokes-gardner-morse-2016]; fz dz: 3.3 kN/mm ' +
      '[published; stokes-gardner-morse-2016]; fz dz sd: 0.7 kN/mm [published; stokes-gardner-morse-2016]; test ' +
      'frequency: 0.01 Hz [published; stokes-gardner-morse-2016]; preload sd: 132 N [published; ' +
      'stokes-gardner-morse-2016]. Table3 footnote: x = posterior-anterior shear/lateral bending;y = lateral ' +
      'shear/flexion-extension;z = compression-tension/axial rotation. My/Ry row absent;mapping is defined. Disc ' +
      'specimens lack posterior elements. Different frequency-series specimen set:eight segments from seven ' +
      'spines,five male/two female,mean age41,tested at0.01Hz;mean preload642±132N. Table use: beside.',
    population:
      'stokes-gardner-morse-2016: Lumbar discs without posterior elements. Preload series: L2-3/L4-5, four ' +
      'female donors aged17–58. Frequency series: eight segments from seven spines, five male and two female, ' +
      'mean age41;642N column measured at0.01Hz. Sample: 8 per series. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Stokes IAF, Gardner-Morse M. 2016. A database of lumbar spinal mechanical behavior for validation of ' +
          'spinal analytical models. J Biomech 49(5):780-785.',
        pmid: '26900035',
        doi: '10.1016/j.jbiomech.2016.01.035',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-joint-springs-thoracic-rom',
    feature: 'Wilke ROM inputs for secants',
    publishedNumber:
      't1 t2 flexion: 7.1 deg [published; wilke-2017]; t1 t2 extension: 6.7 deg [published; wilke-2017]; t1 t2 ' +
      'lb: 5.9 deg [published; wilke-2017]; t1 t2 ar: 6.2 deg [published; wilke-2017]; t2 t7 lb lower: 4.9 deg ' +
      '[published; wilke-2017]; t2 t7 lb upper: 5.6 deg [published; wilke-2017]; t2 t6 ar lower: 5.2 deg ' +
      '[published; wilke-2017]; t2 t6 ar upper: 5.7 deg [published; wilke-2017]; t6 t7 ar: 5.9 deg [published; ' +
      'wilke-2017]; t7 t8 lb: 3.7 deg [published; wilke-2017]; t8 t12 lb lower: 3.8 deg [published; wilke-2017]; ' +
      't8 t12 lb upper: 4.5 deg [published; wilke-2017]; t11 t12 ar: 3.3 deg [published; wilke-2017]; t7 t8 ar: ' +
      'about 5 deg [published; wilke-2017]; t8 t12 ar upper: about 5 deg [published; wilke-2017]; applied ' +
      'moment: 7.5 N·m [published; wilke-2017]; t2 t7 flexion extension lower: 3.1 deg [published; wilke-2017]; ' +
      't2 t7 flexion extension upper: 3.8 deg [published; wilke-2017]; t2 t3 flexion extension: 4.1 deg ' +
      '[published; wilke-2017]; t7 t8 flexion: 2.8 deg [published; wilke-2017]; t7 t8 extension: 2.8 deg ' +
      '[published; wilke-2017]; t8 t12 flexion extension lower: 3.1 deg [published; wilke-2017]; t8 t12 flexion ' +
      'extension upper: 3.8 deg [published; wilke-2017]. At±7.5N·m,the AR range5.2–5.7degrees covers T2-T3 ' +
      'through T5-T6 (T2–T6);T6-T7 separately5.9. LB4.9–5.6covers T2-T3 through T6-T7. Values include neutral ' +
      'zone. Table use: input.',
    population:
      'wilke-2017: Human cadaver thoracic functional units with costovertebral joints kept; ±7.5 Nm. Sample: 68 ' +
      'functional units from29 donors;6–7 per level. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Wilke HJ, Herkommer A, Werner K, Liebsch C. 2017. In vitro analysis of the segmental flexibility of the ' +
          'thoracic spine. PLoS One 12(5):e0177823.',
        pmid: '28520819',
        doi: '10.1371/journal.pone.0177823',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-rod-materials-titanium-alloy',
    feature: 'Wrought Ti-6Al-4V',
    publishedNumber:
      'young modulus: 114 GPa [published; prasad-2017]; yield lower: 827 MPa [published; prasad-2017]; yield ' +
      'upper: 1103 MPa [published; prasad-2017]; uts lower: 860 MPa [published; prasad-2017]; uts upper: 965 MPa ' +
      '[published; prasad-2017]. Prasad Table1 set. Bench uses E114GPa only from this row;strength pair retained ' +
      'beside the bench. Printed yield upper1103MPa exceeds tensile upper965MPa;do not sample these strengths. ' +
      'No Poisson ratio/shear modulus supplied for this Prasad set. Table use: beside.',
    population:
      'prasad-2017: Review of implant alloys; wrought Ti-6Al-4V,CoCrMo and Stainless steel316L;Table1 states no ' +
      'steel condition. Sample: not supplied. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Prasad K, Bazaka O, Chua M, Rochford M, Fedrick L, Spoor J, Symes R, Tieppo M, Collins C, Cao A, Markwell ' +
          'D, Ostrikov KK, Bazaka K. 2017. Metallic Biomaterials: Current Challenges and Opportunities. Materials ' +
          '(Basel) 10(8):E884.',
        pmid: '28773240',
        doi: '10.3390/ma10080884',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-rod-materials-cobalt-chrome',
    feature: 'CoCrMo',
    publishedNumber:
      'young modulus: 240 GPa [published; prasad-2017]; yield lower: 500 MPa [published; prasad-2017]; yield ' +
      'upper: 1500 MPa [published; prasad-2017]; uts lower: 900 MPa [published; prasad-2017]; uts upper: 1540 ' +
      'MPa [published; prasad-2017]. Prasad Table1 set. Bench uses this Table1 yield/ultimate pair at one common ' +
      'quantile,as the current rod-strength rule states. No Poisson ratio/shear modulus supplied for this Prasad ' +
      'set. Table use: bench.',
    population:
      'prasad-2017: Review of implant alloys; wrought Ti-6Al-4V,CoCrMo and Stainless steel316L;Table1 states no ' +
      'steel condition. Sample: not supplied. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Prasad K, Bazaka O, Chua M, Rochford M, Fedrick L, Spoor J, Symes R, Tieppo M, Collins C, Cao A, Markwell ' +
          'D, Ostrikov KK, Bazaka K. 2017. Metallic Biomaterials: Current Challenges and Opportunities. Materials ' +
          '(Basel) 10(8):E884.',
        pmid: '28773240',
        doi: '10.3390/ma10080884',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-rod-materials-stainless-steel',
    feature: 'Stainless steel316L (Table1;condition unspecified)',
    publishedNumber:
      'young modulus: 193 GPa [published; prasad-2017]; yield lower: 170 MPa [published; prasad-2017]; yield ' +
      'upper: 310 MPa [published; prasad-2017]; uts lower: 540 MPa [published; prasad-2017]; uts upper: 1000 MPa ' +
      '[published; prasad-2017]. Prasad Table1 set. Bench uses this Table1 yield/ultimate pair at one common ' +
      'quantile,as the current rod-strength rule states. No Poisson ratio/shear modulus supplied for this Prasad ' +
      'set. Table use: bench.',
    population:
      'prasad-2017: Review of implant alloys; wrought Ti-6Al-4V,CoCrMo and Stainless steel316L;Table1 states no ' +
      'steel condition. Sample: not supplied. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Prasad K, Bazaka O, Chua M, Rochford M, Fedrick L, Spoor J, Symes R, Tieppo M, Collins C, Cao A, Markwell ' +
          'D, Ostrikov KK, Bazaka K. 2017. Metallic Biomaterials: Current Challenges and Opportunities. Materials ' +
          '(Basel) 10(8):E884.',
        pmid: '28773240',
        doi: '10.3390/ma10080884',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-rod-materials-prasad-body-text',
    feature: 'Alternative body-text values',
    publishedNumber:
      'ti young modulus: 115 GPa [published; prasad-2017]; cocr yield lower: 448 MPa [published; prasad-2017]; ' +
      'cocr yield upper: 1606 MPa [published; prasad-2017]; ti yield lower: 896 MPa [published; prasad-2017]; ti ' +
      'yield upper: 1034 MPa [published; prasad-2017]; ti uts lower: 965 MPa [published; prasad-2017]; ti uts ' +
      'upper: 1103 MPa [published; prasad-2017]; cocr uts lower: 655 MPa [published; prasad-2017]; cocr uts ' +
      'upper: 1896 MPa [published; prasad-2017]. DISAGREE with Table1. Bench selects only the titanium strength ' +
      'pair,through titanium-alloy-body-text. CoCr uses Table1 strengths;Ti modulus stays114GPa from Table1. ' +
      'Table use: beside.',
    population:
      'prasad-2017: Review of implant alloys; wrought Ti-6Al-4V,CoCrMo and Stainless steel316L;Table1 states no ' +
      'steel condition. Sample: not supplied. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Prasad K, Bazaka O, Chua M, Rochford M, Fedrick L, Spoor J, Symes R, Tieppo M, Collins C, Cao A, Markwell ' +
          'D, Ostrikov KK, Bazaka K. 2017. Metallic Biomaterials: Current Challenges and Opportunities. Materials ' +
          '(Basel) 10(8):E884.',
        pmid: '28773240',
        doi: '10.3390/ma10080884',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-rod-materials-tachi-cocr',
    feature: 'CoCr modulus printed in planner',
    publishedNumber:
      'young modulus: 420 GPa [published; tachi-2021]; poisson ratio: 0.3 ratio [published; tachi-2021]. ' +
      'DISAGREE; probable error or deliberate stiffening; bench uses Prasad 240 GPa. Audit confirms Poisson ' +
      'ratio0.3 for this separate material set,not for Prasad. Table use: beside.',
    population:
      'tachi-2021: AIS planner comparison against postoperative CT. Sample: 47. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Tachi H, Kato K, Abe Y, Kokabu T, Yamada K, Iwasaki N, Sudo H. 2021. Front Bioeng Biotechnol 9:746902.',
        pmid: '34712654',
        doi: '10.3389/fbioe.2021.746902',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-rod-materials-little-ti',
    feature: 'Ti material set in FE planner',
    publishedNumber:
      'young modulus: 108 GPa [published; little-2013]; yield: 390 MPa [published; little-2013]; poisson ratio: ' +
      '0.3 ratio [published; little-2013]. DISAGREE; as printed; bench uses Prasad set. Audit confirms Poisson ' +
      'ratio0.3 for this separate material set,not for Prasad. Table use: beside.',
    population:
      'little-2013: AIS FE comparison with1-week standing Cobb; adult material properties. Sample: 8. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Little JP, Izatt MT, Labrom RD, Askin GN, Adam CJ. 2013. An FE investigation simulating intra-operative ' +
          'corrective forces applied to correct scoliosis deformity. Scoliosis 8(1):9.',
        pmid: '23680391',
        doi: '10.1186/1748-7161-8-9',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-rod-materials-jeong-ti-eli',
    feature: 'Ti-6Al-4V ELI F1717 FE set',
    publishedNumber:
      'young modulus: 108 GPa [published; jeong-2025]; poisson ratio: 0.342 ratio [published; jeong-2025]; ' +
      'yield: 864 MPa [published; jeong-2025]. Separate set; do not silently attach its Poisson ratio to Prasad ' +
      'E = 114. Table use: beside.',
    population:
      'jeong-2025: FE of ASTM F1717 construct; Ti-6Al-4V ELI; cyclic behaviour not addressed. Sample: not ' +
      'supplied. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Jeong BC, Sim O, Lee C. 2025. Simulation and validation of spinal construct testing based on ASTM F1717. ' +
          'Front Bioeng Biotechnol 13:1673061.',
        pmid: '41383715',
        doi: '10.3389/fbioe.2025.1673061',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-rod-materials-song-ti',
    feature: 'Ti pedicle FE set',
    publishedNumber:
      'young modulus: 113 GPa [published; song-2021]; poisson ratio: 0.3 ratio [published; song-2021]. Separate ' +
      'set; not a Poisson ratio for Prasad material set. Table use: beside.',
    population:
      'song-2021: L3–S1 FE models; Ti alloy; 300 N preload plus10 Nm moment. Sample: not supplied. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Song M, Sun K, Li Z, Zong J, Tian X, Ma K, Wang S. 2021. Stress distribution of different lumbar ' +
          'posterior pedicle screw insertion techniques: a combination study of finite element analysis and ' +
          'biomechanical test. Sci Rep 11(1):12968.',
        pmid: '34155224',
        doi: '10.1038/s41598-021-90686-6',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-rod-materials-textbook-cocr',
    feature: 'Textbook modulus via review',
    publishedNumber:
      'young modulus lower: 200 GPa [published; ohrt-nissen-2018]; young modulus upper: 300 GPa [published; ' +
      'ohrt-nissen-2018]. DISAGREE; loose textbook bounds cited by full-text review; bench uses Prasad. Table ' +
      'use: beside.',
    population:
      'ohrt-nissen-2018: Review quoting textbook alloy ranges; fatigue cycle count not given. Sample: not ' +
      'supplied. [peer-reviewed; full-text].',
    sources: [
      {
        citation: 'Ohrt-Nissen S, Dahl B, Gehrchen M. 2018. Neurospine 15(2):123-130.',
        pmid: '29991241',
        doi: '10.14245/ns.1836050.025',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-rod-materials-textbook-ti',
    feature: 'Textbook modulus via review',
    publishedNumber:
      'young modulus lower: 110 GPa [published; ohrt-nissen-2018]; young modulus upper: 116 GPa [published; ' +
      'ohrt-nissen-2018]. DISAGREE; loose textbook bounds cited by full-text review; bench uses Prasad. Table ' +
      'use: beside.',
    population:
      'ohrt-nissen-2018: Review quoting textbook alloy ranges; fatigue cycle count not given. Sample: not ' +
      'supplied. [peer-reviewed; full-text].',
    sources: [
      {
        citation: 'Ohrt-Nissen S, Dahl B, Gehrchen M. 2018. Neurospine 15(2):123-130.',
        pmid: '29991241',
        doi: '10.14245/ns.1836050.025',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-rod-materials-textbook-steel',
    feature: 'Textbook modulus via review',
    publishedNumber:
      'young modulus: 190 GPa [published; ohrt-nissen-2018]. DISAGREE; loose textbook bounds cited by full-text ' +
      'review; bench uses Prasad. Table use: beside.',
    population:
      'ohrt-nissen-2018: Review quoting textbook alloy ranges; fatigue cycle count not given. Sample: not ' +
      'supplied. [peer-reviewed; full-text].',
    sources: [
      {
        citation: 'Ohrt-Nissen S, Dahl B, Gehrchen M. 2018. Neurospine 15(2):123-130.',
        pmid: '29991241',
        doi: '10.14245/ns.1836050.025',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-rod-materials-ti6al7nb',
    feature: 'Ti6Al7Nb rod as modelled',
    publishedNumber:
      'young modulus: 105 GPa [published; abe-2015]; hardening modulus: 2.41 GPa [published; abe-2015]; yield: ' +
      '900 MPa [published; abe-2015]; yield strain: 0.00857 ratio [published; abe-2015]. Different alloy and ' +
      'model set from wrought Ti-6Al-4V; no UTS supplied. Table use: beside.',
    population:
      'abe-2015: AIS patients, mean age14.8; 6 mm Ti6Al7Nb rods, elasto-plastic inverse FE. Sample: 20. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Abe Y, Ito M, Abumi K, Sudo H, Salmingo R, Tadano S. 2015. Scoliosis corrective force estimation from the ' +
          'implanted rod deformation using 3D-FEM analysis. Scoliosis 10(Suppl 2):S2.',
        pmid: '25810754',
        doi: '10.1186/1748-7161-10-s2-s2',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-rod-materials-ultra-high-strength-steel',
    feature: 'Ultra-high-strength steel comparison',
    publishedNumber:
      'uts: 200 ksi [published; cidambi-2012]; approximate uts: about 1380 MPa [own-inference; ' +
      'project-assumptions]. Copy both the published ksi and the note’s approximate MPa; reader does not convert ' +
      'them. Table use: beside.',
    population:
      'cidambi-2012: Patients with 5.5 mm ultra-high-strength steel rods; disease label not restated in the ' +
      'supplying table. Sample: 27. [peer-reviewed; abstract]. project-assumptions: No measured population; ' +
      'explicit choices and calculations in the project notes. Sample: not supplied. [own-inference; secondary].',
    sources: [
      {
        citation:
          'Cidambi KR, Glaser DA, Bastrom TP, Nunn TN, Ono T, Newton PO. 2012. Postoperative changes in spinal rod ' +
          'contour in adolescent idiopathic scoliosis: an in vivo deformation study. Spine (Phila Pa 1976) ' +
          '37(18):1566-1572.',
        pmid: '22426445',
        doi: '10.1097/brs.0b013e318252ccbe',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-rod-materials-section-table-reference',
    feature: 'Reference inputs for computed section Table A',
    publishedNumber:
      'titanium yield: 900 MPa [own-inference; project-assumptions]; young modulus titanium: 114 GPa [published; ' +
      'prasad-2017]; young modulus cobalt chrome: 240 GPa [published; prasad-2017]; young modulus stainless ' +
      'steel: 193 GPa [published; prasad-2017]. Inputs beside copied own-inference outputs; I=pi*d^4/64; ' +
      'Z=pi*d^3/32; EI=E*I; My=yield*Z. No new calculation. Table use: input.',
    population:
      'project-assumptions: No measured population; explicit choices and calculations in the project notes. ' +
      'Sample: not supplied. [own-inference; secondary]. prasad-2017: Review of implant alloys; wrought ' +
      'Ti-6Al-4V,CoCrMo and Stainless steel316L;Table1 states no steel condition. Sample: not supplied. ' +
      '[peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Prasad K, Bazaka O, Chua M, Rochford M, Fedrick L, Spoor J, Symes R, Tieppo M, Collins C, Cao A, Markwell ' +
          'D, Ostrikov KK, Bazaka K. 2017. Metallic Biomaterials: Current Challenges and Opportunities. Materials ' +
          '(Basel) 10(8):E884.',
        pmid: '28773240',
        doi: '10.3390/ma10080884',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-bench-rod-materials-titanium-alloy-body-text',
    feature: 'Chosen titanium yield/ultimate pair from Prasad body text',
    publishedNumber:
      'yield lower: 896 MPa [published; prasad-2017]; yield upper: 1034 MPa [published; prasad-2017]; uts lower: ' +
      '965 MPa [published; prasad-2017]; uts upper: 1103 MPa [published; prasad-2017]. the current rod-strength ' +
      'rule:use one common quantile for yield896–1034MPa and ultimate965–1103MPa. Young modulus remains114GPa ' +
      'from Table1;do not replace it by body-text115GPa implicitly. Table use: bench.',
    population:
      'prasad-2017: Review of implant alloys; wrought Ti-6Al-4V,CoCrMo and Stainless steel316L;Table1 states no ' +
      'steel condition. Sample: not supplied. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Prasad K, Bazaka O, Chua M, Rochford M, Fedrick L, Spoor J, Symes R, Tieppo M, Collins C, Cao A, Markwell ' +
          'D, Ostrikov KK, Bazaka K. 2017. Metallic Biomaterials: Current Challenges and Opportunities. Materials ' +
          '(Basel) 10(8):E884.',
        pmid: '28773240',
        doi: '10.3390/ma10080884',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-readings-curve-floor',
    feature: 'Naming a curve in the displayed readings',
    publishedNumber:
      'Cobb magnitude at least 10 degrees. This is an angular display floor; the placement rules retain smaller ' +
      'curves. The source also requires recognisable axial rotation in its definition; the display uses the ' +
      'angular part alone.',
    population: 'sosort-2016: Idiopathic scoliosis during growth. Sample: not supplied. [guideline; full-text].',
    sources: [
      {
        citation:
          'Negrini S, Donzelli S, Aulisa AG, Czaprowski D, Schreiber S, de Mauroy JC, Diers H, Grivas TB, Knott P, ' +
          'Kotwicki T, et al. 2018. Scoliosis Spinal Disord. 13:3.',
        pmid: '29435499',
        doi: '10.1186/s13013-017-0145-8',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-rule-end-plus-one',
    feature: 'Adding one level below the lower end',
    publishedNumber: 'Lower end vertebra plus 1 level; a geometric level rule, distinct from a measured distance.',
    population:
      'liu-2020: Lenke 1A/2A thoracic-only fusions. Sample: 6 studies; 732 patients; 150 with adding-on; ' +
      'per-reference denominators retained in the table. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Liu CW, Lenke LG, Tan LA, Oh T, Chao KH, Lin SD, Pan RY. 2020. Selection of the Lowest Instrumented ' +
          'Vertebra and Relative Odds Ratio of Distal Adding-on for Lenke Type 1A and 2A Curves in Adolescent ' +
          'Idiopathic Scoliosis: A Systematic Review and Meta-analysis. Neurospine 17(4):902-909.',
        pmid: '33401869',
        doi: '10.14245/ns.2040234.117',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-rule-suk-neutral',
    feature: 'Finding the neutral vertebra for the lower-end rule',
    publishedNumber:
      'Published neutral-to-end offsets are recorded in the threshold entry. Own geometric reading: neutral is ' +
      'the nearest measured-twist zero crossing; the nearer-zero member is used, with a caudal tie. A typical ' +
      'twist never supplies this reading. No degree-to-rotation-grade conversion is supplied.',
    population:
      'liu-2020: Lenke 1A/2A thoracic-only fusions. Sample: 6 studies; 732 patients; 150 with adding-on; ' +
      'per-reference denominators retained in the table. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Liu CW, Lenke LG, Tan LA, Oh T, Chao KH, Lin SD, Pan RY. 2020. Selection of the Lowest Instrumented ' +
          'Vertebra and Relative Odds Ratio of Distal Adding-on for Lenke Type 1A and 2A Curves in Adolescent ' +
          'Idiopathic Scoliosis: A Systematic Review and Meta-analysis. Neurospine 17(4):902-909.',
        pmid: '33401869',
        doi: '10.14245/ns.2040234.117',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-rule-stable-minus-one',
    feature: 'The stable vertebra and lumbar modifier',
    publishedNumber:
      'Use 1 level above the stable vertebra for modifier A or B, and the stable vertebra for C. Pedicle-centre ' +
      'bracketing is a project reading of bisected; missing scale or a missing pedicle table leaves the modifier ' +
      'unreadable.',
    population:
      'bmc-level-review: AIS level-selection review; primary populations differ by rule. Sample: Review; ' +
      'aggregate n not supplied. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Chen R, Xi Y, Wang T, Wang A, Ma Z, Liang M, Yuan S, Zang L, Fan N. 2026. Advances in fusion level ' +
          'selection and surgical approaches for adolescent idiopathic scoliosis based on the Lenke classification ' +
          'system: a narrative review. BMC Surg 26(1):102.',
        pmid: '41492130',
        doi: '10.1186/s12893-025-03481-9',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-rule-lenke-l3-l4',
    feature: 'Choosing between L3 and L4',
    publishedNumber:
      'Published criteria name the L1/L2 apex boundary, the L3/L4 disc opening and a rotation grade. There is no ' +
      'supplied conversion from twist degrees to that grade, so the complete rule is unreadable. Numerical ' +
      'criteria remain in their separate threshold entries.',
    population:
      'bmc-level-review: AIS level-selection review; primary populations differ by rule. Sample: Review; ' +
      'aggregate n not supplied. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Chen R, Xi Y, Wang T, Wang A, Ma Z, Liang M, Yuan S, Zang L, Fan N. 2026. Advances in fusion level ' +
          'selection and surgical approaches for adolescent idiopathic scoliosis based on the Lenke classification ' +
          'system: a narrative review. BMC Surg 26(1):102.',
        pmid: '41492130',
        doi: '10.1186/s12893-025-03481-9',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-rule-last-touched',
    feature: 'The last body touched by the centre line',
    publishedNumber:
      'First body touched while proceeding down through T12 to L5; contact is read against the body silhouette. ' +
      'No extra millimetre cutoff is supplied.',
    population:
      'liu-2020: Lenke 1A/2A thoracic-only fusions. Sample: 6 studies; 732 patients; 150 with adding-on; ' +
      'per-reference denominators retained in the table. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Liu CW, Lenke LG, Tan LA, Oh T, Chao KH, Lin SD, Pan RY. 2020. Selection of the Lowest Instrumented ' +
          'Vertebra and Relative Odds Ratio of Distal Adding-on for Lenke Type 1A and 2A Curves in Adolescent ' +
          'Idiopathic Scoliosis: A Systematic Review and Meta-analysis. Neurospine 17(4):902-909.',
        pmid: '33401869',
        doi: '10.14245/ns.2040234.117',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-rule-substantially-touched',
    feature: 'Pedicle contact by the centre line',
    publishedNumber:
      'Use the substantially touched vertebra, or 1 level below the last touched vertebra when contact is not ' +
      'substantial. Table pedicle outlines are an assumption; no scale means the millimetre test cannot be read.',
    population:
      'liu-2020: Lenke 1A/2A thoracic-only fusions. Sample: 6 studies; 732 patients; 150 with adding-on; ' +
      'per-reference denominators retained in the table. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Liu CW, Lenke LG, Tan LA, Oh T, Chao KH, Lin SD, Pan RY. 2020. Selection of the Lowest Instrumented ' +
          'Vertebra and Relative Odds Ratio of Distal Adding-on for Lenke Type 1A and 2A Curves in Adolescent ' +
          'Idiopathic Scoliosis: A Systematic Review and Meta-analysis. Neurospine 17(4):902-909.',
        pmid: '33401869',
        doi: '10.14245/ns.2040234.117',
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-rule-upper-end-vertebra',
    feature: 'Upper end of the included curves',
    publishedNumber:
      'The upper end of the highest included curve. Minor-curve structural thresholds are recorded separately; ' +
      'an absent film never counts as a met test.',
    population:
      'lenke-2001: AIS classification using standing coronal/lateral and supine side-bending films. Sample: ' +
      '27-patient radiographic reliability set; 5 developers and 7 independent surgeons. [peer-reviewed; ' +
      'abstract].',
    sources: [
      {
        citation:
          'Lenke LG, Betz RR, Harms J, Bridwell KH, Clements DH, Lowe TG, Blanke K. 2001. Adolescent idiopathic ' +
          'scoliosis: a new classification to determine extent of spinal arthrodesis. J Bone Joint Surg Am ' +
          '83(8):1169-1181.',
        pmid: '11507125',
        doi: null,
      },
    ] as readonly EvidenceSource[],
    checked: false,
    status: 'experimental',
  },
  {
    id: 'place-rule-lenke-shoulder',
    feature: 'Shoulder rule awaiting a shoulder reading',
    publishedNumber:
      'No shoulder height is measured by this spine model. The rule remains unreadable; T1 tilt is not ' +
      'substituted for shoulder height.',
    population:
      'bmc-level-review: AIS level-selection review; primary populations differ by rule. Sample: Review; ' +
      'aggregate n not supplied. [peer-reviewed; full-text].',
    sources: [
      {
        citation:
          'Chen R, Xi Y, Wang T, Wang A, Ma Z, Liang M, Yuan S, Zang L, Fan N. 2026. Advances in fusion level ' +
          'selection and surgical approaches for adolescent idiopathic scoliosis based on the Lenke classification ' +
          'system: a narrative review. BMC Surg 26(1):102.',
        pmid: '41492130',
        doi: '10.1186/s12893-025-03481-9',
      },
    ] as readonly EvidenceSource[],
    checked: false,
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
