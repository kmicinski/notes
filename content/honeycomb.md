---
title: HoneyComb: A Parallel Worst-Case Optimal Join on Multicores
date: 2026-02-18
type: paper
bibtex: |
  @article{honeycomb,
  author = {Wu, Jiacheng and Suciu, Dan},
  title = {HoneyComb: A Parallel Worst-Case Optimal Join on Multicores},
  year = {2025},
  issue_date = {June 2025},
  publisher = {Association for Computing Machinery},
  address = {New York, NY, USA},
  volume = {3},
  number = {3},
  url = {https://doi.org/10.1145/3725307},
  doi = {10.1145/3725307},
  abstract = {To achieve true scalability on massive datasets, a modern query engine needs to be able to take advantage of large, shared-memory, multicore systems. Binary joins are conceptually easy to parallelize on a multicore system; however, several applications require a different approach to query evaluation, using a Worst-Case Optimal Join (WCOJ) algorithm. WCOJ is known to outperform traditional query plans for cyclic queries. However, there is no obvious adaptation of WCOJ to parallel architectures. The few existing systems that parallelizeWCOJ do this by partitioning only the top variable of theWCOJ algorithm. This leads to work skew (since some relations end up being read entirely by every thread), possible contention between threads (when the hierarchical trie index is built lazily, which is the case on most recent WCOJ systems), and exacerbates the redundant computations already existing in WCOJ.},
  journal = {Proc. ACM Manag. Data},
  month = jun,
  articleno = {170},
  numpages = {27},
  keywords = {multicore, optimization, parallelization, worst-case optimal join}
  }
pdf: 2502.06715v1.pdf
---

## Summary

## Key Contributions

## Notes