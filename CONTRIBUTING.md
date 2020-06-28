# Contributing to cargo-criterion

## Ideas, Experiences and Questions

The easiest way to contribute to cargo-criterion is to use it and report your experiences, ask questions and contribute ideas. We'd love to hear your thoughts on how to make cargo-criterion better, or your comments on why you are or are not currently using it.

Issues, ideas, requests and questions should be posted on the issue tracker at:

https://github.com/bheisler/cargo-criterion/issues

## A Note on Dependency Updates

cargo-criterion does not accept pull requests to update dependencies unless specifically
requested by the maintaner(s). Dependencies are updated manually by the maintainer(s) before each
new release.

## Code

Pull requests are welcome, though please raise an issue for discussion first if none exists. We're happy to assist new contributors.

If you're not sure what to work on, try checking the [Beginner label](https://github.com/bheisler/cargo-criterion/issues?q=is%3Aissue+is%3Aopen+label%3ABeginner)

To make changes to the code, fork the repo and clone it:

`git clone git@github.com:your-username/cargo-criterion.git`

You'll probably want to install [gnuplot](http://www.gnuplot.info/) as well. See the gnuplot website for installation instructions.

Then make your changes to the code. When you're done, run the tests:

```
cargo test --all
cargo bench
```

It's a good idea to run clippy and fix any warnings as well:

```
cargo clippy --all
```

Finally, run Rustfmt to maintain a common code style:

```
cargo fmt --all
```

Don't forget to update the CHANGELOG.md file and any appropriate documentation. Once you're finished, push to your fork and submit a pull request. We try to respond to new issues and pull requests quickly, so if there hasn't been any response for more than a few days feel free to ping @bheisler.

Some things that will increase the chance that your pull request is accepted:

* Write tests
* Clearly document public methods
* Write a good commit message

## Code of Conduct

We follow the [Rust Code of Conduct](http://www.rust-lang.org/conduct.html).
