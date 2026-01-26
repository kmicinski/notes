---
title: Algorithms for Optimizing Acyclic Queries
date: 2026-01-26
type: paper
bib_key: wang:icdt26
bibtex: |
  @misc{luo2026algorithmsoptimizingacyclicqueries,
        title={Algorithms for Optimizing Acyclic Queries}, 
        author={Zheng Luo and Wim Van den Broeck and Guy Van den Broeck and Yisu Remy Wang},
        year={2026},
        eprint={2509.14144},
        archivePrefix={arXiv},
        primaryClass={cs.DB},
        url={https://arxiv.org/abs/2509.14144}, 
  }
---

## Summary

### Introductory Comments (Jan '26)

- First off: there's a methodological difference between *operator* trees and *join* trees. For example, you might have R ⋈ (Q ⋈ P) or (R ⋈ Q) ⋈ P--both of these implicitly materialize the result of an inner relation, which presumes you are materializing a subordinate result. By contrast, a join tree enables doing more optimal evaluation by avoiding intermediate tuple materialization, eliminating "dangling tuples" by using semijoins.

- The new innovations in this paper relate to providing new optimization strategies that focus on join trees rather than operator trees. Traditional (cost-based, I believe) estimators are tuned to work with the traditional operator-based plans.

## Key Contributions

- Hypothesis(?) Before this paper, there were no provably-good results for identifying good join trees for alpha-acyclic queries:
  - "First, we give an algorithm to enumerate all join trees of an alpha-acylic query by edits in linear time with amortized constant delay, which forms the basis of a coast-based optimizer for acylic joins." 
  - "Second, we show the Maximum Cardinality Search algorithm by Tarjan and Yannakakis constructs the unique *shallowest* join tree for any Berge-acylic query, thus enabling parallel execution of large join queries."
  - "Finally, we prove that a simple algorithm by Hu et al. converts any connected left-deep linear plan of a gamma-acylic query into a join tree, allowing reuse of optimizers developed for binary joins."
    -> (Question): This result seems compelling, because gamma-acylic plans are the most general of the class of acylic plans.

## Sec 1. Introduction

- A significant amount of focused has been placed upon optimizing binary joins, which are fundamentally destined to failure on some queries, even acylic queries. 

## Notes

