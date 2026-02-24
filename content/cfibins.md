---
title: Control flow integrity for COTS binaries
date: 2026-02-16
type: paper
bibtex: |
  @inproceedings{cfibins,
  author = {Zhang, Mingwei and Sekar, R.},
  title = {Control flow integrity for COTS binaries},
  year = {2013},
  isbn = {9781931971034},
  publisher = {USENIX Association},
  address = {USA},
  abstract = {Control-Flow Integrity (CFI) has been recognized as an important low-level security property. Its enforcement can defeat most injected and existing code attacks, including those based on Return-Oriented Programming (ROP). Previous implementations of CFI have required compiler support or the presence of relocation or debug information in the binary. In contrast, we present a technique for applying CFI to stripped binaries on \texttimes{}86/Linux. Ours is the first work to apply CFI to complex shared libraries such as glibc. Through experimental evaluation, we demonstrate that our CFI implementation is effective against control-flow hijack attacks, and eliminates the vast majority of ROP gadgets. To achieve this result, we have developed robust techniques for disassembly, static analysis, and transformation of large binaries. Our techniques have been tested on over 300MB of binaries (executables and shared libraries).},
  booktitle = {Proceedings of the 22nd USENIX Conference on Security},
  pages = {337–352},
  numpages = {16},
  location = {Washington, D.C.},
  series = {SEC'13}
  }

pdf: cfibins.pdf
---

## Summary

## Key Contributions

## Notes

