---
title: Scaling-up in-Memory Datalog Processing: Observations and Techniques
date: 2026-02-19
type: paper
bibtex: |
  @article{recstep,
  author = {Fan, Zhiwei and Zhu, Jianqiao and Zhang, Zuyu and Albarghouthi, Aws and Koutris, Paraschos and Patel, Jignesh M.},
      title = {Scaling-up in-Memory Datalog Processing: Observations and Techniques},
      year = {2019},
      issue_date = {February 2019},
      publisher = {VLDB Endowment},
      volume = {12},
      number = {6},
      issn = {2150-8097},
      url = {https://doi.org/10.14778/3311880.3311886},
      doi = {10.14778/3311880.3311886},
      abstract = {Recursive query processing has experienced a recent resurgence, as a result of its use in many modern application domains, including data integration, graph analytics, security, program analysis, networking and decision making. Due to the large volumes of data being processed, several research efforts across multiple communities have explored how to scale up recursive queries, typically expressed in Datalog. Our experience with these tools indicate that their performance does not translate across domains---e.g., a tool designed for large-scale graph analytics does not exhibit the same performance on program-analysis tasks, and vice versa.Starting from the above observation, we make the following two contributions. First, we perform a detailed experimental evaluation comparing a number of state-of-the-art Datalog systems on a wide spectrum of graph analytics and program-analysis tasks, and summarize the pros and cons of existing techniques. Second, we design and implement our own general-purpose Datalog engine, called RecStep, on top of a parallel single-node relational system. We outline the techniques we applied on RecStep, as well as the contribution of each technique to the overall performance. Using RecStep as a baseline, we demonstrate that it generally out-performs state-of-the-art parallel Datalog engines on complex and large-scale Datalog evaluation, by a 4-6X margin. An additional insight from our work is that it is possible to build a high-performance Datalog system on top of a relational engine, an idea that has been dismissed in past work.},
      journal = {Proc. VLDB Endow.},
      month = {Feb},
      pages = {695--708},
      numpages = {14}
  }
doi: 10.14778/3311880.3311886
---

## Summary

## Key Contributions

## Notes

