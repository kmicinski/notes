---
title: 2023 IEEE International Conference on Cluster Computing (CLUSTER)
date: 2026-02-16
type: paper
bibtex: |
  @INPROCEEDINGS {cluster-sun,
  author = {Y. Sun and S. Kumar and T. Gilray and K. Micinski},
  booktitle = {2023 IEEE International Conference on Cluster Computing (CLUSTER)},
  title = {Communication-Avoiding Recursive Aggregation},
  year = {2023},
  volume = {},
  issn = {},
  pages = {197-208},
  abstract = {Recursive aggregation has been of considerable interest due to its unifying a wide range of deductive-analytic workloads, including social-media mining and graph analytics. For example, Single-Source Shortest Paths (SSSP), Connected Components (CC), and PageRank may all be expressed via recursive aggregates. Implementing recursive aggregation has posed a serious algorithmic challenge, with state-of-the-art work identifying sufficient conditions (e.g., pre-mappability) under which implementations may push aggregation within recursion, avoiding the serious materialization overhead inherent to traditional reachability-based methods (e.g., Datalog).State-of-the-art implementations of engines supporting recursive aggregates focus on large unified machines, due to the challenges posed by mixing semi-naïve evaluation with distribution. In this work, we present an approach to implementing recursive aggregates on high-performance clusters which avoids the communication overhead inhibiting current-generation distributed systems to scale recursive aggregates to extremely high process counts. Our approach leverages the observation that aggregators form functional dependencies, allowing us to implement recursive aggregates via a high-parallel local aggregation to ensure maximal throughput. Additionally, we present a dynamic join planning mechanism, which customizes join order per-iteration based on dynamic relation sizes. We implemented our approach in PARALAGG, a library which allows the declarative implementation of queries which utilize recursive aggregates and executes them using our MPI-based runtime. We evaluate PARALAGG on a large unified node and leadership-class supercomputers, demonstrating scalability up to 16,384 processes.},
  keywords = {sufficient conditions;aggregates;scalability;heuristic algorithms;layout;clustering algorithms;throughput},
  doi = {10.1109/CLUSTER52292.2023.00024},
  url = {https://doi.ieeecomputersociety.org/10.1109/CLUSTER52292.2023.00024},
  publisher = {IEEE Computer Society},
  address = {Los Alamitos, CA, USA},
  month = {nov}
  }
doi: 10.1109/CLUSTER52292.2023.00024
---

## Summary

## Key Contributions

## Notes

