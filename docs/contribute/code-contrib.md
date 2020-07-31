# Code contributions

We greatly appreciate any code contributions even if it is just a small typo fix in the documentation. You can take a look at the issue tracker for [Available tasks](https://github.com/TilBlechschmidt/WebGrid/labels/Status%3A%20Available) which already have a solution outlined. If you are unsure on how to implement a change just ask, we are there to guide you through the process.

You can also take a look at the list of [Accepted tasks](https://github.com/TilBlechschmidt/WebGrid/labels/Status%3A%20Accepted). These are accepted changes that do not have a sketch on how to solve them yet but if you are interested in approaching one we are going to assist you in solving it! Just comment on the issue of interest to let us know.

!!! tip
    Did you know you can edit any documentation page directly just by clicking the edit button on the top right of each page? Please feel free to do so if you have found areas of improvement!

## Getting to know the project

It can be a daunting task to get into a new project, we encountered it ourselves more than we'd like to admit. For this reason a comprehensive guide to the project structure, local developer setup and other topics is available in the [Architecture tab](../architecture/index.md).

We strive to make the onboarding experience as simple and straightforward as possible so if you have any questions or ideas for improving it [please open a ticket](https://github.com/TilBlechschmidt/WebGrid/issues/new/choose)!

## Conventions

This project follows a few conventions regarding code contributions — below is a list of them.

### Commit messages

Your commit messages should always be imparative, capitalized, without any punctuation at the end and able to complete the following sentence:

```
If applied, this commit will <your-commit-message>.
```

You can read more about how to write good commit messages and why its important [over here](https://chris.beams.io/posts/git-commit/). This blog post is used as a guideline for this repository!

#### Gitmoji

All commit messages should be prefixed with a GitHub Emoji that describes in one character what the commit is all about. For reference you should take a look at the [gitmoji page](https://gitmoji.carloscuesta.me)!

#### PGP signing

Every commit must be PGP signed. This can be achieved by either editing files directly on GitHub or [setting up commit signing locally](https://git-scm.com/book/en/v2/Git-Tools-Signing-Your-Work) (make sure to upload your public key to GitHub if you sign locally).

## Workflow

Below are steps which are usually followed for code contributions — use them to get acquainted to the process before contributing or if you are unsure on what follows next.

### 1. Solution sketch

At the beginning of each code contribution a solution sketch has to exist. For some tasks like typo fixes this is a no-brainer but more complex tasks require some discussion up-front on how to approach a problem. This ensures that the result is compliant with project standards and doesn't create unexpected problems down the road!

For issues from the `Available` category a solution sketch is already provided which makes it even easier to get started!

### 2. Implementing changes

Once a solution sketch has been outlined you can start working on the code. Make sure to notify the maintainers so that the issue can be moved to the corresponding lifecycle stage preventing duplicate work.

At this stage you [setup your development environment](./dev-environment.md) (which may or may not be required depending on the changes as e.g. documentation changes can usually be done online using the GitHub Web Editor) and write code. If you have any questions ask on your issue ticket or submit a Draft Pull Request and comment on it if you need code-level assistance!

### 3. Code review

If you are done with your changes or want to seek feedback from maintainers you push your changes and open a Pull Request. Make sure to add the original issue number to the description so it can be associated later. Take special considerations in naming your PR as it will be published in the Changelog 😉

The PR will be reviewed by other project members and automatic tests are run against your code. If everything is well, the changes will be approved and merged into the main branch. However, it is very common that changes are requested — don't feel bad about it, we provide you feedback to improve your already amazing contribution. That brings you back to the previous stage of [Implementing changes](#2-implementing-changes).

Once your changes have been approved, they are merged into the `main` branch and published during the next [release cycle](./release.md)!