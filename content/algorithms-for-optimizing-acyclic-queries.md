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
pdf: wang:icdt26.pdf
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

## Definition: Join Hypergraph
[Remy's blog](https://remy.wang/blog/join-tree.html)

**Intution**: a hypergraph is a *venn diagram* with vertices = variables in the query.

![Viz](https://remy.wang/blog/assets/hypergraph.svg)

The hypergraph of a query has:
- *Variables* (used in the query) as its vertices 
- *hyper-edges* connecting multiple variables when used in one of the atoms of the clause.

Given a conjunctive (Datalog) query Q, its associated hypergraph has a vertex vₓ for each variable x appearing in Q, and a hyperedge {vₓ, vᵧ, v_z, …} for each atom R(x, y, z, …) in the body of Q.
(Open Question): What do you do with negation? We're assuming a positive fragment? 
(Open Question): What if the query uses ground atoms?

### Formal Definition (Defn. 1)

A hypergraph H = (X,R,χ) consists of a set of vertices *X*, a set of hyperedges *R 

## Example: Star Queries

Consider the star queries, which are queries that look like this:

    Q(x,y,z,w) <-- R(x,y), S(y,z), T(y,w)

The general case is that we have N body clauses, all sharing some unique variable (`y` here):

    Q(…) <-- R₁(x, y₁), R₂(x, y₂), …, R_k(x, y_k)

If we draw the hypergraph:
 - One hyperedge per atom
 - All hyperedges intersect in x
 - Thus, we literally get a star: a diagram where `x` is in the center and there are y_k for each

In this case, if the arity of all relations is two, then the *hyperedges* are really just **edges**. However, if we have relations with arity >2, then we see the more general hypergraph structure and you have more of a venn diagram shape to the hypergraph.

## Definition: Join Tree

First, a join tree is *not* the join hypergraph, but it is related to the join hypergraph. A query is alpha-acylic if it has a join tree. A join tree is a decomposition of the query into a tree such that each atom in the query (I guess literal?) is connected in the tree, if we view the tree as a graph. The definition of connectedness here is a bit tough because I don't know if this is a directed graph or not? 

For example, for the query 

```
Q(a,b,c,d,e,f,g,h) :- R(a,c,d), S(b,c,d), W(d,h), U(e,g,h), V(f,g,h).
```

we have: 
```
      W(d,h)
     /      \
    R(a,c,d)  U(e,g,h)
    |        |
    S(b,c,d)  V(f,g,h)
```

> An equivalent definition requires that for any two nodes containing a variable x, the path between the nodes must all contain x. It's not hard to see this is the same as requiring all nodes containing x to be connected.

**Kris Intuition**: there are no places in the graph where x pops out of nowhere. 

## Notes

