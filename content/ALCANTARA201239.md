---
title: Chapter 4 - Building an Efficient Hash Table on the GPU
date: 2026-02-19
type: paper
bibtex: |
  @incollection{ALCANTARA201239,
  title = {Chapter 4 - Building an Efficient Hash Table on the GPU},
  editor = {Wen-mei W. Hwu},
  booktitle = {GPU Computing Gems Jade Edition},
  publisher = {Morgan Kaufmann},
  address = {Boston},
  pages = {39-53},
  year = {2012},
  series = {Applications of GPU Computing Series},
  isbn = {978-0-12-385963-1},
  doi = {https://doi.org/10.1016/B978-0-12-385963-1.00004-6},
  url = {https://www.sciencedirect.com/science/article/pii/B9780123859631000046},
  author = {Dan A. Alcantara and Vasily Volkov and Shubhabrata Sengupta and Michael Mitzenmacher and John D. Owens and Nina Amenta},
  abstract = {Publisher Summary
  This chapter describes a straightforward algorithm for parallel hash table construction on the graphical processing unit (GPU). It constructs the table in global memory and use atomic operations to detect and resolve collisions. Construction and retrieval performance are limited almost entirely by the time required for these uncoalesced memory accesses, which are linear in the total number of accesses; so the design goal is to minimize the average number of accesses per insertion or lookup. In fact, it guarantees a constant worst-case bound on the number of accesses per lookup. Further, one alternative to using a hash table is to store the data in a sorted array and access it via binary search. Sorted arrays can be built very quickly using radix sort because the memory access pattern of radix sort is very localized, allowing the GPU to coalesce many memory accesses and reduce their cost significantly. However, binary search, which incurs as many as lg (N) probes in the worst case, is much less efficient than hash table lookup. GPU hash tables are useful for interactive graphics applications, where they are used to store sparse spatial data—usually 3D models that are voxelized on a uniform grid. Rather than store the entire voxel grid, which is mostly empty, a hash table is built to hold just the occupied voxels.}
  }
doi: https://doi.org/10.1016/B978-0-12-385963-1.00004-6
---

## Summary

## Key Contributions

## Notes

