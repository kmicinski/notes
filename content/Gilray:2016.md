---
title: Allocation characterizes polyvariance: a unified methodology for polyvariant control-flow analysis
date: 2026-02-16
type: paper
bibtex: |
  @inproceedings{Gilray:2016,
  author = {Gilray, Thomas and Adams, Michael D. and Might, Matthew},
  title = {Allocation characterizes polyvariance: a unified methodology for polyvariant control-flow analysis},
  year = {2016},
  isbn = {9781450342193},
  publisher = {Association for Computing Machinery},
  address = {New York, NY, USA},
  url = {https://doi.org/10.1145/2951913.2951936},
  doi = {10.1145/2951913.2951936},
  abstract = {The polyvariance of a static analysis is the degree to which it structurally differentiates approximations of program values. Polyvariant techniques come in a number of different flavors that represent alternative heuristics for managing the trade-off an analysis strikes between precision and complexity. For example, call sensitivity supposes that values will tend to correlate with recent call sites, object sensitivity supposes that values will correlate with the allocation points of related objects, the Cartesian product algorithm supposes correlations between the values of arguments to the same function, and so forth. In this paper, we describe a unified methodology for implementing and understanding polyvariance in a higher-order setting (i.e., for control-flow analyses). We do this by extending the method of abstracting abstract machines (AAM), a systematic approach to producing an abstract interpretation of abstract-machine semantics. AAM eliminates recursion within a language’s semantics by passing around an explicit store, and thus places importance on the strategy an analysis uses for allocating abstract addresses within the abstract heap or store. We build on AAM by showing that the design space of possible abstract allocators exactly and uniquely corresponds to the design space of polyvariant strategies. This allows us to both unify and generalize polyvariance as tunings of a single function. Changes to the behavior of this function easily recapitulate classic styles of analysis and produce novel variations, combinations of techniques, and fundamentally new techniques.},
  booktitle = {Proceedings of the 21st ACM SIGPLAN International Conference on Functional Programming},
  pages = {407–420},
  numpages = {14},
  keywords = {Static analysis, Polyvariance, Control-flow analysis, Context sensitivity, Abstract interpretation, Abstract allocation},
  location = {Nara, Japan},
  series = {ICFP 2016}
  }
doi: 10.1145/2951913.2951936
pdf: Gilray2016.pdf
---

## Summary

## Key Contributions

## Notes

