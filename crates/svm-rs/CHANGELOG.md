# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.23](https://github.com/alloy-rs/svm-rs/releases/tag/v0.5.23) - 2026-01-06

### Other

- Use Official `linux-arm64` Binaries ([#182](https://github.com/alloy-rs/svm-rs/issues/182))

## [0.5.22](https://github.com/alloy-rs/svm-rs/releases/tag/v0.5.22) - 2025-12-04

### Bug Fixes

- [svm-builds] Handle prerelease solidity versions ([#179](https://github.com/alloy-rs/svm-rs/issues/179))

### Miscellaneous Tasks

- Release 0.5.22

## [0.5.21](https://github.com/alloy-rs/svm-rs/releases/tag/v0.5.21) - 2025-11-19

### Features

- Support solc prereleases ([#176](https://github.com/alloy-rs/svm-rs/issues/176))

### Miscellaneous Tasks

- Release 0.5.21

## [0.5.20](https://github.com/alloy-rs/svm-rs/releases/tag/v0.5.20) - 2025-11-18

### Features

- Add support for 0.8.31-pre ([#175](https://github.com/alloy-rs/svm-rs/issues/175))

### Miscellaneous Tasks

- Release 0.5.20
- Support for Windows ARM64 ([#174](https://github.com/alloy-rs/svm-rs/issues/174))
- Improve UnknownVersion error message ([#172](https://github.com/alloy-rs/svm-rs/issues/172))

## [0.5.19](https://github.com/alloy-rs/svm-rs/releases/tag/v0.5.19) - 2025-08-13

### Features

- Add aliases `ls`, `i`, and `rm` ([#168](https://github.com/alloy-rs/svm-rs/issues/168))

### Miscellaneous Tasks

- Release 0.5.19

## [0.5.18](https://github.com/alloy-rs/svm-rs/releases/tag/v0.5.18) - 2025-08-12

### Bug Fixes

- More robust nixos check ([#150](https://github.com/alloy-rs/svm-rs/issues/150))
- Filter for universal macos ([#121](https://github.com/alloy-rs/svm-rs/issues/121))
- Fix loading peer certificates for corporate networks ([#83](https://github.com/alloy-rs/svm-rs/issues/83))

### Dependencies

- Bump to edition 2024 ([#167](https://github.com/alloy-rs/svm-rs/issues/167))
- Bump to Rust 1.89 ([#166](https://github.com/alloy-rs/svm-rs/issues/166))
- [deps] Bump all deps ([#163](https://github.com/alloy-rs/svm-rs/issues/163))
- [deps] Cargo update ([#160](https://github.com/alloy-rs/svm-rs/issues/160))
- [deps] Breaking bumps ([#152](https://github.com/alloy-rs/svm-rs/issues/152))
- Bump fs4 version to 0.12 ([#147](https://github.com/alloy-rs/svm-rs/issues/147))
- [deps] Bumps ([#136](https://github.com/alloy-rs/svm-rs/issues/136))
- [deps] Bump all dependencies ([#131](https://github.com/alloy-rs/svm-rs/issues/131))
- [deps] Rename zip_next to zip ([#128](https://github.com/alloy-rs/svm-rs/issues/128))
- Update dependencies and lockfile ([#114](https://github.com/alloy-rs/svm-rs/issues/114))
- Merge pull request [#112](https://github.com/alloy-rs/svm-rs/issues/112) from alloy-rs/matt/bump-solc-0.8.24
- Merge pull request [#110](https://github.com/alloy-rs/svm-rs/issues/110) from alloy-rs/matt/patch-bump

### Features

- Make svm list work offline ([#164](https://github.com/alloy-rs/svm-rs/issues/164))
- 0.8.30 ([#162](https://github.com/alloy-rs/svm-rs/issues/162))
- 0.8.29 ([#157](https://github.com/alloy-rs/svm-rs/issues/157))
- Add headless arg to install ([#154](https://github.com/alloy-rs/svm-rs/issues/154))
- Add simple retry for text file busy ([#155](https://github.com/alloy-rs/svm-rs/issues/155))
- Update android-aarch64 repo link ([#153](https://github.com/alloy-rs/svm-rs/issues/153))
- Add android-aarch64 support ([#151](https://github.com/alloy-rs/svm-rs/issues/151))
- 0.8.28 ([#146](https://github.com/alloy-rs/svm-rs/issues/146))
- Improve release profile ([#145](https://github.com/alloy-rs/svm-rs/issues/145))
- 0.8.27 ([#142](https://github.com/alloy-rs/svm-rs/issues/142))
- [svm] Add which subcommand ([#137](https://github.com/alloy-rs/svm-rs/issues/137))
- Increase `solc` download timeout window to 10 minutes from 2 minutes ([#133](https://github.com/alloy-rs/svm-rs/issues/133))
- [solc] Support rustup-like version specifiers (`+x.y.z`) ([#125](https://github.com/alloy-rs/svm-rs/issues/125))
- Add support for 0.8.25 ([#118](https://github.com/alloy-rs/svm-rs/issues/118))

### Miscellaneous Tasks

- Release 0.5.18
- [meta] Add CHANGELOG.md generation ([#165](https://github.com/alloy-rs/svm-rs/issues/165))
- Cargo update, clippy ([#148](https://github.com/alloy-rs/svm-rs/issues/148))
- [solc] Use exec on unices ([#135](https://github.com/alloy-rs/svm-rs/issues/135))
- Solc 0.8.26 ([#129](https://github.com/alloy-rs/svm-rs/issues/129))
- Remove readme in manifests
- Improve data dir is not a directory error msg ([#126](https://github.com/alloy-rs/svm-rs/issues/126))
- [meta] Improve CI, use workspace.package ([#115](https://github.com/alloy-rs/svm-rs/issues/115))
- Release 0.3.5
- Solc 0.8.24

### Other

- Disable NixOS patching after 0.8.28 ([#159](https://github.com/alloy-rs/svm-rs/issues/159))
- Eliminate "text file busy" errors ([#140](https://github.com/alloy-rs/svm-rs/issues/140))
- Replace `zip` with `zip_next` ([#127](https://github.com/alloy-rs/svm-rs/issues/127))
- Merge pull request [#113](https://github.com/alloy-rs/svm-rs/issues/113) from alloy-rs/matt/0.305

### Performance

- Use `raw.githubusercontent.com` direct link to avoid 302 redirects ([#132](https://github.com/alloy-rs/svm-rs/issues/132))

### Refactor

- Split out lib.rs into more modules ([#117](https://github.com/alloy-rs/svm-rs/issues/117))
- Library ([#116](https://github.com/alloy-rs/svm-rs/issues/116))

## [0.3.4](https://github.com/alloy-rs/svm-rs/releases/tag/v0.3.4) - 2024-01-23

### Bug Fixes

- Fix various regression on latest refactor
- Fix race condition between instances of svm in setting up the SVM directory

### Dependencies

- Merge pull request [#98](https://github.com/alloy-rs/svm-rs/issues/98) from alloy-rs/matt/bump-version123123
- Bump version
- Merge pull request [#92](https://github.com/alloy-rs/svm-rs/issues/92) from alloy-rs/matt/bump-minor-versions
- Bump minor versions
- Merge pull request [#90](https://github.com/alloy-rs/svm-rs/issues/90) from alloy-rs/matt/bump-version1232
- Bump version
- Bump internal versions

### Features

- New linux bins
- Change shas for macOS
- Update macos URLs and set latest to 0.8.22

### Miscellaneous Tasks

- Release 0.3.4
- Remove todo
- Just lowercase version
- Cache all versions
- Lint
- Cli improvements
- Update versions
- Patch bumps
- Update test
- Update linux builds sha
- Update urls with 0.8.21 mac build sha
- Update urls
- Update tests & solc-builds url
- Replace url commit shas with correct org/shas

### Other

- Merge branch 'master' into matt/update-readme
- Merge branch 'master' into matt/update-readme
- Merge pull request [#108](https://github.com/alloy-rs/svm-rs/issues/108) from alloy-rs/matt/fix-various-issues
- Use doc comment
- Merge pull request [#102](https://github.com/alloy-rs/svm-rs/issues/102) from alloy-rs/evalir/misc-refactor
- Clippy
- Properly handle uninstalled, existing versions in use
- Clippy
- Merge pull request [#100](https://github.com/alloy-rs/svm-rs/issues/100) from alloy-rs/evalir/0.8.23
- Update silicon links
- Merge branch 'master' into evalir/0.8.23
- Merge pull request [#96](https://github.com/alloy-rs/svm-rs/issues/96) from nategraf/victor/fix-setup-svm-dir-race
- Merge branch 'master' into victor/fix-setup-svm-dir-race
- Update crates/svm-rs/src/lib.rs
- Merge pull request [#97](https://github.com/alloy-rs/svm-rs/issues/97) from alloy-rs/evalir/0.8.22
- Merge pull request [#89](https://github.com/alloy-rs/svm-rs/issues/89) from Evalir/evalir/0.8.21
- Merge pull request [#87](https://github.com/alloy-rs/svm-rs/issues/87) from x86y/feat_xdg
- Add a test for XDG compliance and backwards compatibility
- Comply with XDG spec
- Merge pull request [#86](https://github.com/alloy-rs/svm-rs/issues/86) from ethers-rs/v0.8.20
- Merge pull request [#81](https://github.com/alloy-rs/svm-rs/issues/81) from DaniPopes/split-crates
- Deprecate `sha2-asm` feature
- Split crates

[`dyn-abi`]: https://crates.io/crates/alloy-dyn-abi
[dyn-abi]: https://crates.io/crates/alloy-dyn-abi
[`json-abi`]: https://crates.io/crates/alloy-json-abi
[json-abi]: https://crates.io/crates/alloy-json-abi
[`primitives`]: https://crates.io/crates/alloy-primitives
[primitives]: https://crates.io/crates/alloy-primitives
[`sol-macro`]: https://crates.io/crates/alloy-sol-macro
[sol-macro]: https://crates.io/crates/alloy-sol-macro
[`sol-type-parser`]: https://crates.io/crates/alloy-sol-type-parser
[sol-type-parser]: https://crates.io/crates/alloy-sol-type-parser
[`sol-types`]: https://crates.io/crates/alloy-sol-types
[sol-types]: https://crates.io/crates/alloy-sol-types
[`syn-solidity`]: https://crates.io/crates/syn-solidity
[syn-solidity]: https://crates.io/crates/syn-solidity

<!-- generated by git-cliff -->
