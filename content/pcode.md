---
title: A Formal Semantics for P-Code
date: 2026-02-16
type: paper
bibtex: |
  @InProceedings{pcode,
  author="Naus, Nico
  and Verbeek, Freek
  and Walker, Dale
  and Ravindran, Binoy",
  editor="Lal, Akash
  and Tonetta, Stefano",
  title="A Formal Semantics for P-Code",
  booktitle="Verified Software. Theories, Tools and Experiments.",
  year="2023",
  publisher="Springer International Publishing",
  address="Cham",
  pages="111--128",
  abstract="Decompilation is currently a widely used tool in reverse engineering and exploit detection in binaries. Ghidra, developed by the National Security Agency, is one of the most popular decompilers. It decompiles binaries to high P-Code, from which the final decompilation output in C code is generated. Ghidra allows users to work with P-Code, so users can analyze the intermediate representation directly. Several projects make use of this to build tools that perform verification, decompilation, taint analysis and emulation, to name a few. P-Code lacks a formal semantics, and its documentation is limited. It has a notoriously subtle semantics, which makes it hard to do any sort of analysis on P-Code. We show that P-Code, as-is, cannot be given an executable semantics. In this paper, we augment P-Code and define a complete, executable, formal semantics for it. This is done by looking at the documentation and the decompilation results of binaries with known source code. The development of a formal P-Code semantics uncovered several issues in Ghidra, P-Code, and the documentation. We show that these issues affect projects that rely on Ghidra and P-Code. We evaluate the executability of our semantics by building a P-Code interpreter that directly uses our semantics. Our work uncovered several issues in Ghidra and allows Ghidra users to better leverage P-Code.",
  isbn="978-3-031-25803-9"
  }
---

## Summary

## Key Contributions

## Notes

