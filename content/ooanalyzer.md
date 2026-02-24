---
title: Using Logic Programming to Recover C++ Classes and Methods from Compiled Executables
date: 2026-02-16
type: paper
bibtex: |
  @inproceedings{ooanalyzer,
  author = {Schwartz, Edward J. and Cohen, Cory F. and Duggan, Michael and Gennari, Jeffrey and Havrilla, Jeffrey S. and Hines, Charles},
  title = {Using Logic Programming to Recover C++ Classes and Methods from Compiled Executables},
  year = {2018},
  isbn = {9781450356930},
  publisher = {Association for Computing Machinery},
  address = {New York, NY, USA},
  url = {https://doi.org/10.1145/3243734.3243793},
  doi = {10.1145/3243734.3243793},
  abstract = {High-level C++ source code abstractions such as classes and methods greatly assist human analysts and automated algorithms alike when analyzing C++ programs. Unfortunately, these abstractions are lost when compiling C++ source code, which impedes the understanding of C++ executables. In this paper, we propose a system, OOAnalyzer, that uses an innovative new design to statically recover detailed C++ abstractions from executables in a scalable manner. OOAnalyzer's design is motivated by the observation that many human analysts reason about C++ programs by recognizing simple patterns in binary code and then combining these findings using logical inference, domain knowledge, and intuition. We codify this approach by combining a lightweight symbolic analysis with a flexible Prolog-based reasoning system. Unlike most existing work, OOAnalyzer is able to recover both polymorphic and non-polymorphic C++ classes. We show in our evaluation that OOAnalyzer assigns over 78\% of methods to the correct class on our test corpus, which includes both malware and real-world software such as Firefox and MySQL. These recovered abstractions can help analysts understand the behavior of C++ malware and cleanware, and can also improve the precision of program analyses on C++ executables.},
  booktitle = {Proceedings of the 2018 ACM SIGSAC Conference on Computer and Communications Security},
  pages = {426–441},
  numpages = {16},
  keywords = {software reverse engineering, malware analysis, binary analysis},
  location = {Toronto, Canada},
  series = {CCS '18}
  }
doi: 10.1145/3243734.3243793
---

## Summary

## Key Contributions

## Notes

