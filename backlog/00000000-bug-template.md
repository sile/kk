# Bug: <title>

- Status: open | fixed | not-a-bug

## Summary

One paragraph. What is broken, and what is the smallest input or sequence that
shows it? An issue is a **bug** (rather than an RFC) only when the current
behavior cannot be defended under its own documented contract: input is
dropped, output is wrong, or a documented invariant is violated. If the current
behavior is defensible and you are arguing for a different API, write an RFC
instead.

A bug report is shorter than an RFC. It needs a reproduction, the observed
behavior, and the expected behavior. It does **not** need a full design
rationale; leave the fix to the pull request that settles it.

## Reproduction

The smallest exact input and the exact calls that show the problem, so that
someone can reproduce it without reading the implementation. Prefer a concrete
value (a key sequence, a specific command-line invocation, a specific file
that is opened) over a description.

```text
<exact input>
<exact call(s)>
<observed output>
```

If the behavior depends on how the input is split or fed, say so here; that is
often the difference between "does not decode" and "decodes only sometimes".

## Observed behavior

What actually happens, and where in `src/` the behavior comes from. Quote the
relevant test or branch rather than paraphrasing it. Include the invariant that
is violated (for example, "a cut leaves the cursor one column past the end of
the line").

## Expected behavior

What the documented contract says should happen instead. Cite the rustdoc,
invariant, or type whose promise the observed behavior breaks.

## Impact

Who is affected and how badly. Is this a correctness problem (a user cannot
get the right result) or an ergonomics one? A correctness problem that is
reproducible from the public API is a bug; an ergonomics problem is usually an
RFC. Note any resource effect (unbounded growth, a stuck loop).

## Notes

Anything that helps the fix without turning this into a design document:
related cases that share the same cause and should be fixed together, why a
simpler-looking fix is wrong, or whether resolving the ambiguity properly needs
an RFC. Leave this section out if there is nothing to add.
