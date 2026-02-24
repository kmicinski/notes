---
title: Fast Equi-Join Algorithms on GPUs: Design and Implementation
date: 2026-02-16
type: paper
bibtex: |
  @inproceedings{fast-equijoin,
  	author = {Rui, Ran and Tu, Yi-Cheng},
  	title = {Fast Equi-Join Algorithms on GPUs: Design and Implementation},
  	year = {2017},
  	isbn = {9781450352826},
  	publisher = {Association for Computing Machinery},
  	address = {New York, NY, USA},
  	url = {https://doi.org/10.1145/3085504.3085521},
  	doi = {10.1145/3085504.3085521},
  	abstract = {Processing relational joins on modern GPUs has attracted much attention in the past few years. With the rapid development on the hardware and software environment in the GPU world, the existing GPU join algorithms designed for earlier architecture cannot make the most out of latest GPU products. In this paper, we report new design and implementation of join algorithms with high performance under today's GPGPU environment. This is a key component of our scientific database engine named G-SDMS. In particular, we overhaul the popular radix hash join and redesign sort-merge join algorithms on GPUs by applying a series of novel techniques to utilize the hardware capacity of latest Nvidia GPU architecture and new features of the CUDA programming framework. Our algorithms take advantage of revised hardware arrangement, larger register file and shared memory, native atomic operation, dynamic parallelism, and CUDA Streams. Experiments show that our new hash join algorithm is 2.0 to 14.6 times as efficient as existing GPU implementation, while the new sort-merge join achieves a speedup of 4.0X to 4.9X. Compared to the best CPU sort-merge join and hash join known to date, our optimized code achieves up to 10.5X and 5.5X speedup. Moreover, we extend our design to scenarios where large data tables cannot fit in the GPU memory.},
  	booktitle = {Proceedings of the 29th International Conference on Scientific and Statistical Database Management},
  	articleno = {17},
  	numpages = {12},
  	location = {Chicago, IL, USA},
  	series = {SSDBM '17}
  }
doi: 10.1145/3085504.3085521
pdf: fast-equijoin.pdf
---

## Summary

## Key Contributions

## Notes

<!-- BEGIN AUTO-CITATIONS -->
## References

- [@0469d9] Sort vs. Hash revisited: fast join implementation on modern multi-core CPUs
- [@39e1e9] SciDB: A database management system for applications with complex analytics
- [@439311] Merge path-parallel merging made simple
- [@74cff4] Sloavx: Scalable logarithmic alltoallv algorithm for hierarchical multicore systems
- [@853f8b] Datalog Reloaded: First International Workshop, Datalog 2010, Oxford, UK, March 16-19, 2010. Revised Selected Papers
- [@90c864] Relational Query Coprocessing on Graphics Processors
- [@a1b755] Main-Memory Hash Joins on Multi-Core CPUs: Tuning to the Underlying Hardware
- [@bc1e97] 2015 IEEE International Conference on Big Data (Big Data)
- [@e77394] GPU merge path: a GPU merging algorithm
<!-- END AUTO-CITATIONS -->
