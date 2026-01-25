# "Focusing on Pattern Matching"
## Neelakantan R. Krishnaswami, Carnegie Mellon University, neelk@cs.cmu.edu
#### link: https://www.cl.cam.ac.uk/~nk480/pattern-popl09.pdf
#### tag: neelk-popl09

We often think of pattern matching as secondary--something desugared
by a compiler pass into the fundamental connectives of the type theory
(logic, etc.) via elaboration into usages of projections,
let-elimination, etc. This paper asks: what if we wanted to build
pattern matching into the rules of the type theory? However, there is
a more important message in this paper than that superficial
goal. Instead, we can think about the structure of these proof systems
as being operationally-minded, in terms of canonicalizing the proof
search in a very relevant manner.

When we think about a traditional proof system, we often specify it
via natural deduction rules, e.g., from 3GIp (third Gentzen system for
intuitionistic logic) we have two *right* rules (which manipulate
things on the right side of the turnstile) for ∨, which we would think
of (from the perspective of elementary natural deduction) as the
introduction forms:

```
∨R:    Γ ⊢ A        Γ ⊢ B
      ---------   ----------
      Γ ⊢ A ∨ B   Γ ⊢ A ∨ B
```

The issue is that the way the rules are written, if we want to *prove*
`A ∨ B` on the right, we need to know *which* of `A` or `B` is
true. For example, to prove `B ⊢ (A ∨ B) ∨ C`, we need to first look
at `(A ∨ B)`, *then* at `B`. However, when we initially start with the
goal `(A ∨ B) ∨ C`, we face an obvious issue: we could apply either
the first or second of the right rules, which mirrors the genuine
branching we would be doing in (say) a SAT solver. But--and here's the
crucial part--what I just sketched is simply an intuitive explanation
of how to *operationalize* the rules efficiently. If I am actually
pedantic about it, I might need to say: apply every possible rule, at
every possible time, in every possible manner. This would come at the
cost of massive blowup for the following reason: in the case of
*invertible* rules, we gain no new information and change provability
in absolutely no way. For example, if we consider the rule:

```
  Γ, A ⊢ B
  ---------
  Γ ⊢ A → B 
```

In this case, when you see an `A → B` on the right, the *only* way to
prove it is to bring A in on the left and prove that (if `A` ends up
being unimportant, it doesn't matter--the structural rules tell us
this). The rule can be read *bidirectionally*, either forward or
backwards. This is because the rule is *invertible*. Invertibility is
tightly related to pattern matching: it captures exactly those steps
of computation where we can safely and eagerly decompose data without
committing to a branch. This observation underlies focusing: proof
search can be organized so that all invertible rules are applied
eagerly, leaving only the genuinely choice---adding rules to change
between "modes" precisely where pattern matching needs to make
decisions.

So instead of elementary natural deduction where you could attempt to
apply any rule at any time and possibly fail (e.g., failing to unify
the head with the goal or failing down the line at one of the
premises), we want an *ordered* system to avoid uselessly doing
duplicate work. From the paper: 

> Andreoli introduced the concept of focusing (Andreoli 1992) as a
> technique to reduce the nondeterminism in the sequent
> calculus. First, he observed (as had others (Miller et al. 1991))
> that applications of invertible rules could be done eagerly, since
> inversion does not change provability. Then, he observed that each
> connective in linear logic was either invertible on the left and
> noninvertible on the right (the positive types), or invertible on
> the right and non-invertible on the left (the negative
> types). Finally, he made the remarkable observation that in a fully
> inverted sequent (i.e., one in which no further inversions are
> possible), it is possible to selecta single hypothesis (on either
> the left or the right) and then eagerly try to prove it, *without
> losing completeness.*

## The point of focusing

The reason we do focusing is to give a canonical structure for
proofs. Proofs in focused sequent calculi are *normal forms* in the
sense that there is never any ambiguity which rule to apply. This is
another important and subtle point which was not immediately clear to
me: while `Γ` is a multiset in traditional sequent calculi, here we
can think of the various formulas which we might need to branch on as
being *ordered*, so as to ensure that branching in the proof system
doesn't have to happen in a disorganized way: in practice, it may be
quite desirable to have heuristics control branch exploration,
however.

## Sec 2.1: Basic intuitionistic system

```
Types A ::= X | 1 | A × B | A → B | 0 | A + B
Positives P ::= X | 1 | A × B | 0 | A + B
Negatives N ::= X | A → B
```

There are **FOUR** different judgments:

- There is a *right focus* phase: Γ ⊢ A

```
     Γ⊢A  Γ⊢B           Γ⊢A        Γ⊢B        Γ;·⊢N
──── ─────────          ────       ────       ─────
Γ⊢1  Γ⊢A×B              Γ⊢A+B      Γ⊢A+B      Γ⊢N
```

- There is a *right-inversion* phase: Γ, Δ ⊢ A
  - The only relevant rule is inversion on →

```
→R      BLURL
Γ;Δ,A⊢B Γ;Δ▷P
─────── ──────
Γ;Δ⊢A→B Γ;Δ⊢P
```

- Followed by *left-inversion* to a fixed point: Γ, Δ ▷ P
  - The context Δ is *ordered* and structural rules *do not apply*
  - This is the *pattern matching phase* 

```
HypL     1L          ×L              0L
Γ,N;Δ▷P  Γ;Δ▷P       Γ;A,B,Δ▷P
───────  ───────     ─────────
Γ;N,Δ▷P  Γ;1,Δ▷P     Γ;A×B,Δ▷P        Γ;0,Δ▷P


+L                      FocusR
Γ;A,Δ▷P  Γ;B,Δ▷P        Γ⊢P
───────────────         ─────
Γ;A+B,Δ▷P               Γ;·▷P


FocusL                  FocusLP
Γ▷X                     Γ▷P   Γ;P▷Q
─────                   ───────────
Γ;·▷X                   Γ;·▷Q
```

- Finally, we have the *left focusing* phase: `Γ▷A`

```
HYP          →L
A∈Γ          Γ▷A→B  Γ⊢A
────         ───────────
Γ▷A          Γ▷B
```

## Soundness is trivial, completeness is harder

- Every focused sequent calculus proof can easily be projected to one
  in the traditional sequent calculus: just throw away the blurs, etc.

- Proving completeness is interesting and harder: "The completeness
proof for focusing with respect to the sequent calculus is a little
bit harder, and can be found elsewhere (Liang and Miller 2007)." (@Sec
2.1)

## Proof Terms / Programming

```
Positive Right e ::= 〈〉 | 〈e, e′〉 | inl e | inr e | u
Negative Right u ::= λp. u | r
Arms (Positive Left) r ::= [] | [r | r′] | e | t | case(t, p ⇒ r)
Applications (Neg. Left) t ::= x | t e
Patterns p ::= x | 〈〉 | 〈p, p′〉 | [] | [p | p′]
Ordinary Contexts Γ ::= · | Γ, x : N
Pattern Contexts ∆ ::= · | ∆, p : A
```
