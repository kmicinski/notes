---
title: Symbolic Execution with SYMCC: Don't Interpret, Compile!
date: 2026-02-16
type: paper
bibtex: |
  @inproceedings{symcc,
  author = {Poeplau, Sebastian and Francillon, Aur\'{e}lien},
  title = {Symbolic Execution with SYMCC: Don't Interpret, Compile!},
  year = {2020},
  isbn = {978-1-939133-17-5},
  publisher = {USENIX Association},
  address = {USA},
  abstract = {A major impediment to practical symbolic execution is speed, especially when compared to near-native speed solutions like fuzz testing. We propose a compilation-based approach to symbolic execution that performs better than state-of-the-art implementations by orders of magnitude. We present SYMCC, an LLVM-based C and C++ compiler that builds concolic execution right into the binary. It can be used by software developers as a drop-in replacement for clang and clang++, and we show how to add support for other languages with little effort. In comparison with KLEE, SYMCC is faster by up to three orders of magnitude and an average factor of 12. It also outperforms QSYM, a system that recently showed great performance improvements over other implementations, by up to two orders of magnitude and an average factor of 10. Using it on real-world software, we found that our approach consistently achieves higher coverage, and we discovered two vulnerabilities in the heavily tested OpenJPEG project, which have been confirmed by the project maintainers and assigned CVE identifiers.},
  booktitle = {Proceedings of the 29th USENIX Conference on Security Symposium},
  articleno = {11},
  numpages = {18},
  series = {SEC'20}
  }
---

## Summary

## Key Contributions

## Notes

