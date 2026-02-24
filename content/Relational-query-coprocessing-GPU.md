---
title: Relational Query Coprocessing on Graphics Processors
date: 2026-02-16
type: paper
bibtex: |
  @article{Relational-query-coprocessing-GPU,
  	author = {He, Bingsheng and Lu, Mian and Yang, Ke and Fang, Rui and Govindaraju, Naga K. and Luo, Qiong and Sander, Pedro V.},
  	title = {Relational Query Coprocessing on Graphics Processors},
  	year = {2009},
  	issue_date = {December 2009},
  	publisher = {Association for Computing Machinery},
  	address = {New York, NY, USA},
  	volume = {34},
  	number = {4},
  	issn = {0362-5915},
  	url = {https://doi.org/10.1145/1620585.1620588},
  	doi = {10.1145/1620585.1620588},
  	abstract = {Graphics processors (GPUs) have recently emerged as powerful coprocessors for general purpose computation. Compared with commodity CPUs, GPUs have an order of magnitude higher computation power as well as memory bandwidth. Moreover, new-generation GPUs allow writes to random memory locations, provide efficient interprocessor communication through on-chip local memory, and support a general purpose parallel programming model. Nevertheless, many of the GPU features are specialized for graphics processing, including the massively multithreaded architecture, the Single-Instruction-Multiple-Data processing style, and the execution model of a single application at a time. Additionally, GPUs rely on a bus of limited bandwidth to transfer data to and from the CPU, do not allow dynamic memory allocation from GPU kernels, and have little hardware support for write conflicts. Therefore, a careful design and implementation is required to utilize the GPU for coprocessing database queries.In this article, we present our design, implementation, and evaluation of an in-memory relational query coprocessing system, GDB, on the GPU. Taking advantage of the GPU hardware features, we design a set of highly optimized data-parallel primitives such as split and sort, and use these primitives to implement common relational query processing algorithms. Our algorithms utilize the high parallelism as well as the high memory bandwidth of the GPU, and use parallel computation and memory optimizations to effectively reduce memory stalls. Furthermore, we propose coprocessing techniques that take into account both the computation resources and the GPU-CPU data transfer cost so that each operator in a query can utilize suitable processors—the CPU, the GPU, or both—for an optimized overall performance. We have evaluated our GDB system on a machine with an Intel quad-core CPU and an NVIDIA GeForce 8800 GTX GPU. Our workloads include microbenchmark queries on memory-resident data as well as TPC-H queries that involve complex data types and multiple query operators on data sets larger than the GPU memory. Our results show that our GPU-based algorithms are 2--27x faster than their optimized CPU-based counterparts on in-memory data. Moreover, the performance of our coprocessing scheme is similar to, or better than, both the GPU-only and the CPU-only schemes.},
  	journal = {ACM Trans. Database Syst.},
  	month = dec,
  	articleno = {21},
  	numpages = {39},
  	keywords = {graphics processors, sort, Relational database, primitive, parallel processing, join}
  }
doi: 10.1145/1620585.1620588
---

## Summary

## Key Contributions

## Notes

