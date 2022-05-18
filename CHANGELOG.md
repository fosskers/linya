# `linya` Changelog

## 0.3.0 (2022-05-18)

#### Added

- The `stderr` method for `Progress`, which allows one to print intermittent
  messages alongside the progress bars.

## 0.2.2 (2022-02-21)

#### Fixed

- An overflow panic that affected certain hardware with a comparatively small `usize::MAX`.

## 0.2.1 (2021-06-12)

#### Added

- An example of how to use Linya without Rayon.

#### Changed

- Removed `Arc` usage in Rayon examples.

#### Fixed

- A rare panic under a very specific shell piping situation.

## 0.2.0 (2021-05-27)

#### Added

- A `Default` instance for `Progress`.
- A proper LICENSE file.

## 0.1.1 (2020-12-19)

#### Changed

- Progress bars are now written to standard error instead of standard output. [#1]

#### Fixed

- Some documentation inaccuracies.

[#1]: https://github.com/fosskers/linya/pull/1

## 0.1.0

Initial release.
