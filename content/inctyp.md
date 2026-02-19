---
title: A systematic approach to deriving incremental type checkers
date: 2026-02-16
type: paper
bibtex: |
  @article{inctyp,
  author = {Pacak, Andr\'{e} and Erdweg, Sebastian and Szab\'{o}, Tam\'{a}s},
  title = {A systematic approach to deriving incremental type checkers},
  year = {2020},
  issue_date = {November 2020},
  publisher = {Association for Computing Machinery},
  address = {New York, NY, USA},
  volume = {4},
  number = {OOPSLA},
  url = {https://doi.org/10.1145/3428195},
  doi = {10.1145/3428195},
  abstract = {Static typing can guide programmers if feedback is immediate. Therefore, all major IDEs incrementalize type checking in some way. However, prior approaches to incremental type checking are often specialized and hard to transfer to new type systems. In this paper, we propose a systematic approach for deriving incremental type checkers from textbook-style type system specifications. Our approach is based on compiling inference rules to Datalog, a carefully limited logic programming language for which incremental solvers exist. The key contribution of this paper is to discover an encoding of the infinite typing relation as a finite Datalog relation in a way that yields efficient incremental updates. We implemented the compiler as part of a type system DSL and show that it supports simple types, some local type inference, operator overloading, universal types, and iso-recursive types.},
  journal = {Proc. ACM Program. Lang.},
  month = nov,
  articleno = {127},
  numpages = {28},
  keywords = {datalog, incremental type checking, type system transformation}
  }
doi: 10.1145/3428195
---

## Summary

## Key Contributions

## Notes

