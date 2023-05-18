# Solidity Compiler Version Manager

[<img alt="crates.io" src="https://img.shields.io/crates/v/svm-rs.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/svm-rs)
[<img alt="docs.rs" src="https://img.shields.io/docsrs/svm-rs/latest?color=66c2a5&label=docs-rs&style=for-the-badge" height="20">](https://docs.rs/svm-rs/latest/svm_lib/)
[<img alt="build status" src="https://img.shields.io/github/actions/workflow/status/roynalnaruto/svm-rs/ci.yml?branch=master&style=for-the-badge" height="20">](https://github.com/roynalnaruto/svm-rs/actions?query=branch%3Amaster)

### Install

From [crates.io](https://crates.io):

```sh
cargo install svm-rs
```

Or from the repository:

```sh
cargo install --locked --git https://github.com/roynalnaruto/svm-rs
```

### Usage

-   List available versions

```sh
svm list
```

-   Install a version

```sh
svm install <version>
```

-   Use an installed version

```sh
svm use <version>
```

-   Remove an installed version

```sh
svm remove <version>
```
