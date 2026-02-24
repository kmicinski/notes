---
title: Semantic Code Search via Equational Reasoning
date: 2026-02-16
type: paper
bibtex: |
  @inproceedings{semantic-search,
  author = {Premtoon, Varot and Koppel, James and Solar-Lezama, Armando},
  title = {Semantic Code Search via Equational Reasoning},
  year = {2020},
  isbn = {9781450376136},
  publisher = {Association for Computing Machinery},
  address = {New York, NY, USA},
  url = {https://doi.org/10.1145/3385412.3386001},
  doi = {10.1145/3385412.3386001},
  abstract = {We present a new approach to semantic code search based on equational reasoning, and the Yogo tool implementing this approach. Our approach works by considering not only the dataflow graph of a function, but also the dataflow graphs of all equivalent functions reachable via a set of rewrite rules. In doing so, it can recognize an operation even if it uses alternate APIs, is in a different but mathematically-equivalent form, is split apart with temporary variables, or is interleaved with other code. Furthermore, it can recognize when code is an instance of some higher-level concept such as iterating through a file. Because of this, from a single query, Yogo can find equivalent code in multiple languages. Our evaluation further shows the utility of Yogo beyond code search: encoding a buggy pattern as a Yogo query, we found a bug in Oracle’s Graal compiler which had been missed by a hand-written static analyzer designed for that exact kind of bug. Yogo is built on the Cubix multi-language infrastructure, and currently supports Java and Python.},
  booktitle = {Proceedings of the 41st ACM SIGPLAN Conference on Programming Language Design and Implementation},
  pages = {1066–1082},
  numpages = {17},
  keywords = {code search, equational reasoning},
  location = {London, UK},
  series = {PLDI 2020}
  }
doi: 10.1145/3385412.3386001
pdf: semantic-search.pdf
---

## Summary

## Key Contributions

## Notes

