# backlog

This directory holds design proposals (RFCs) and, in the future, other
unprocessed work items such as TODOs and bug reports for kk.

## Purpose

An RFC here records *how a decision was reached*, not *what the current
specification is*.

The authoritative specification is:

- the rustdoc comments in `src/` for the public API, and
- `README.md` for the crate's scope and boundaries.

RFCs exist so that future maintainers can reconstruct the reasoning behind a
change. They are not a spec and must not be treated as one. Code and `README.md`
must stand on their own and must **not** reference a file in `backlog/`; if a
decision matters to a reader, write it where the reader is looking instead.

The rule is one-way: `src/` and `README.md` must never point at a file under
`backlog/`, because those are the artifacts users read and they have to make
sense without it. The reverse is fine and expected — a proposal may cite
`src/` and `README.md` freely to explain the code and docs it wants to change.

## Layout

```
backlog/
  README.md                       # this file: process, naming, states
  00000000-rfc-template.md        # template for a new RFC
  00000000-bug-template.md        # template for a new bug report
  YYYYMMDD-rfc-slug.md            # an open RFC
  YYYYMMDD-bug-slug.md            # an open bug report
  done/
    .gitkeep                      # keeps the empty directory in git
    YYYYMMDD-rfc-slug.md          # a settled RFC
    YYYYMMDD-bug-slug.md          # a settled bug report
  scripts/
    settle.sh                     # settle a landed item
```

Nothing but items live directly under `backlog/` or `backlog/done/`: a file
there is always an item. Item *kind* is carried by a filename prefix (`rfc-`
and `bug-` today; `todo-` if such items are added later), not by a directory.
Keeping the tree shallow is intentional: an item is always one or two levels
deep, and support files live in their own directory rather than beside them.

An item is an **RFC** when the question is *what the API should be* — the
current behavior is defensible and the proposal argues for changing it. An item
is a **bug** when the current behavior cannot be defended at all under its own
documented contract (input dropped, output wrong, a documented invariant
violated). A bug report is shorter than an RFC: it needs a reproduction, the
observed behavior, and the expected behavior, not a full design rationale.

## Naming

`YYYYMMDD-<kind>-<slug>.md`

- `YYYYMMDD` is the date the item was created, e.g. `20260915`.
- `<kind>` is `rfc` or `bug`.
- `<slug>` is a short lower-case hyphenated summary, e.g. `search-prompt-recall`.

Examples: `20260915-rfc-search-prompt-recall.md`,
`20260915-bug-cut-leaves-cursor-past-end-of-line.md`

There is intentionally **no sequence number**. Items are not cited by number,
and neither `src/` nor `README.md` may reference them, so a stable identifier
buys nothing. The date prefix sorts the directory chronologically, which makes
it obvious at a glance which items are oldest. Collisions are avoided by the
slug; two items on the same day simply get different slugs. If two items would
share a date and a slug, they are really one item and should be merged.

Because no numbering is used, there is no counter file to maintain.

The templates are named `00000000-rfc-template.md` and
`00000000-bug-template.md`, not `template-rfc.md` and `template-bug.md`, so
that they sort ahead of every real item (digits sort before letters in git's
ordering, so a leading `t` would not). The `00000000` is a date placeholder
that no item will ever use; the `rfc-` and `bug-` kind prefixes keep the
templates consistent with the naming scheme above, and there is one template
per kind because a bug report is shaped differently from an RFC (see the
kind table above).

## States

An item has one of two states, expressed by its location:

| State  | Location          | Meaning                                        |
| ------ | ----------------- | ---------------------------------------------- |
| `open` | `backlog/`        | Being drafted or discussed.                    |
| `done` | `backlog/done/`   | Settled; no longer being worked on.            |

Whether a settled RFC was **accepted** or **rejected** is recorded in its
`Status` field and in an `## Outcome` section, which is written when the
proposal is settled and left out while it is open. It is deliberately not
split into separate directories:

- kk does not value long-term stability as a primary goal, so "accepted"
is not a permanent commitment. An accepted RFC may later be changed or even
removed, which makes an "accepted" label on an old RFC misleading rather than
informative.
- `accepted` and `rejected` are the same operational state: the discussion is
over and nobody is working on it. One `done/` directory expresses that; the
qualitative difference belongs in the text, where the reasoning lives.

`postponed` items stay `open` (in `backlog/`), because they are still
unresolved.

A **bug** is settled when it is fixed (moved to `done/` with the fix) or when
it is determined to be intended behavior (moved to `done/` with the reason it
is not a bug).

## Moving an item to `done/`

Use `git mv` so history follows the file:

```sh
git mv backlog/20260915-rfc-search-prompt-recall.md backlog/done/
```

Update the `Status` field and add the `## Outcome` section in the same commit,
so the item never sits in `done/` without saying what settled it. Run
`backlog/scripts/settle.sh`, which does all of it:

```sh
backlog/scripts/settle.sh backlog/20260915-rfc-search-prompt-recall.md --pr 27 --body-file outcome.md
```

It switches to `main`, fast-forwards it, sets `Status` from the item's kind
(`fixed` for a bug, `accepted` for an RFC), appends the outcome, moves the
file, commits, pushes, and deletes the merged branch. Run it from a clean
working tree, right after the pull request is merged. Either branch will do:
on `main` it just proceeds, and on the branch the pull request came from it
switches to `main` first. Any other branch is a mistake and stops the script.

Pass the prose of the outcome section as a file instead of retyping it
later: it is written at the moment of settling, when what actually landed is
still fresh, and it describes the change that happened rather than the one the
item predicted. There is no `## Outcome` while an item is open, so this write
is also the only edit the section ever gets.

The prose is required, and the script stops if the file is empty. An outcome
with nothing to say about the change it settled usually means it is being
reconstructed from the item's own text rather than recalled from the change.

The section has a fixed shape — the pull request that settled the item, the
prose, and a closing line — and the script writes the parts the item cannot
know (the pull request number and the merge commit) around your prose:

```markdown
## Outcome

Fixed in [#38](https://github.com/sile/kk/pull/38) (merged as `ed036a6`).

The search prompt now recalls the last query when it is reopened.

The scope is unchanged from what is described above.
```

That closing line is deliberate: an item describes the change it proposes, so
the outcome states whether anything else moved with it. Write the reason when
it did; leave the line alone when it did not.

`## Outcome` goes at the end of the file, after every other section. Existing
items vary in where they put it because the section was added by hand before
this script existed; new items all end with it.

## Pull requests

An RFC that is implemented reaches `main` through a pull request, like any
other change. The same one-way rule applies to it: a pull request must make
sense to someone who never opens `backlog/`.

Write the title and body as a description of the change itself, not of the
proposal process. Name what the change does and why, the way any other pull
request would. Do not mention that an RFC was accepted or rejected, and do
not cite a path under `backlog/`; a reviewer who wants the reasoning can
find it, and a reader who does not need it is not sent there.

A bug is fixed by an ordinary pull request that reproduces the bug in a test
and fixes it; the bug report itself is not part of the change's public story.

## Packaging

The `backlog/` directory is development material and is excluded from the
published crate. This is configured in `Cargo.toml`:

```toml
exclude = ["backlog/"]
```

Do not remove that line. It keeps items out of the crate tarball without
affecting what git tracks.
