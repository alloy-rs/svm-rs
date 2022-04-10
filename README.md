# Solidity Compiler Version Manager

[<img alt="crates.io" src="https://img.shields.io/crates/v/svm-rs.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/svm-rs)
[<img alt="docs.rs" src="https://img.shields.io/docsrs/svm-rs/latest?color=66c2a5&label=docs-rs&logo=data%3Aimage%2Fsvg%2Bxml%3Bbase64%2CPHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K&style=for-the-badge" height="20">](https://docs.rs/svm-rs/latest/svm_lib/)
[<img alt="build status" src="https://img.shields.io/github/workflow/status/roynalnaruto/svm-rs/Rust/master?style=for-the-badge" height="20">](https://github.com/roynalnaruto/svm-rs/actions?query=branch%3Amaster)


### Install
```
$ cargo install svm-rs
```

### Usage
* List available versions
```
$ svm list
```
* Install a version
```
$ svm install <version>
```
* Use an installed version
```
$ svm use <version>
```
* Remove an installed version
```
$ svm remove <version>
```

### .svmrc

You can create a `.svmrc` file containing a solidity version number in the
project root directory (or any parent directory). Afterwards `solc` will use
the version specified in the `.svmrc` file.

For example, to make svm default to the solc v0.8.13:

    $ echo "0.8.13" > .svmrc

Then when you run `solc`:

    $ solc --version
    solc, the solidity compiler commandline interface
    Version: 0.8.13+commit.abaa5c0e.Darwin.appleclang

`solc` will traverse directory structure upwards from the current directory
looking for the `.svmrc` file. In other words, running `solc` in any
subdirectory of a directory with an `.svmrc` will result in that `.svmrc`
being utilized.
