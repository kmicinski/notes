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

<!-- BEGIN AUTO-CITATIONS -->
## References

- [@08cc77] MPI-aware compiler optimizations for improving communication-computation overlap (author_year:50%)
- [@432be8] Declarative recursive computation on an rdbms, or, why you should use a database for distributed machine learning (author_year:50%)
- [@5ab54d] User Interactions and Permission Use on Android (author_year:50%)
- [@611b7e] Design and Implementation of the LogicBlox System (doi:100%)
- [@624ae6] A Survey of Binary Code Similarity (author_year:50%)
- [@625a84] Efficient Join Algorithms for Large Database Tables in a Multi-GPU Environment (author_year:50%)
- [@89b366] DeepDi (author_year:50%)
- [@8d6412] Better together: Unifying datalog and equality saturation (author_year:50%)
- [@939f70] Leapfrog Triejoin: A Simple, Worst-Case Optimal Join Algorithm (author_year:50%)
- [@93ae2f] Realistic, mathematically tractable graph generation and evolution, using kronecker multiplication (author_year:50%)
- [@9eee52] Declarative Probabilistic Programming with Datalog (author_year:50%)
- [@a26e84] RetDec: An Open-Source Machine-Code Decompiler Based on LLVM (author_year:50%)
- [@aaa112] Code Llama (author_year:50%)
- [@c91c64] 2019 IEEE/ACM 41st International Conference on Software Engineering (ICSE) (author_year:50%)
- [@df9cd2] Asynchronous and fault-tolerant recursive datalog evaluation in shared-nothing engines (author_year:50%)
- [@fc4889] Equality Saturation: A New Approach to Optimization (author_year:50%)
<!-- END AUTO-CITATIONS -->
