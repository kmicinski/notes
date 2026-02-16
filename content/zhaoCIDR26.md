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
---

## Summary

- SQL server already captures Yannakakis algorithm as part of its design by using bitmaps in filters. This paper studies how it works.
- SQL server blends bitmap filters into a pull-based execution (Sec. 3) 
- In conclusion, SQL server implicitly considers all Yannakakis-style plans for acylic joins!


## Key Contributions

## Questions

Stupid question: why was it that the discovery of bloom filters made this palatable to industry?



