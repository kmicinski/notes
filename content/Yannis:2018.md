---
title: Building Efficient Query Engines in a High-Level Language
date: 2026-02-16
type: paper
bibtex: |
  @article{Yannis:2018,
  author = {Shaikhha, Amir and Klonatos, Yannis and Koch, Christoph},
  title = {Building Efficient Query Engines in a High-Level Language},
  year = {2018},
  issue_date = {March 2018},
  publisher = {Association for Computing Machinery},
  address = {New York, NY, USA},
  volume = {43},
  number = {1},
  issn = {0362-5915},
  url = {https://doi.org/10.1145/3183653},
  doi = {10.1145/3183653},
  abstract = {Abstraction without regret refers to the vision of using high-level programming languages for systems development without experiencing a negative impact on performance. A database system designed according to this vision offers both increased productivity and high performance instead of sacrificing the former for the latter as is the case with existing, monolithic implementations that are hard to maintain and extend.In this article, we realize this vision in the domain of analytical query processing. We present LegoBase, a query engine written in the high-level programming language Scala. The key technique to regain efficiency is to apply generative programming: LegoBase performs source-to-source compilation and optimizes database systems code by converting the high-level Scala code to specialized, low-level C code. We show how generative programming allows to easily implement a wide spectrum of optimizations, such as introducing data partitioning or switching from a row to a column data layout, which are difficult to achieve with existing low-level query compilers that handle only queries. We demonstrate that sufficiently powerful abstractions are essential for dealing with the complexity of the optimization effort, shielding developers from compiler internals and decoupling individual optimizations from each other.We evaluate our approach with the TPC-H benchmark and show that (a) with all optimizations enabled, our architecture significantly outperforms a commercial in-memory database as well as an existing query compiler. (b) Programmers need to provide just a few hundred lines of high-level code for implementing the optimizations, instead of complicated low-level code that is required by existing query compilation approaches. (c) These optimizations may potentially come at the cost of using more system memory for improved performance. (d) The compilation overhead is low compared to the overall execution time, thus making our approach usable in practice for compiling query engines.},
  journal = {ACM Trans. Database Syst.},
  month = {apr},
  articleno = {4},
  numpages = {45},
  keywords = {query processing, query compilation, optimizing compilers, code generation, abstraction without regret, High-level programming languages}
  }
doi: 10.1145/3183653
---

## Summary

## Key Contributions

## Notes

