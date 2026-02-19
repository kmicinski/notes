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

Overall a very instructive paper about the implementation of modern high-performance SQL servers, and reveals a very interesting observation about how SQL server is already Yannakakis optimal for acylic queries.

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

- So it seems like SQL server (and other DB engines, generally) are essentially iterator-based with blocks of rowgroups, where within a rowgroup they use some vectorization (easier due to known sizes of things, modulo some variability).

- HJ.next() implements two steps:
  -> 1) Opens the build side, creates a hash table and (optionally) bitmap filters, closes it
    -> Wow, it occurs to me that these hash tables are being re-created then thrown away? In Datalog would we not be constantly keeping these always?
  -> 2) Opens the probe, pushes down bitmaps into the probe subtree, and calls next() to pull batches from the probe subtree to probe against the hash table and emit matching rows. 

#### Bitmap Construction

The "Bitmap Construction" section is helpful. First, the optimizer is using histograms (how?) to decide if it's even worth doing bitmap construction--the system defers construction of the bitmap (line 4) to *runtime*, which can use dynamic stats gathered during execution. An essentil trick being used here is to store (alongside the bitmap, and maybe rowblocks too) min / max values (which come from a hash? Or the underlying data?) Is this a place where key skew and data distribution have an effect on the performance, I wonder?

#### Bitmap Probing 

"Once created, bitmaps are heuristically pushed as deep as possible into the probe-side subplan to drop non-matching rows at the earliest opportunity. When pushed down to the scan operators, the min/max values accompanying the bitmap achieve rowgroup elimation: each rowgroup keeps min/max stats of its underlying data chunk (yup, answering my before question). If the ranges are disjoint then the scan is that rowgroupo is completely bypassed I guess in the O.next() operator on line 5 in Fig 2.

Interestingly, it seems there are different kinds of filters available for use by the system.

Question: on line 7, we have "if hash_table.probe(t2)," is that a tuple or should it not be a set of tuples (should this not be a for t2 in hash_table.probe? Am I being naive?)

### Sec 3.2: It works because Bitmaps are Composable

While each of the individual HJ operators alone implements nothing but some extra bitmaps, the magic lies in the fact that they compose in SQL Server's pull-based execution. This would be great if I understood what pull-based execution meant but I do not. Luckily this section explains: the magic is this. Consider the query C ⋈ O ⋈ L. When we compute the subordinate join (O ⋈ L), we will do it like we always do: when we scan O, we're pre-filtering on the cascading c_custkey bitmap (line 7, Fig 3). Only qualifying rows are inserted into the hash table and a second bitmap filter for O on o_orderkey is built. As the final pipeline, the table L is scanned and pre-iltered by the o_orderkey bitmap--but crucially, this is also implicitly filtered on the c_custkey bitmap. 


#### Sec 3.3: No, actually it works because you can skip the top-down semijoin pass

So the bottom-up pass in SQL server is basically Yannakakis but also constructing hash tables on C and O\top = O left semijin C. But the last stage differs: Yannakakis says that we also need to to a top-down join, followed by the full join. SQL Server skips this, so what gives? In fact, it turns out that Bagdan et al. comes to the rescue, giving us some results on output enueration. 

Unfortunately I tried a bit to follow the instance optimality proof, but I was not really able to do to well at this.

##### Algorithm 1: You can implement YA just by using SQL Server's HJ

- Lines 3/4 crucial here.

### Section 3.4: It also works for cyclic joins!? 

I did not really read this section, I only made it here and gave it a short skim.

### Performance evaluation

- What is inst. opt? and why would it be x? 

- Good results almost all of the time in general, results vary from 1.2-~3-4x depending on the query, best on a line graph and least-good on antijoin & join. 

##### Monotonicity to the rescue

However, I do think I follow the root of the intuition behind the proof of instance optimality for Algorithm 1. The crux of the result lines on the assumption that the Bloom filter is good enough, which then leads to a central insight. "A key insight is that when joining (in a top-down order) between a parent and a child in the join tree (Line 5), every probe into the child's hash table is a hit because the parent table has already been semijoined by that child in the bottom-up phase. As such, starting from the root (reduced) table, the intermediate join results can only grow *monotonically* along the join tree until reaching the final output, bounding all intermediates by O(OUT).

##### Robustness

Also, a consequence of Algorithm 

## Questions

A lot of the assumptions in Section 3 regard having the Bloom filter be perfect. In general this is probably fair, but what about heavy key skew? Is that an issue? 

Stupid question: why was it that the discovery of bloom filters made this palatable to industry?


