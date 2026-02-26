---
title: Abstracting abstract machines
date: 2026-02-16
type: paper
bibtex: |
  @inproceedings{aam,
  author = {Van Horn, David and Might, Matthew},
  title = {Abstracting abstract machines},
  year = {2010},
  isbn = {9781605587943},
  publisher = {Association for Computing Machinery},
  address = {New York, NY, USA},
  url = {https://doi.org/10.1145/1863543.1863553},
  doi = {10.1145/1863543.1863553},
  abstract = {We describe a derivational approach to abstract interpretation that yields novel and transparently sound static analyses when applied to well-established abstract machines. To demonstrate the technique and support our claim, we transform the CEK machine of Felleisen and Friedman, a lazy variant of Krivine's machine, and the stack-inspecting CM machine of Clements and Felleisen into abstract interpretations of themselves. The resulting analyses bound temporal ordering of program events; predict return-flow and stack-inspection behavior; and approximate the flow and evaluation of by-need parameters. For all of these machines, we find that a series of well-known concrete machine refactorings, plus a technique we call store-allocated continuations, leads to machines that abstract into static analyses simply by bounding their stores. We demonstrate that the technique scales up uniformly to allow static analysis of realistic language features, including tail calls, conditionals, side effects, exceptions, first-class continuations, and even garbage collection.},
  booktitle = {Proceedings of the 15th ACM SIGPLAN International Conference on Functional Programming},
  pages = {51–62},
  numpages = {12},
  keywords = {abstract interpretation, abstract machines},
  location = {Baltimore, Maryland, USA},
  series = {ICFP '10}
  }
doi: 10.1145/1863543.1863553

pdf: aam.pdf
---

## Summary

## Key Contributions

## Notes

