---
title: GPU Join Processing Revisited
date: 2026-02-16
type: paper
bibtex: |
  @inproceedings{GPU-join-revisited,
  	author = {Kaldewey, Tim and Lohman, Guy and Mueller, Rene and Volk, Peter},
  	title = {GPU Join Processing Revisited},
  	year = {2012},
  	isbn = {9781450314459},
  	publisher = {Association for Computing Machinery},
  	address = {New York, NY, USA},
  	url = {https://doi.org/10.1145/2236584.2236592},
  	doi = {10.1145/2236584.2236592},
  	abstract = {Until recently, the use of graphics processing units (GPUs) for query processing was limited by the amount of memory on the graphics card, a few gigabytes at best. Moreover, input tables had to be copied to GPU memory before they could be processed, and after computation was completed, query results had to be copied back to CPU memory. The newest generation of Nvidia GPUs and development tools introduces a common memory address space, which now allows the GPU to access CPU memory directly, lifting size limitations and obviating data copy operations. We confirm that this new technology can sustain 98% of its nominal rate of 6.3 GB/sec in practice, and exploit it to process database hash joins at the same rate, i.e., the join is processed "on the fly" as the GPU reads the input tables from CPU memory at PCI-E speeds. Compared to the fastest published results for in-memory joins on the CPU, this represents more than half an order of magnitude speed-up. All of our results include the cost of result materialization (often omitted in earlier work), and we investigate the implications of changing join predicate selectivity and table size.},
  	booktitle = {Proceedings of the Eighth International Workshop on Data Management on New Hardware},
  	pages = {55–62},
  	numpages = {8},
  	location = {Scottsdale, Arizona},
  	series = {DaMoN '12}
  }
doi: 10.1145/2236584.2236592
---

## Summary

## Key Contributions

## Notes

