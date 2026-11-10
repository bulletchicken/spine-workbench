// The evidence ledger: for each feature, the published number behind it,
// where it was published, and whether that evidence supports the feature
// or the feature is experimental. Features the evidence does not yet
// support carry the experimental badge wherever they are shown.

/** A published source, cited by its identifiers. No links: the app never goes online. */
export interface EvidenceSource {
  /** Authors, year, title and journal, as written in a reference list. */
  citation: string;
  /** PubMed id, digits only. */
  pmid: string | null;
  doi: string | null;
}

export interface EvidenceEntry {
  /** Stable kebab-case id the app refers to, such as `two-film-twist`. */
  id: string;
  /** What the app does, in the user's words. */
  feature: string;
  /** The number as published, with its unit and spread; `null` when no number was published. */
  publishedNumber: string | null;
  sources: EvidenceSource[];
  /** `supported`: the published evidence covers the feature as built. Anything less is `experimental`. */
  status: 'supported' | 'experimental';
}
