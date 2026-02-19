---
title: D
date: 2026-02-16
type: paper
bibtex: |
  @InProceedings{foerster2018,
      title = {{D}i{CE}: The Infinitely Differentiable {M}onte {C}arlo Estimator},
      author = {Foerster, Jakob and Farquhar, Gregory and Al-Shedivat, Maruan and Rockt{\"a}schel, Tim and Xing, Eric and Whiteson, Shimon},
      booktitle = {Proceedings of the 35th International Conference on Machine Learning},
      pages = {1529--1538},
      year = {2018},
      editor = {Jennifer Dy and Andreas Krause},
      volume = {80},
      series = {Proceedings of Machine Learning Research},
      address = {Stockholmsmässan, Stockholm Sweden},
      month = {10--15 Jul},
      publisher = {PMLR},
      pdf = {http://proceedings.mlr.press/v80/foerster18a/foerster18a.pdf},
      url = {http://proceedings.mlr.press/v80/foerster18a.html},
      abstract = {The score function estimator is widely used for estimating gradients of stochastic objectives in stochastic computation graphs (SCG), eg., in reinforcement learning and meta-learning. While deriving the first-order gradient estimators by differentiating a surrogate loss (SL) objective is computationally and conceptually simple, using the same approach for higher-order derivatives is more challenging. Firstly, analytically deriving and implementing such estimators is laborious and not compliant with automatic differentiation. Secondly, repeatedly applying SL to construct new objectives for each order derivative involves increasingly cumbersome graph manipulations. Lastly, to match the first-order gradient under differentiation, SL treats part of the cost as a fixed sample, which we show leads to missing and wrong terms for estimators of higher-order derivatives. To address all these shortcomings in a unified way, we introduce DiCE, which provides a single objective that can be differentiated repeatedly, generating correct estimators of derivatives of any order in SCGs. Unlike SL, DiCE relies on automatic differentiation for performing the requisite graph manipulations. We verify the correctness of DiCE both through a proof and numerical evaluation of the DiCE derivative estimates. We also use DiCE to propose and evaluate a novel approach for multi-agent learning. Our code is available at https://github.com/alshedivat/lola.}
  }
---

## Summary

## Key Contributions

## Notes

