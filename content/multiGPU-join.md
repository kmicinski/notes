---
title: Efficient Join Algorithms for Large Database Tables in a Multi-GPU Environment
date: 2026-02-16
type: paper
bibtex: |
  @article{multiGPU-join,
  	author = {Rui, Ran and Li, Hao and Tu, Yi-Cheng},
  	title = {Efficient Join Algorithms for Large Database Tables in a Multi-GPU Environment},
  	year = {2020},
  	issue_date = {December 2020},
  	publisher = {VLDB Endowment},
  	volume = {14},
  	number = {4},
  	issn = {2150-8097},
  	url = {https://doi.org/10.14778/3436905.3436927},
  	doi = {10.14778/3436905.3436927},
  	abstract = {Relational join processing is one of the core functionalities in database management systems. It has been demonstrated that GPUs as a general-purpose parallel computing platform is very promising in processing relational joins. However, join algorithms often need to handle very large input data, which is an issue that was not sufficiently addressed in existing work. Besides, as more and more desktop and workstation platforms support multi-GPU environment, the combined computing capability of multiple GPUs can easily achieve that of a computing cluster. It is worth exploring how join processing would benefit from the adaptation of multiple GPUs. We identify the low rate and complex patterns of data transfer among the CPU and GPUs as the main challenges in designing efficient algorithms for large table joins. To overcome such challenges, we propose three distinctive designs of multi-GPU join algorithms, namely, the nested loop, global sort-merge and hybrid joins for large table joins with different join conditions. Extensive experiments running on multiple databases and two different hardware configurations demonstrate high scalability of our algorithms over data size and significant performance boost brought by the use of multiple GPUs. Furthermore, our algorithms achieve much better performance as compared to existing join algorithms, with a speedup up to 25X and 2.8X over best known code developed for multi-core CPUs and GPUs respectively.},
  	journal = {Proc. VLDB Endow.},
  	month = dec,
  	pages = {708–720},
  	numpages = {13}
  }
doi: 10.14778/3436905.3436927
pdf: p708-rui.pdf
---

## Summary

## Key Contributions

## Notes

<!-- BEGIN AUTO-CITATIONS -->
## References

- [@0469d9] Sort vs. Hash revisited: fast join implementation on modern multi-core CPUs
- [@335fc9] Fast Equi-Join Algorithms on GPUs: Design and Implementation
- [@39e1e9] SciDB: A database management system for applications with complex analytics
- [@439311] Merge path-parallel merging made simple
- [@74cff4] Sloavx: Scalable logarithmic alltoallv algorithm for hierarchical multicore systems
- [@853f8b] Datalog Reloaded: First International Workshop, Datalog 2010, Oxford, UK, March 16-19, 2010. Revised Selected Papers
- [@90c864] Relational Query Coprocessing on Graphics Processors
- [@a1b755] Main-Memory Hash Joins on Multi-Core CPUs: Tuning to the Underlying Hardware
- [@bc1e97] 2015 IEEE International Conference on Big Data (Big Data)
- [@be8404] 2019 IEEE 21st International Conference on High Performance Computing and Communications; IEEE 17th International Conference on Smart City; IEEE 5th International Conference on Data Science and Systems (HPCC/SmartCity/DSS)
- [@d4f8cb] Hardware-conscious hash-joins on gpus
- [@e77394] GPU merge path: a GPU merging algorithm
<!-- END AUTO-CITATIONS -->
