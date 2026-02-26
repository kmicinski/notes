---
title: Incremental whole-program analysis in Datalog with lattices
date: 2026-02-16
type: paper
bibtex: |
  @inproceedings{inca,
  author = {Szab\'{o}, Tam\'{a}s and Erdweg, Sebastian and Bergmann, G\'{a}bor},
  title = {Incremental whole-program analysis in Datalog with lattices},
  year = {2021},
  isbn = {9781450383912},
  publisher = {Association for Computing Machinery},
  address = {New York, NY, USA},
  url = {https://doi.org/10.1145/3453483.3454026},
  doi = {10.1145/3453483.3454026},
  abstract = {Incremental static analyses provide up-to-date analysis results in time proportional to the size of a code change, not the entire code base. This promises fast feedback to programmers in IDEs and when checking in commits. However, existing incremental analysis frameworks fail to deliver on this promise for whole-program lattice-based data-flow analyses. In particular, prior Datalog-based frameworks yield good incremental performance only for intra-procedural analyses. In this paper, we first present a methodology to empirically test if a computation is amenable to incrementalization. Using this methodology, we find that incremental whole-program analysis may be possible. Second, we present a new incremental Datalog solver called LADDDER to eliminate the shortcomings of prior Datalog-based analysis frameworks. Our Datalog solver uses a non-standard aggregation semantics which allows us to loosen monotonicity requirements on analyses and to improve the performance of lattice aggregators considerably. Our evaluation on real-world Java code confirms that LADDDER provides up-to-date points-to, constant propagation, and interval information in milliseconds.},
  booktitle = {Proceedings of the 42nd ACM SIGPLAN International Conference on Programming Language Design and Implementation},
  pages = {1–15},
  numpages = {15},
  keywords = {Static Analysis, Incremental Computing, Datalog},
  location = {Virtual, Canada},
  series = {PLDI 2021}
  }
doi: 10.1145/3453483.3454026
pdf: inca.pdf
---

## Summary

## Key Contributions

## Notes

