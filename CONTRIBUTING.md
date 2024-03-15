# Guidance for external contributors

Do you feel keen and able to help with Plonky2? That's great! We
encourage external contributions!

We want to make it easy for you to contribute, but at the same time we
must manage the burden of reviewing external contributions. We are a
small team, and the time we spend reviewing external contributions is
time we are not developing ourselves.

We also want to help you to avoid inadvertently duplicating work that
is already underway, or building something that we will not
want to incorporate.

First and foremost, please keep in mind that this is a highly
technical piece of software and contributing is only suitable for
experienced mathematicians, cryptographers and software engineers.

The Polygon Zero Team reserves the right to accept or reject any
external contribution for any reason, including a simple lack of time
to maintain it (now or in the future); we may even decline to review
something that is not considered a sufficiently high priority for us.

To avoid disappointment, please communicate your intention to
contribute openly, while respecting the limited time and availability
we have to review and provide guidance for external contributions. It
is a good idea to drop a note in our public Discord #development
channel of your intention to work on something, whether an issue, a
new feature, or a performance improvement. This is probably all that's
really required to avoid duplication of work with other contributors.

What follows are some more specific requests for how to write PRs in a
way that will make them easy for us to review. Deviating from these
guidelines may result in your PR being rejected, ignored or forgotten.


## General guidance for your PR

Obviously PRs will not be considered unless they pass our Github
CI. The Github CI is not executed for PRs from forks, but you can
simulate the Github CI by running the commands in
`.github/workflows/ci.yml`.

Under no circumstances should a single PR mix different purposes: Your
PR is either a bug fix, a new feature, or a performance improvement,
never a combination. Nor should you include, for example, two
unrelated performance improvements in one PR. Please just submit
separate PRs. The goal is to make reviewing your PR as simple as
possible, and you should be thinking about how to compose the PR to
minimise the burden on the reviewer.

Also note that any PR that depends on unstable features will be
automatically rejected. The Polygon Zero Team may enable a small
number of unstable features in the future for our exclusive use;
nevertheless we aim to minimise the number of such features, and the
number of uses of them, to the greatest extent possible.

Here are a few specific guidelines for the three main categories of
PRs that we expect:


### The PR fixes a bug

In the PR description, please clearly but briefly describe

1. the bug (could be a reference to a GH issue; if it is from a
   discussion (on Discord/email/etc. for example), please copy in the
   relevant parts of the discussion);
2. what turned out to the cause the bug; and
3. how the PR fixes the bug.

Wherever possible, PRs that fix bugs should include additional tests
that (i) trigger the original bug and (ii) pass after applying the PR.


### The PR implements a new feature

If you plan to contribute an implementation of a new feature, please
double-check with the Polygon Zero team that it is a sufficient
priority for us that it will be reviewed and integrated.

In the PR description, please clearly but briefly describe

1. what the feature does
2. the approach taken to implement it

All PRs for new features must include a suitable test suite.


### The PR improves performance

Performance improvements are particularly welcome! Please note that it
can be quite difficult to establish true improvements for the
workloads we care about. To help filter out false positives, the PR
description for a performance improvement must clearly identify

1. the target bottleneck (only one per PR to avoid confusing things!)
2. how performance is measured
3. characteristics of the machine used (CPU, OS, #threads if appropriate)
4. performance before and after the PR
