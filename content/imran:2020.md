---
title: Distributed Graph Analytics with Datalog Queries in Flink
date: 2026-02-19
type: paper
bibtex: |
  @InProceedings{imran:2020,
  author="Imran, Muhammad
  and G{\'e}vay, G{\'a}bor E.
  and Markl, Volker",
  editor="Qin, Lu
  and Zhang, Wenjie
  and Zhang, Ying
  and Peng, You
  and Kato, Hiroyuki
  and Wang, Wei
  and Xiao, Chuan",
  title="Distributed Graph Analytics with Datalog Queries in Flink",
  booktitle="Software Foundations for Data Interoperability and Large Scale Graph Data Analytics",
  year="2020",
  publisher="Springer International Publishing",
  address="Cham",
  pages="70--83",
  abstract="Large-scale, parallel graph processing has been in demand over the past decade. Succinct program structure and efficient execution are among the essential requirements of graph processing frameworks. In this paper, we present Cog, which executes Datalog programs on the Apache Flink distributed dataflow system. We chose Datalog for its compact program structure and Flink for its efficiency. We implemented a parallel semi-naive evaluation algorithm exploiting Flink's delta iteration to propagate only the tuples that need to be further processed to the subsequent iterations. Flink's delta iteration feature reduces the overhead present in acyclic dataflow systems, such as Spark, when evaluating recursive queries, hence making it more efficient. We demonstrated in our experiments that Cog outperformed BigDatalog, the state-of-the-art distributed Datalog evaluation system, in most of the tests.",
  isbn="978-3-030-61133-0"
  }
---

## Summary

## Key Contributions

## Notes

