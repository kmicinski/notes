---
title: Main-Memory Hash Joins on Multi-Core CPUs: Tuning to the Underlying Hardware
date: 2026-02-16
type: paper
bibtex: |
  @inproceedings{hash-join,
    	author = {Teubner, Jens and Alonso, Gustavo and Balkesen, Cagri and Ozsu, M. Tamer},
    	title = {Main-Memory Hash Joins on Multi-Core CPUs: Tuning to the Underlying Hardware},
    	year = {2013},
    	isbn = {9781467349093},
    	publisher = {IEEE Computer Society},
    	address = {USA},
    	url = {https://doi.org/10.1109/ICDE.2013.6544839},
    	doi = {10.1109/ICDE.2013.6544839},
    	abstract = {The architectural changes introduced with multi-core CPUs have triggered a redesign of main-memory join algorithms. In the last few years, two diverging views have appeared. One approach advocates careful tailoring of the algorithm to the architectural parameters (cache sizes, TLB, and memory bandwidth). The other approach argues that modern hardware is good enough at hiding cache and TLB miss latencies and, consequently, the careful tailoring can be omitted without sacrificing performance. In this paper we demonstrate through experimental analysis of different algorithms and architectures that hardware still matters. Join algorithms that are hardware conscious perform better than hardware-oblivious approaches. The analysis and comparisons in the paper show that many of the claims regarding the behavior of join algorithms that have appeared in literature are due to selection effects (relative table sizes, tuple sizes, the underlying architecture, using sorted data, etc.) and are not supported by experiments run under different parameters settings. Through the analysis, we shed light on how modern hardware affects the implementation of data operators and provide the fastest implementation of radix join to date, reaching close to 200 million tuples per second.},
    	booktitle = {Proceedings of the 2013 IEEE International Conference on Data Engineering (ICDE 2013)},
    	pages = {362–373},
    	numpages = {12},
    	series = {ICDE '13}
    }
doi: 10.1109/ICDE.2013.6544839
pdf: hash-join.pdf
---

## Summary

## Key Contributions

## Notes

