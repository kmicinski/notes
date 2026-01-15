---
title: "Attention Is All You Need"
date: 2024-12-15
type: paper
bib_key: vaswani2017attention
authors: Vaswani, Shazeer, Parmar, Uszkoreit, Jones, Gomez, Kaiser, Polosukhin
venue: NeurIPS
year: 2017
bibtex: |
  @inproceedings{vaswani2017attention,
    title={Attention is all you need},
    author={Vaswani, Ashish and Shazeer, Noam and Parmar, Niki and Uszkoreit, Jakob and Jones, Llion and Gomez, Aidan N and Kaiser, {\L}ukasz and Polosukhin, Illia},
    booktitle={Advances in neural information processing systems},
    volume={30},
    year={2017}
  }
time:
  - date: 2024-12-15
    minutes: 120
    category: reading
    description: Initial read-through
  - date: 2024-12-18
    minutes: 90
    category: reading
    description: Detailed study of attention mechanism
---

# Attention Is All You Need

## Summary

This paper introduces the **Transformer architecture**, which relies entirely on self-attention mechanisms, dispensing with recurrence and convolutions.

## Key Contributions

1. **Self-Attention Mechanism**: The core innovation allowing the model to weigh the importance of different parts of the input.

2. **Multi-Head Attention**: Allows the model to jointly attend to information from different representation subspaces.

3. **Positional Encoding**: Since the model contains no recurrence, positional encodings are added to give the model information about token positions.

## Architecture Notes

The Transformer follows an encoder-decoder structure:

```
Input -> Encoder Stack -> Decoder Stack -> Output
```

Each encoder layer has:
- Multi-head self-attention
- Position-wise feed-forward network

## Personal Notes

This paper fundamentally changed how I think about sequence modeling. The elimination of recurrence allows for much better parallelization during training.

Connects to my broader project ideas in [@8264bc].

## Open Questions

- How does the attention mechanism scale to very long sequences?
- What are the memory implications of self-attention?
