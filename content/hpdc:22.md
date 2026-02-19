---
title: Optimizing the Bruck Algorithm for Non-uniform All-to-all Communication
date: 2026-02-16
type: paper
bibtex: |
  @inproceedings{hpdc:22,
  author = {Fan, Ke and Gilray, Thomas and Pascucci, Valerio and Huang, Xuan and Micinski, Kristopher and Kumar, Sidharth},
  title = {Optimizing the Bruck Algorithm for Non-uniform All-to-all Communication},
  year = {2022},
  isbn = {9781450391993},
  publisher = {Association for Computing Machinery},
  address = {New York, NY, USA},
  url = {https://doi.org/10.1145/3502181.3531468},
  doi = {10.1145/3502181.3531468},
  abstract = {In MPI, collective routines MPI_Alltoall and MPI_Alltoallv play an important role in facilitating all-to-all inter-process data exchange. MPI_Alltoallv is a generalization of MPI_Alltoall, supporting the exchange of non-uniform distributions of data. Popular implementations of MPI, such as MPICH and OpenMPI, implement MPI_Alltoall using a combination of techniques such as the Spread-out algorithm and the Bruck algorithm. Spread-out has a linear complexity in P, compared to Bruck's logarithmic complexity (P: process count); a selection between these two techniques is made at runtime based on the data block size. However, MPI_Alltoallv is typically implemented using only variants of the spread-out algorithm, and therefore misses out on the performance benefits that the log-time Bruck algorithm offers (especially for smaller data loads).In this paper, we first implement and empirically evaluate all existing variants of the Bruck algorithm for uniform and non-uniform data loads-this forms the basis for our own Bruck-based non-uniform all-to-all algorithms. In particular, we developed two open-source implementations, padded Bruck and two-phase Bruck, that efficiently generalize Bruck algorithm to non-uniform all-to-all data exchange. We empirically validate the techniques on three supercomputers: Theta, Cori, and Stampede, using both microbenchmarks and two real-world applications: graph mining and program analysis. We perform weak and strong scaling studies for a range of average message sizes, degrees of imbalance, and distribution schemes, and demonstrate that our techniques outperform vendor-optimized Cray's MPI_Alltoallv by as much as 50\% for some workloads and scales.},
  booktitle = {Proceedings of the 31st International Symposium on High-Performance Parallel and Distributed Computing},
  pages = {172–184},
  numpages = {13},
  keywords = {mpi, collective communication, bruck algorithm, alltoallv},
  location = {Minneapolis, MN, USA},
  series = {HPDC '22}
  }
doi: 10.1145/3502181.3531468
---

## Summary

## Key Contributions

## Notes

