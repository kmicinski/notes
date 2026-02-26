---
title: Automatic heap layout manipulation for exploitation
date: 2026-02-16
type: paper
bibtex: |
  @inproceedings{heelan:ahl,
  author = {Heelan, Sean and Melham, Tom and Kroening, Daniel},
  title = {Automatic heap layout manipulation for exploitation},
  year = {2018},
  isbn = {9781931971461},
  publisher = {USENIX Association},
  address = {USA},
  abstract = {Heap layout manipulation is integral to exploiting heap-based memory corruption vulnerabilities. In this paper we present the first automatic approach to the problem, based on pseudo-random black-box search. Our approach searches for the inputs required to place the source of a heap-based buffer overflow or underflow next to heap-allocated objects that an exploit developer, or automatic exploit generation system, wishes to read or corrupt. We present a framework for benchmarking heap layout manipulation algorithms, and use it to evaluate our approach on several real-world allocators, showing that pseudo-random black box search can be highly effective. We then present SHRIKE, a novel system that can perform automatic heap layout manipulation on the PHP interpreter and can be used in the construction of controlflow hijacking exploits. Starting from PHP's regression tests, SHRIKE discovers fragments of PHP code that interact with the interpreter's heap in useful ways, such as making allocations and deallocations of particular sizes, or allocating objects containing sensitive data, such as pointers. SHRIKE then uses our search algorithm to piece together these fragments into programs, searching for one that achieves a desired heap layout. SHRIKE allows an exploit developer to focus on the higher level concepts in an exploit, and to defer the resolution of heap layout constraints to SHRIKE. We demonstrate this by using SHRIKE in the construction of a control-flow hijacking exploit for the PHP interpreter.},
  booktitle = {Proceedings of the 27th USENIX Conference on Security Symposium},
  pages = {763–779},
  numpages = {17},
  location = {Baltimore, MD, USA},
  series = {SEC'18}
  }
---

## Summary

## Key Contributions

## Notes

