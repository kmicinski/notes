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

- [@08cc77] danalis2009mpi
- [@195d2f] MPI collectives for multi-core clusters: Optimized performance of the hybrid MPI+ MPI parallel codes
- [@26a42a] The art of balance: a RateupDB™ experience of building a CPU/GPU hybrid database product
- [@432be8] Declarative recursive computation on an rdbms, or, why you should use a database for distributed machine learning
- [@5b108c] Database architecture optimized for the new bottleneck: Memory access
- [@611b7e] Design and Implementation of the LogicBlox System
- [@624ae6] Haq:2021
- [@625a84] Efficient Join Algorithms for Large Database Tables in a Multi-GPU Environment
- [@7af265] Optimizing parallel recursive datalog evaluation on multicore machines
- [@89b366] DeepDi
- [@939f70] Leapfrog Triejoin: A Simple, Worst-Case Optimal Join Algorithm
- [@93ae2f] Realistic, mathematically tractable graph generation and evolution, using kronecker multiplication
- [@9eee52] Declarative Probabilistic Programming with Datalog
- [@a26e84] RetDec: An Open-Source Machine-Code Decompiler Based on LLVM
- [@aaa112] roziere2023codellama
- [@d02082] Free Join: Unifying Worst-Case Optimal and Traditional Joins
- [@df96b5] WarpCore: A Library for fast Hash Tables on GPUs
- [@df9cd2] Asynchronous and fault-tolerant recursive datalog evaluation in shared-nothing engines
- [@f0d4e7] 8425196
- [@fc4889] tate:2009
<!-- END AUTO-CITATIONS -->
