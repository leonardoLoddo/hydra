# Changelog

All notable changes to Hydra will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.1.0 (2026-08-17)


### ⚠ BREAKING CHANGES

* **config:** Development configurations containing $schema must remove that field before Hydra can load them.
* **config:** Version 1 project configuration and Git-common heads.json state are no longer supported. Recreate existing development fixtures with hydra init.

### Features

* add installable Hydra agent skill ([f2cc84e](https://github.com/leonardoLoddo/hydra/commit/f2cc84e6ca326cfb8652a181a7f5a5e46e206812))
* **art:** add initial hydra-art.txt file with artwork representation ([95c31df](https://github.com/leonardoLoddo/hydra/commit/95c31df3f223492945e65d3b863c49a93bd1cd1f))
* **cli:** add dynamic shell completions ([a2e9fcb](https://github.com/leonardoLoddo/hydra/commit/a2e9fcbf4560ce9669051f797e8aa15250fbb669))
* **cli:** implement project initialization ([53bb707](https://github.com/leonardoLoddo/hydra/commit/53bb70729f238cc8be6bf67fd23147a7e361455e))
* **cli:** manage optional Codex skill ([9ee1d3c](https://github.com/leonardoLoddo/hydra/commit/9ee1d3c2aa78059852b0fa0d169cced0e3c1c82a))
* **config:** add editor-facing JSON schema ([3468f86](https://github.com/leonardoLoddo/hydra/commit/3468f86f4cd780ae1071625250e385a8187ab326))
* **config:** add initial .hydra.json configuration file ([fec1a20](https://github.com/leonardoLoddo/hydra/commit/fec1a20fd04e0c4e68712261ffc8beb080744cbc))
* **config:** establish portable Head directory ownership ([fb2bf4b](https://github.com/leonardoLoddo/hydra/commit/fb2bf4b124705670acbe7031207d0b0640dfa635))
* **core:** harden initialization and verify storage ([abfcbec](https://github.com/leonardoLoddo/hydra/commit/abfcbec97fdf2010ce376cdf8063d355f6eaa29d))
* **core:** implement guided state repair ([c00c390](https://github.com/leonardoLoddo/hydra/commit/c00c390a84b2297b4a218b810ecfdc260553b54c))
* **doctor:** add storage diagnostics ([16c64cd](https://github.com/leonardoLoddo/hydra/commit/16c64cdf8c904949ce89670689ae10abd9e4cf1c))
* **head-close:** integrate through clean target worktrees ([e9bc168](https://github.com/leonardoLoddo/hydra/commit/e9bc1682c4d22c4b5b3354570c03f73abdf55906))
* **head:** add configurable close adapter ([953737c](https://github.com/leonardoLoddo/hydra/commit/953737c33f241f3595d2c537a476a6d027a874be))
* **head:** add read-only Head inspection ([b329473](https://github.com/leonardoLoddo/hydra/commit/b329473fa35d9f7086fc0911d2a346305b90f72d))
* **head:** adopt manifest-backed untracked heads ([730088f](https://github.com/leonardoLoddo/hydra/commit/730088f0053b04b9a5b1908e5a89c6ac95e5d5d5))
* **head:** implement configured Head opening ([31a7f9b](https://github.com/leonardoLoddo/hydra/commit/31a7f9b5380054d02929f907861d432e9bb9014c))
* **head:** implement head creation ([6b17959](https://github.com/leonardoLoddo/hydra/commit/6b17959b91b5474354f4bace916cf846634fbd93))
* **head:** implement isolated Head close ([78ec871](https://github.com/leonardoLoddo/hydra/commit/78ec871d231ed728fd5667c07bc3fa5a25404fed))
* **head:** implement protected Head removal ([1e53892](https://github.com/leonardoLoddo/hydra/commit/1e53892838061b0b1cc0782d39ba6de7e23969cc))
* **head:** make creation feedback context-aware ([e6e71f9](https://github.com/leonardoLoddo/hydra/commit/e6e71f9fe997786ed386b494b603925fa540a9e2))
* **head:** recover abandoned state locks ([937b5a7](https://github.com/leonardoLoddo/hydra/commit/937b5a73d03843d197981d96bb84d3d45b985375))
* **head:** recover from lost private manifests ([ddce1b4](https://github.com/leonardoLoddo/hydra/commit/ddce1b4c944f2a47627172968db90ccc13e83f69))
* **head:** recover interrupted pre-worktree creation ([827208e](https://github.com/leonardoLoddo/hydra/commit/827208eb50c1199e9d0feaad4c4e42a22120abd8))
* **head:** recover missing inventory from manifests ([7200675](https://github.com/leonardoLoddo/hydra/commit/720067596de7143834a237188ae817421832deb7))
* **init:** reuse empty owned Heads directories ([2307dfb](https://github.com/leonardoLoddo/hydra/commit/2307dfb90bf0b9619c8b9685e03301f09b329a9a))
* **overlay:** offer exclusion of unsafe symlinks ([fb31278](https://github.com/leonardoLoddo/hydra/commit/fb312783fbaed6d854e057e87dee724805fe1b48))
* **storage:** add deterministic full-copy mode ([6c3c201](https://github.com/leonardoLoddo/hydra/commit/6c3c201190a0f4b5aadbc7f01925549c70dba80d))
* stored cool ascii art ([adab0cd](https://github.com/leonardoLoddo/hydra/commit/adab0cd9d95293830ce4b01b9932f8dfac2f7ec3))


### Bug Fixes

* **ci:** test Homebrew Formula from a temporary tap ([5f78968](https://github.com/leonardoLoddo/hydra/commit/5f78968d0555379f78a50c158ed179ab6479f44c))
* **cli:** expose nested command syntax ([2490c94](https://github.com/leonardoLoddo/hydra/commit/2490c94bafe83dc838326776bb635f217f282f37))
* **config:** remove unpublished JSON Schema support ([a9fe010](https://github.com/leonardoLoddo/hydra/commit/a9fe010df68d0b9e9d32903b38de4e4bf3b84710))
* **head:** clarify ref and policy diagnostics ([d235679](https://github.com/leonardoLoddo/hydra/commit/d23567944362cba5f186db38f944511541eb359d))
* **head:** report observed worktree state accurately ([85552df](https://github.com/leonardoLoddo/hydra/commit/85552dfaeea7613f925afd32edfada51fba50028))
* **head:** resolve commands through canonical parent ([2fb31d2](https://github.com/leonardoLoddo/hydra/commit/2fb31d2d7c68053a8e1aa517003c8148edd81843))
* **integration:** enhance native integration logic for isolated divergent histories ([41f1581](https://github.com/leonardoLoddo/hydra/commit/41f158121704963cdfc88cbaef54a904bc945b55))


### Performance Improvements

* **head:** accelerate complete Head creation ([0c71bfb](https://github.com/leonardoLoddo/hydra/commit/0c71bfb92809a23d4673908cef5464d235460c9d))

## [Unreleased]
