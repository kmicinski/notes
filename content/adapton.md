---
title: Adapton: composable, demand-driven incremental computation
date: 2026-02-16
type: paper
bibtex: |
  @inproceedings{adapton,
  author = {Hammer, Matthew A. and Phang, Khoo Yit and Hicks, Michael and Foster, Jeffrey S.},
  title = {Adapton: composable, demand-driven incremental computation},
  year = {2014},
  isbn = {9781450327848},
  publisher = {Association for Computing Machinery},
  address = {New York, NY, USA},
  url = {https://doi.org/10.1145/2594291.2594324},
  doi = {10.1145/2594291.2594324},
  abstract = {Many researchers have proposed programming languages that support incremental computation (IC), which allows programs to be efficiently re-executed after a small change to the input. However, existing implementations of such languages have two important drawbacks. First, recomputation is oblivious to specific demands on the program output; that is, if a program input changes, all dependencies will be recomputed, even if an observer no longer requires certain outputs. Second, programs are made incremental as a unit, with little or no support for reusing results outside of their original context, e.g., when reordered.To address these problems, we present λiccdd, a core calculus that applies a demand-driven semantics to incremental computation, tracking changes in a hierarchical fashion in a novel demanded computation graph. λiccdd also formalizes an explicit separation between inner, incremental computations and outer observers. This combination ensures λiccdd programs only recompute computations as demanded by observers, and allows inner computations to be reused more liberally. We present Adapton, an OCaml library implementing λiccdd. We evaluated Adapton on a range of benchmarks, and found that it provides reliable speedups, and in many cases dramatically outperforms state-of-the-art IC approaches.},
  booktitle = {Proceedings of the 35th ACM SIGPLAN Conference on Programming Language Design and Implementation},
  pages = {156–166},
  numpages = {11},
  keywords = {call-by-push-value (CBPV), demanded computation graph (DCG) incremental computation, laziness, self-adjusting computation, thunks},
  location = {Edinburgh, United Kingdom},
  series = {PLDI '14}
  }
doi: 10.1145/2594291.2594324
pdf: adapton.pdf
---

## Summary

## Key Contributions

## Notes

