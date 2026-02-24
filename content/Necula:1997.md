---
title: Proof-Carrying Code
date: 2026-02-16
type: paper
bibtex: |
  @inproceedings{Necula:1997,
  author = {Necula, George C.},
  title = {Proof-Carrying Code},
  year = {1997},
  isbn = {0897918533},
  publisher = {Association for Computing Machinery},
  address = {New York, NY, USA},
  url = {https://doi.org/10.1145/263699.263712},
  doi = {10.1145/263699.263712},
  abstract = {This paper describes proof-carrying code (PCC), a mechanism by which a host system can determine with certainty that it is safe to execute a program supplied (possibly in binary form) by an untrusted source. For this to be possible, the untrusted code producer must supply with the code a safety proof that attests to the code's adherence to a previously defined safety policy. The host can then easily and quickly validate the proof without using cryptography and without consulting any external agents.In order to gain preliminary experience with PCC, we have performed several case studies. We show in this paper how proof-carrying code might be used to develop safe assembly-language extensions of ML programs. In the context of this case study, we present and prove the adequacy of concrete representations for the safety policy, the safety proofs, and the proof validation. Finally, we briefly discuss how we use proof-carrying code to develop network packet filters that are faster than similar filters developed using other techniques and are formally guaranteed to be safe with respect to a given operating system safety policy.},
  booktitle = {Proceedings of the 24th ACM SIGPLAN-SIGACT Symposium on Principles of Programming Languages},
  pages = {106–119},
  numpages = {14},
  location = {Paris, France},
  series = {POPL '97}
  }
doi: 10.1145/263699.263712
---

## Summary

## Key Contributions

## Notes

