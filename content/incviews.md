---
title: Maintaining views incrementally
date: 2026-02-16
type: paper
bibtex: |
  @inproceedings{incviews,
  author = {Gupta, Ashish and Mumick, Inderpal Singh and Subrahmanian, V. S.},
  title = {Maintaining views incrementally},
  year = {1993},
  isbn = {0897915925},
  publisher = {Association for Computing Machinery},
  address = {New York, NY, USA},
  url = {https://doi.org/10.1145/170035.170066},
  doi = {10.1145/170035.170066},
  abstract = {We present incremental evaluation algorithms to compute changes to materialized views in relational and deductive database systems, in response to changes (insertions, deletions, and updates) to the relations. The view definitions can be in SQL or Datalog, and may use UNION, negation, aggregation (e.g. SUM, MIN), linear recursion, and general recursion.We first present a counting algorithm that tracks the number of alternative derivations (counts) for each derived tuple in a view. The algorithm works with both set and duplicate semantics. We present the algorithm for nonrecursive views (with negation and aggregation), and show that the count for a tuple can be computed at little or no cost above the cost of deriving the tuple. The algorithm is optimal in that it computes exactly those view tuples that are inserted or deleted. Note that we store only the number of derivations, not the derivations themselves.We then present the Delete and Rederive algorithm, DRed, for incremental maintenance of recursive views (negation and aggregation are permitted). The algorithm works by first deleting a superset of the tuples that need to be deleted, and then rederiving some of them. The algorithm can also be used when the view definition is itself altered.},
  booktitle = {Proceedings of the 1993 ACM SIGMOD International Conference on Management of Data},
  pages = {157–166},
  numpages = {10},
  location = {Washington, D.C., USA},
  series = {SIGMOD '93}
  }
doi: 10.1145/170035.170066
pdf: incviews.pdf
---

## Summary

## Key Contributions

## Notes
