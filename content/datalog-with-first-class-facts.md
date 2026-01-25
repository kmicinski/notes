---
title: Datalog with First-Class Facts
date: 2026-01-24
type: paper
bib_key: micinski2024datalog
bibtex: |
@article{micinski2024datalog,
author = {Gilray, Thomas and Sahebolamri, Arash and Sun, Yihao and Kunapaneni, Sowmith and Kumar, Sidharth and Micinski, Kristopher},
title = {Datalog with First-Class Facts},
year = {2024},
issue_date = {November 2024},
publisher = {VLDB Endowment},
volume = {18},
number = {3},
issn = {2150-8097},
url = {https://doi.org/10.14778/3712221.3712232},
doi = {10.14778/3712221.3712232},
abstract = {Datalog is a popular logic programming language for deductive reasoning tasks in a wide array of applications, including business analytics, program analysis, and ontological reasoning. However, Datalog's restriction to flat facts over atomic constants leads to challenges in working with tree-structured data, such as derivation trees or abstract syntax trees. To ameliorate Datalog's restrictions, popular extensions of Datalog support features such as existential quantification in rule heads (Datalog*, Datalog∃) or algebraic data types (Souffl\'{e}). Unfortunately, these are imperfect solutions for reasoning over structured and recursive data types, with general existentials leading to complex implementations requiring unification, and ADTs unable to trigger rule evaluation and failing to support efficient indexing.We present DL∃!, a Datalog with first-class facts, wherein every fact is identified with a Skolem term unique to the fact. We show that this restriction offers an attractive price point for Datalogbased reasoning over tree-shaped data, demonstrating its application to databases, artificial intelligence, and programming languages. We implemented DL∃! as a system Slog, which leverages the uniqueness restriction of DL∃! to enable a communication-avoiding, massively-parallel implementation built on MPI. We show that Slog outperforms leading systems (Nemo, Vlog, RDFox, and Souffl\'{e}) on a variety of benchmarks, with the potential to scale to thousands of threads.},
journal = {Proc. VLDB Endow.},
month = nov,
pages = {651–665},
numpages = {15}
}

---

## Summary



## Key Contributions

## Notes

