---
title: Parallel Chopped Symbolic Execution
date: 2026-02-16
type: paper
bibtex: |
  @InProceedings{par-chopped,
  author="Singh, Shikhar
  and Khurshid, Sarfraz",
  editor="Lin, Shang-Wei
  and Hou, Zhe
  and Mahony, Brendan",
  title="Parallel Chopped Symbolic Execution",
  booktitle="Formal Methods and Software Engineering",
  year="2020",
  publisher="Springer International Publishing",
  address="Cham",
  pages="107--125",
  abstract="Symbolic execution, a well-known and widely studied software testing technique, faces scalability issues due to path explosion that limits its effectiveness. Recent work on chopped symbolic execution introduced the Chopper technique that allows the user to specify uninteresting parts of code that the symbolic analysis can try to ignore by focusing first on the essential parts. If necessary, the ignored parts are later explored once their impact on the main code under analysis becomes unavoidable. We introduce a parallel approach to chopped symbolic execution that integrates path-based partitioning with Chopper. Our tool, called PChop, speeds up chopped symbolic exploration by allowing multiple participating workers to explore non-overlapping regions of the code in parallel. We demonstrate the impact of our technique in a failure reproduction scenario, where we use both PChop and Chopper to re-create security vulnerabilities in the GNU libtasn1. The experimental results show that PChop is beneficial in situations where Chopper requires more than a minute to find the vulnerability when using a specific search strategy. For two vulnerabilities, PChop identified a previously undocumented code location to manifest each of them.",
  isbn="978-3-030-63406-3"
  }
---

## Summary

## Key Contributions

## Notes

