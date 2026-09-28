#!/usr/bin/env bash
#
# Settle a backlog item whose change has landed on main.
#
# Usage:
#     backlog/scripts/settle.sh <item-file> --pr <N> --body-file <path>
#
# The item file is the open one under backlog/ (for example
# backlog/20260915-rfc-search-prompt-recall.md). The pull request number is
# the change that settled it.
#
# The prose in --body-file becomes the body of the ## Outcome section, between
# the line naming the pull request and the line stating that the scope is
# unchanged. Write it fresh at the moment the item is settled: it records what
# was actually done, which is often not what the text above it predicted.
# Taking it from a file rather than the command line keeps the prose out of
# shell history, and taking it from a file rather than standard input makes the
# call read the same way every time and keeps the prose somewhere it can be
# read back before it is committed.
#
# The prose is required. An outcome with nothing to say about the change it
# settled is almost always a sign that it was written later, from the item's
# own text, rather than at the moment it landed.
#
# The script moves the item into backlog/done/, sets its Status, appends the
# ## Outcome section, commits, pushes, and deletes the merged branch. Run it
# from a clean working tree, either on main or on the branch the pull request
# came from; the script switches to main itself in the latter case.

set -euo pipefail

usage() {
    cat <<'EOF'
usage: settle.sh <item-file> --pr <N> --body-file <path>

  <item-file>        an open item under backlog/ (backlog/YYYYMMDD-<kind>-<slug>.md)
  --pr <N>           the pull request that settled it
  --body-file <path> file holding the body of the ## Outcome section; required
EOF
exit 2
}

main() {
    local item="" pr="" body_file="" body=""

    while [ $# -gt 0 ]; do
        case "$1" in
            --pr) pr="${2:-}"; shift 2 ;;
            --body-file) body_file="${2:-}"; shift 2 ;;
            -h|--help) usage ;;
            -*) usage ;;
            *) item="$1"; shift ;;
        esac
    done

    [ -n "$item" ] || usage
    [ -n "$pr" ] || usage
    [ -n "$body_file" ] || usage
    [ -f "$item" ] || die "no such item: $item"
    [ -f "$body_file" ] || die "no such body file: $body_file"

    body="$(cat "$body_file")"
    [ -n "$body" ] || die "the ## Outcome section has no body"

    local slug kind status action
    slug="$(basename "$item")"
    slug="${slug%.md}"

    case "$slug" in
        *-rfc-*) kind=rfc; status=accepted; action=Implemented ;;
        *-bug-*) kind=bug; status=fixed; action=Fixed ;;
        *) die "cannot tell rfc from bug in: $item" ;;
    esac

    local root repo
    root="$(git rev-parse --show-toplevel)"
    repo="$(gh repo view --json nameWithOwner -q .nameWithOwner)"

    cd "$root"
    settle "$item" "$slug" "$pr" "$repo" "$status" "$action" "$body"
}

settle() {
    local item="$1" slug="$2" pr="$3" repo="$4" status="$5" action="$6" body="$7"

    [ -z "$(git status --porcelain)" ] || die "the working tree is not clean"

    local merge sha head
    read -r merge head <<EOF
$(gh pr view "$pr" --repo "$repo" --json mergeCommit,headRefName -q '"\(.mergeCommit.oid) \(.headRefName)"')
EOF
    [ -n "$merge" ] && [ "$merge" != null ] || die "pull request #$pr has no merge commit"
    sha="$(git rev-parse --short "$merge")"

    # Landing a change usually leaves the merged branch checked out, so switch
    # to main rather than making the caller do it. Only where that branch is
    # the one being settled: anywhere else the script is being run by mistake,
    # and guessing which branch was meant would be worse than stopping.
    local branch
    branch="$(git rev-parse --abbrev-ref HEAD)"
    if [ "$branch" != main ]; then
        [ "$branch" = "$head" ] || die "run this on main or on $head, not on $branch"
        git switch --quiet main
    fi

    git fetch --quiet origin main
    git merge --quiet --ff-only origin/main

    local dest="backlog/done/$(basename "$item")"
    [ ! -e "$dest" ] || die "$dest already exists"

    grep -q '^- Status:' "$item" || die "no '- Status: ...' line in $item"

    # The file is rebuilt in one pass and moved afterwards: writing through
    # $dest directly would collide with git mv, which refuses to overwrite an
    # existing path.
    #
    # The Status line is replaced by matching the whole line rather than a
    # particular value, so this keeps working whatever the template lists as
    # the alternatives.
    local tmp="$item.tmp"
    {
        sed "s|^- Status:.*|- Status: $status|" "$item"
        printf '\n## Outcome\n\n'
        printf '%s in [#%s](https://github.com/%s/pull/%s) (merged as `%s`).\n' \
            "$action" "$pr" "$repo" "$pr" "$sha"
        printf '\n%s\n' "$body"
        printf '\nThe scope is unchanged from what is described above.\n'
    } > "$tmp"

    git mv "$item" "$dest" || die "could not move $item into done/"
    mv "$tmp" "$dest"

    git add "$dest"
    git commit --quiet -m "Move the $slug to done/"
    git push --quiet origin main

    if [ -n "$head" ] && [ "$head" != main ] && git rev-parse --verify --quiet "refs/heads/$head" >/dev/null; then
        git branch -D "$head"
    fi

    # Say what was done, then show the state a reader would check anyway: the
    # commit that just went out and whether the tree is back where it started.
    echo "settled $slug (PR #$pr, $sha)"
    echo
    git --no-pager log --oneline -1
    git --no-pager status --short --branch
}

die() {
    echo "settle.sh: $1" >&2
    exit 1
}

main "$@"
