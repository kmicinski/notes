---
title: Efficient Incremental Static Analysis Using Path Abstraction
date: 2026-02-16
type: paper
bibtex: |
  @InProceedings{incrementalpathabstraction,
  author="Mudduluru, Rashmi
  and Ramanathan, Murali Krishna",
  editor="Gnesi, Stefania
  and Rensink, Arend",
  title="Efficient Incremental Static Analysis Using Path Abstraction",
  booktitle="Fundamental Approaches to Software Engineering",
  year="2014",
  publisher="Springer Berlin Heidelberg",
  address="Berlin, Heidelberg",
  pages="125--139",
  abstract="Incremental static analysis involves analyzing changes to a version of a source code along with analyzing code regions that are semantically affected by the changes. Existing analysis tools that attempt to perform incremental analysis can perform redundant computations due to poor abstraction. In this paper, we design a novel and efficient incremental analysis algorithm for reducing the overall analysis time. We use a path abstraction that encodes different paths in the program as a set of constraints. The constraints encoded as boolean formulas are input to a SAT solver and the (un)satisfiability of the formulas drives the analysis further. While a majority of boolean formulas are similar across multiple versions, the problem of finding their equivalence is graph isomorphism complete. We address a relaxed version of the problem by designing efficient memoization techniques to identify equivalence of boolean formulas to improve the performance of the static analysis engine. Our experimental results on a number of large codebases (upto 87 KLoC) show a performance gain of upto 32{\%} when incremental analysis is used. The overhead associated with identifying equivalence of boolean formulas is less (not more than 8.4{\%}) than the overall reduction in analysis time.",
  isbn="978-3-642-54804-8"
  }
---

## Summary

## Key Contributions

## Notes

