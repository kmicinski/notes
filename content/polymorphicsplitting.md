---
title: Polymorphic splitting: an effective polyvariant flow analysis
date: 2026-02-16
type: paper
bibtex: |
  @article{polymorphicsplitting,
  author = {Wright, Andrew K. and Jagannathan, Suresh},
  title = {Polymorphic splitting: an effective polyvariant flow analysis},
  year = {1998},
  issue_date = {Jan. 1998},
  publisher = {Association for Computing Machinery},
  address = {New York, NY, USA},
  volume = {20},
  number = {1},
  issn = {0164-0925},
  url = {https://doi.org/10.1145/271510.271523},
  doi = {10.1145/271510.271523},
  abstract = {This article describes a general-purpose program analysis that
  computes global control-flow and data-flow information for
  higher-order, call-by-value languages. The analysis employs a
  novel form of polyvariance called polymorhic splitting that uses
  let-expressions as syntactic clues to gain precision. The information
  derived from the analysis is used both to eliminate run-time checks
  and to inline procedure. The analysis and optimizations have been
  applied to a suite of Scheme programs. Experimental results obtained
  from the prototype implementation indicate that the analysis is
  extremely precise and has reasonable cost. Compared to monovariant
  flow analyses such as 0CFA, or analyses based on type inference such as
  soft typing, the analysis eliminates significantly more run-time checks. Run-time check elimination and inlining together typically yield a 20 to 40\% performance improvement for the benchmark suite, with some programs running four times as fast.},
  journal = {ACM Trans. Program. Lang. Syst.},
  month = {jan},
  pages = {166–207},
  numpages = {42},
  keywords = {run-time checks, polyvariance, inlining, flow analysis}
  }
doi: 10.1145/271510.271523
---

## Summary

## Key Contributions

## Notes

