---
title: I Can’t Believe It’s Not Yannakakis: Pragmatic Bitmap Filters in Microsoft SQL Server
date: 2026-02-16
type: paper
bibtex: |
  @inproceedings{zhaoCIDR26,
    title={I Can’t Believe It’s Not Yannakakis: Pragmatic Bitmap Filters in Microsoft SQL Server},
    author={Zhao, Hangdong and Tian, Yuanyuan and Alotaibi, Rana and Ding, Bailu and Bruno, Nicolas and Camacho-Rodr{\'\i}guez, Jes{\'u}s and Papadimos, Vassilis and Ju{\'a}rez, Ernesto Cervantes and Galindo-Legaria, Cesar and Curino, Carlo},
    booktitle={Proceedings of the Conference on Innovative Data Systems Research (CIDR)},
    year={2026}
  }
pdf: p29-zhao.pdf
---

## Summary

Overall a very instructive paper about the implementation of modern high-performance SQL servers, and reveals a very interesting 

- SQL server already captures Yannakakis algorithm as part of its design by using bitmaps in filters. This paper studies how it works.
- SQL server blends bitmap filters into a pull-based execution (Sec. 3) 
- In conclusion, SQL server implicitly considers all Yannakakis-style plans for acylic joins!

### 2.1 Yannakakis algorithm

In YA, we have the running intersection property: if we take any attribute and look at the set of relations containing that attribute, they have to be a connected part of the join tree we choose. 

- Notice that the algorithm is batch: pipe output from bottom up into top down (or vice versa).
- Semijoin, Semijoin, Full join

### 2.2 So good: why so bad?

- Predicate Transfer (PT) is the idea of replacing the semijoins of YA with Bloom filters [18, 32, 45], which can efficiently propagate across join predicates.
  -> "Efficiently propagate across join predicates?" -- I do not understand this.
  -> When do bloom filters _transfer_? I think I am missing something there. I understand a Bloom filter as a one-way probabilistic hashset.
  -> What are the _predicates_ in the predicate transfer technique?

- Diamond-Hardened Joins: "Birler et al. [8] advocates for a novel decomposition of hash joins into two operators: lookup and expand. Each lookup performs a semijoin while keeping iterators that allow a later expansion into full joins." So I guess the idea here is that you can kind of fuse iterators by exposing their workings. I get that intuitively after having read the diamond-hardened join paper. I think this is related to the observation that Remy (I believe) mentioned previously, that you can often optimize YA by eliding some phases.

- Pre-computed Data Structures: I didn't really understand this particular section too well.

## Sec 3: How does it all work?

Section 3 shows us why paying for SQL server has been a justified decision even dating back to 2012.

### Batch Mode Hash Join

- "Batch mode is SQL Server's vectorized execution model that organizes columnar data on disc into *rowgroups* (each holding ~900 rows), while at execution time, batch mode operators extract them into in-memory data *batches*, as opposed to operating on one row at a time, or row mode [25].
 
  -> I guess the key idea is that when database developers talk about "vectorizing," they generally mean a kind of iterator-based batching. I think there is an implicit idea of a push-and-pull understanding, where you ae working in chunks of 900 rows at a time. I m a bit confused on the row vs. column distinction here--since its columnar does that mean you've actually got 900 pairs of IDs plus datum in a rowgroup? Looks like they use vectorized instructions (AVX?) within a batch.

- "The (batch mode) hash join (HJ) is the sole execution primitive that unlocks the potential of SQL Server's bitmap filters..

- 

## Key Contributions

## Questions

Stupid question: why was it that the discovery of bloom filters made this palatable to industry?


