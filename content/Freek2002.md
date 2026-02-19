---
title: Formally verified lifting of C-compiled x86-64 binaries
date: 2026-02-16
type: paper
bibtex: |
  @inproceedings{Freek2002,
  author = {Verbeek, Freek and Bockenek, Joshua and Fu, Zhoulai and Ravindran, Binoy},
  title = {Formally verified lifting of C-compiled x86-64 binaries},
  year = {2022},
  isbn = {9781450392655},
  publisher = {Association for Computing Machinery},
  address = {New York, NY, USA},
  url = {https://doi.org/10.1145/3519939.3523702},
  doi = {10.1145/3519939.3523702},
  abstract = {Lifting binaries to a higher-level representation is an essential step for decompilation, binary verification, patching and security analysis. In this paper, we present the first approach to provably overapproximative x86-64 binary lifting. A stripped binary is verified for certain sanity properties such as return address integrity and calling convention adherence. Establishing these properties allows the binary to be lifted to a representation that contains an overapproximation of all possible execution paths of the binary. The lifted representation contains disassembled instructions, reconstructed control flow, invariants and proof obligations that are sufficient to prove the sanity properties as well as correctness of the lifted representation. We apply this approach to Linux Foundation and Intel’s Xen Hypervisor covering about 400K instructions. This demonstrates our approach is the first approach to provably overapproximative binary lifting scalable to commercial off-the-shelf systems. The lifted representation is exportable to the Isabelle/HOL theorem prover, allowing formal verification of its correctness. If our technique succeeds and the proofs obligations are proven true, then – under the generated assumptions – the lifted representation is correct.},
  booktitle = {Proceedings of the 43rd ACM SIGPLAN International Conference on Programming Language Design and Implementation},
  pages = {934–949},
  numpages = {16},
  keywords = {Binary Analysis, Disassembly, Formal Verification},
  location = {San Diego, CA, USA},
  series = {PLDI 2022}
  }
doi: 10.1145/3519939.3523702
---

## Summary

## Key Contributions

## Notes

