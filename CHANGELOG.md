# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0] - 2021-05-12

### Changes

- Change subcommand `list` to `log`.
- Change subcommand `tasks` to `list`.
- Pretty print `show-manager` subcommand output.

## [0.4.0] - 2021-05-04

### Changed

- Improve outputs of `tasks` and `list` subcommands.
- Change database structure.

## [0.3.0] - 2021-04-28

### Added

- Add `show-manager` subcommand.
- Add `reset-manager` subcommand.

### Changed

- Color outputs.
- Ask for confirmation when using `delete` subcommand.

### Fixed

- Align `No` column of the task list.
- Prevent `start` subcommand from panicing when no task number is provided.

## [0.2.0] - 2021-04-18

### Added

- Add `CHANGELOG.md` file.

### Changed

- `--date` option can accept `YYYYMMDD` format in addition to `YYYY-MM-DD`.
- Refine output messages.
- Add `LICENSE` and `CHANGELOG.md` in the release package.
- Add more unit tests.
- Update `rusqlite` version to `0.25.0`.

### Fixed

- Fix the string representation of the negative duration.
- Fix a typo in `README.md`.

[Unreleased]: https://github.com/tomyukn/tasklog/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/tomyukn/tasklog/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/tomyukn/tasklog/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/tomyukn/tasklog/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/tomyukn/tasklog/compare/v0.1.0...v0.2.0
