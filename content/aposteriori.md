---
title: A Posteriori Soundness for Non-deterministic Abstract Interpretations
date: 2026-02-16
type: paper
bibtex: |
  @InProceedings{aposteriori,
  author="Might, Matthew
  and Manolios, Panagiotis",
  editor="Jones, Neil D.
  and M{\"u}ller-Olm, Markus",
  title="A Posteriori Soundness for Non-deterministic Abstract Interpretations",
  booktitle="Verification, Model Checking, and Abstract Interpretation",
  year="2009",
  publisher="Springer Berlin Heidelberg",
  address="Berlin, Heidelberg",
  pages="260--274",
  abstract="An abstract interpretation's resource-allocation policy (e.g., one heap summary node per allocation site) largely determines both its speed and precision. Historically, context has driven allocation policies, and as a result, these policies are said to determine the ``context-sensitivity'' of the analysis. This work gives analysis designers newfound freedom to manipulate speed and precision by severing the link between allocation policy and context-sensitivity: abstract allocation policies may be unhinged not only from context, but also from even a predefined correspondence with a concrete allocation policy. We do so by proving that abstract allocation policies can be made non-deterministic without sacrificing correctness; this non-determinism permits precision-guided allocation policies previously assumed to be unsafe. To prove correctness, we introduce the notion of a posteriori soundness for an analysis. A proof of a posteriori soundness differs from a standard proof of soundness in that the abstraction maps used in an a posteriori proof cannot be constructed until after an analysis has been run. Delaying construction allows them to be built so as to justify the decisions made by non-determinism. The crux of the a posteriori soundness theorem is to demonstrate that a justifying abstraction map can always be constructed.",
  isbn="978-3-540-93900-9"
  }

pdf: aposteriori.pdf
---

## Summary

## Key Contributions

## Notes

