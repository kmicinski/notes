---
title: The Size-Change Principle for Program Termination
date: 2026-02-16
type: paper
bibtex: |
  @article{size-change,
  author = {Lee, Chin Soon and Jones, Neil D. and Ben-Amram, Amir M.},
  title = {The Size-Change Principle for Program Termination},
  year = {2001},
  issue_date = {March 2001},
  publisher = {Association for Computing Machinery},
  address = {New York, NY, USA},
  volume = {36},
  number = {3},
  issn = {0362-1340},
  url = {https://doi.org/10.1145/373243.360210},
  doi = {10.1145/373243.360210},
  abstract = {The "size-change termination" principle for a first-order functional language with well-founded data is: a program terminates on all inputs if every infinite call sequence (following program control flow) would cause an infinite descent in some data values.Size-change analysis is based only on local approximations to parameter size changes derivable from program syntax. The set of infinite call sequences that follow program flow and can be recognized as causing infinite descent is an ω-regular set, representable by a B\"{u}chi automaton. Algorithms for such automata can be used to decide size-change termination. We also give a direct algorithm operating on "size-change graphs" (without the passage to automata).Compared to other results in the literature, termination analysis based on the size-change principle is surprisingly simple and general: lexical orders (also called lexicographic orders), indirect function calls and permuted arguments (descent that is not in-situ) are all handled automatically and without special treatment, with no need for manually supplied argument orders, or theorem-proving methods not certain to terminate at analysis time.We establish the problem's intrinsic complexity. This turns out to be surprisingly high, complete for PSPACE, in spite of the simplicity of the principle. PSPACE hardness is proved by a reduction from Boolean program termination. An ineresting consequence: the same hardness result applies to many other analyses found in the termination and quasitermination literature.},
  journal = {SIGPLAN Not.},
  month = {jan},
  pages = {81–92},
  numpages = {12},
  keywords = {PSPACE-completeness, termination, program analysis, partial evaluation, omega automaton}
  }
doi: 10.1145/373243.360210
---

## Summary

## Key Contributions

## Notes

