---
title: Functional pearl: a SQL to C compiler in 500 lines of code
date: 2026-02-16
type: paper
bibtex: |
  @inproceedings{Rompf:2015,
  author = {Rompf, Tiark and Amin, Nada},
  title = {Functional pearl: a SQL to C compiler in 500 lines of code},
  year = {2015},
  isbn = {9781450336697},
  publisher = {Association for Computing Machinery},
  address = {New York, NY, USA},
  url = {https://doi.org/10.1145/2784731.2784760},
  doi = {10.1145/2784731.2784760},
  abstract = {We present the design and implementation of a SQL query processor that outperforms existing database systems and is written in just about 500 lines of Scala code -- a convincing case study that high-level functional programming can handily beat C for systems-level programming where the last drop of performance matters. The key enabler is a shift in perspective towards generative programming. The core of the query engine is an interpreter for relational algebra operations, written in Scala. Using the open-source LMS Framework (Lightweight Modular Staging), we turn this interpreter into a query compiler with very low effort. To do so, we capitalize on an old and widely known result from partial evaluation known as Futamura projections, which state that a program that can specialize an interpreter to any given input program is equivalent to a compiler. In this pearl, we discuss LMS programming patterns such as mixed-stage data structures (e.g. data records with static schema and dynamic field components) and techniques to generate low-level C code, including specialized data structures and data loading primitives.},
  booktitle = {Proceedings of the 20th ACM SIGPLAN International Conference on Functional Programming},
  pages = {2–9},
  numpages = {8},
  keywords = {Futamura Projections, Generative Programming, Query Compilation, SQL, Staging},
  location = {Vancouver, BC, Canada},
  series = {ICFP 2015}
  }
doi: 10.1145/2784731.2784760
---

## Summary

## Key Contributions

## Notes

