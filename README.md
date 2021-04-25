# Autorebase

I work on a repo with a fast-moving `master` branch and it gets pretty tedious typing

```
git switch master
git pull
git switch add_tests
git rebase master
git switch implement_feature
git rebase master
git switch implement_another_feature
git rebase master
git switch fix_bug
git rebase master
...
```

This program tries to do that for you. If there's a conflict it tries to rebase the branch as far as it can without conflicts. It's not finished yet.

## Usage

    autorebase

That's it for now.

## Limitations

It probably won't be able to rebase branches that aren't trees, i.e. branches that contain merge commits.
