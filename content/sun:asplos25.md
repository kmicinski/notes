---
title: Optimizing Datalog for the GPU
date: 2026-02-16
type: paper
bibtex: |
  @inproceedings{sun:asplos25,
  author = {Sun, Yihao and Shovon, Ahmedur Rahman and Gilray, Thomas and Kumar, Sidharth and Micinski, Kristopher},
  title = {Optimizing Datalog for the GPU},
  year = {2025},
  isbn = {9798400706981},
  publisher = {Association for Computing Machinery},
  address = {New York, NY, USA},
  url = {https://doi.org/10.1145/3669940.3707274},
  doi = {10.1145/3669940.3707274},
  abstract = {Modern Datalog engines (e.g., LogicBlox, Souffl\'{e}, ddlog) enable their users to write declarative queries which compute recursive deductions over extensional facts, leaving high-performance operationalization (query planning, semi-na\"{\i}ve evaluation, and parallelization) to the engine. Such engines form the backbone of modern high-throughput applications in static analysis, network monitoring, and social-media mining. In this paper, we present a methodology for implementing a modern in-memory Datalog engine on data center GPUs, allowing us to achieve significant (up to 45\texttimes{}) gains compared to Souffl\'{e} (a modern CPU-based engine) on context-sensitive points-to analysis of httpd. We present GPUlog, a Datalog engine backend that implements iterated relational algebra kernels over a novel range-indexed data structure we call the hash-indexed sorted array (HISA). HISA combines the algorithmic benefits of incremental range-indexed relations with the raw computation throughput of operations over dense data structures. Our experiments show that GPUlog is significantly faster than CPU-based Datalog engines, while achieving favorable memory footprint compared to contemporary GPU-based joins.},
  booktitle = {Proceedings of the 30th ACM International Conference on Architectural Support for Programming Languages and Operating Systems, Volume 1},
  pages = {762–776},
  numpages = {15},
  keywords = {analytic databases, datalog, gpu},
  location = {Rotterdam, Netherlands},
  series = {ASPLOS '25}
  }
doi: 10.1145/3669940.3707274
---

## Summary

## Key Contributions

## Notes

