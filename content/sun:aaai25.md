---
title: Column-Oriented Datalog on the GPU
date: 2026-02-16
type: paper
bibtex: |
  @article{sun:aaai25, title={Column-Oriented Datalog on the GPU}, volume={39}, url={https://ojs.aaai.org/index.php/AAAI/article/view/33665}, DOI={10.1609/aaai.v39i14.33665}, abstractNote={Datalog is a logic programming language widely used in knowledge representation and reasoning (KRR), program analysis, and social media mining due to its expressiveness and high performance. Traditionally, Datalog engines use either row-oriented or column-oriented storage. Engines like VLog and Nemo favor column-oriented storage for efficiency on limited-resource machines, while row-oriented engines like Soufflé use advanced datastructures with locking to perform better on multi-core CPUs. The advent of modern datacenter GPUs, such as the NVIDIA H100 with its ability to run over 16k threads simultaneously and high memory bandwidth, has reopened the debate on which storage layout is more effective. This paper presents the first column-oriented Datalog engines tailored to the strengths of modern GPUs. We present VFLog, a CUDA-based Datalog runtime library with a column-oriented GPU datastructure that supports all necessary relational algebra operations. Our results demonstrate over 200x performance gains over SOTA CPU-based column-oriented Datalog engines and a 2.5x speedup over GPU Datalog engines in various workloads, including KRR.}, number={14}, journal={Proceedings of the AAAI Conference on Artificial Intelligence}, author={Sun, Yihao and Kumar, Sidharth and Gilray, Thomas and Micinski, Kristopher}, year={2025}, month={Apr.}, pages={15177-15185} }
doi: 10.1609/aaai.v39i14.33665
---

## Summary

## Key Contributions

## Notes

