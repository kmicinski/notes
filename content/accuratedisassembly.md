---
title: Accurate Disassembly of Complex Binaries Without Use of Compiler Metadata
date: 2026-02-16
type: paper
bibtex: |
  @inproceedings{accuratedisassembly,
  author = {Priyadarshan, Soumyakant and Nguyen, Huan and Sekar, R.},
  title = {Accurate Disassembly of Complex Binaries Without Use of Compiler Metadata},
  year = {2024},
  isbn = {9798400703942},
  publisher = {Association for Computing Machinery},
  address = {New York, NY, USA},
  url = {https://doi.org/10.1145/3623278.3624766},
  doi = {10.1145/3623278.3624766},
  abstract = {Accurate disassembly of stripped binaries is the first step in binary analysis, instrumentation and reverse engineering. Complex instruction sets such as the x86 pose major challenges in this context because it is very difficult to distinguish between code and embedded data. To make progress, many recent approaches have either made optimistic assumptions (e.g., absence of embedded data) or relied on additional compiler-generated metadata (e.g., relocation info and/or exception handling metadata). Unfortunately, many complex binaries do contain embedded data, while lacking the additional metadata needed by these techniques. We therefore present a novel approach for accurate disassembly that uses statistical properties of data to detect code, and behavioral properties of code to flag data. We present new static analysis and data-driven probabilistic techniques that are then combined using a prioritized error correction algorithm to achieve results that are 3X to 4X more accurate than the best previous results.},
  booktitle = {Proceedings of the 28th ACM International Conference on Architectural Support for Programming Languages and Operating Systems, Volume 4},
  pages = {1–18},
  numpages = {18},
  location = {Vancouver, BC, Canada},
  series = {ASPLOS '23}
  }
doi: 10.1145/3623278.3624766
pdf: accuratedisassembly.pdf
---

## Summary

## Key Contributions

## Notes

