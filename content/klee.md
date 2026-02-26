---
title: KLEE: Unassisted and Automatic Generation of High-Coverage Tests for Complex Systems Programs
date: 2026-02-16
type: paper
bibtex: |
  @inproceedings{klee,
  author = {Cadar, Cristian and Dunbar, Daniel and Engler, Dawson},
  title = {KLEE: Unassisted and Automatic Generation of High-Coverage Tests for Complex Systems Programs},
  year = {2008},
  publisher = {USENIX Association},
  address = {USA},
  abstract = {We present a new symbolic execution tool, KLEE, capable of automatically generating tests that achieve high coverage on a diverse set of complex and environmentally-intensive programs. We used KLEE to thoroughly check all 89 stand-alone programs in the GNU COREUTILS utility suite, which form the core user-level environment installed on millions of Unix systems, and arguably are the single most heavily tested set of open-source programs in existence. KLEE-generated tests achieve high line coverage -- on average over 90% per tool (median: over 94%) -- and significantly beat the coverage of the developers' own hand-written test suite. When we did the same for 75 equivalent tools in the BUSYBOX embedded system suite, results were even better, including 100% coverage on 31 of them.We also used KLEE as a bug finding tool, applying it to 452 applications (over 430K total lines of code), where it found 56 serious bugs, including three in COREUTILS that had been missed for over 15 years. Finally, we used KLEE to crosscheck purportedly identical BUSYBOX and COREUTILS utilities, finding functional correctness errors and a myriad of inconsistencies.},
  booktitle = {Proceedings of the 8th USENIX Conference on Operating Systems Design and Implementation},
  pages = {209–224},
  numpages = {16},
  location = {San Diego, California},
  series = {OSDI'08}
  }
---

## Summary

## Key Contributions

## Notes

