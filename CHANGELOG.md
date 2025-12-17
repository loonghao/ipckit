# Changelog

## [0.1.5](https://github.com/loonghao/ipckit/compare/ipckit-v0.1.4...ipckit-v0.1.5) (2025-12-17)


### Features

* add EventStream and TaskManager Python bindings, update docs with FastAPI/Robyn integration ([c0c6584](https://github.com/loonghao/ipckit/commit/c0c65842f49407e3adf45b1a28100d1a07564f85))
* add Python bindings for ChannelMetrics and API Server ([0ac98e9](https://github.com/loonghao/ipckit/commit/0ac98e91575928c1d553da01ba2724efb6434845))
* add timeout support for ApiClient and SocketClient ([6d654b7](https://github.com/loonghao/ipckit/commit/6d654b7b4bd6db7e0189d0a7964e8670c10e4361))


### Bug Fixes

* use safe buffer size for unbounded async channel ([2a82f11](https://github.com/loonghao/ipckit/commit/2a82f111ef61018d1e2f52b35a469a3166e08b7d))

## [0.1.4](https://github.com/loonghao/ipckit/compare/ipckit-v0.1.3...ipckit-v0.1.4) (2025-12-16)


### Features

* implement CLI Bridge for CLI tool integration (Issue [#17](https://github.com/loonghao/ipckit/issues/17)) ([e20376f](https://github.com/loonghao/ipckit/commit/e20376f4327d8829c08778042c291f4fedab8b0b))


### Bug Fixes

* add delay in test_multiple_bridges to ensure unique task IDs ([4e53508](https://github.com/loonghao/ipckit/commit/4e53508a3c5c1549b4ac5c11049cf93e788388c8))

## [0.1.3](https://github.com/loonghao/ipckit/compare/ipckit-v0.1.2...ipckit-v0.1.3) (2025-12-14)


### Features

* implement remaining GitHub issues ([#8](https://github.com/loonghao/ipckit/issues/8), [#9](https://github.com/loonghao/ipckit/issues/9), [#10](https://github.com/loonghao/ipckit/issues/10), [#11](https://github.com/loonghao/ipckit/issues/11), [#12](https://github.com/loonghao/ipckit/issues/12), [#14](https://github.com/loonghao/ipckit/issues/14)) ([c60d274](https://github.com/loonghao/ipckit/commit/c60d2748c1ce72a67e5817673227f3a998512d97))


### Bug Fixes

* quote GITHUB_STEP_SUMMARY variable in pr-checks workflow ([7929ba9](https://github.com/loonghao/ipckit/commit/7929ba9a59d8ee44f7285c76fb6f993b7904f08a))
* replace deprecated pyo3 APIs (downcast -&gt; cast_exact, allow_threads -&gt; detach, PyObject -&gt; Py&lt;PyAny&gt;) ([b755572](https://github.com/loonghao/ipckit/commit/b7555724fd2c82a7ffbc57e2ea32cc1d05bcc330))

## [0.1.2](https://github.com/loonghao/ipckit/compare/ipckit-v0.1.1...ipckit-v0.1.2) (2025-12-13)


### Features

* implement ThreadChannel, EventStream, TaskManager, and SocketServer - Closes [#13](https://github.com/loonghao/ipckit/issues/13), [#15](https://github.com/loonghao/ipckit/issues/15), [#16](https://github.com/loonghao/ipckit/issues/16), [#18](https://github.com/loonghao/ipckit/issues/18) ([6ade9d6](https://github.com/loonghao/ipckit/commit/6ade9d6c7881819ac98af6d0af8cef042bd72a89))


### Bug Fixes

* add missing IpcError import for Unix native backend ([57e545d](https://github.com/loonghao/ipckit/commit/57e545d3b7a66a5de0cb51ca52e317d89e0e3756))


### Code Refactoring

* split python.rs into bindings module for better maintainability ([1701411](https://github.com/loonghao/ipckit/commit/170141124dbb159d283e600561277b772c26e2b9))


### Documentation

* add LocalSocket and GracefulChannel usage examples ([1a2c50c](https://github.com/loonghao/ipckit/commit/1a2c50c90f40319483aa3ac6bd783251de3f46a4))

## [0.1.1](https://github.com/loonghao/ipckit/compare/ipckit-v0.1.0...ipckit-v0.1.1) (2025-12-13)


### Features

* add GracefulChannel for graceful shutdown mechanism (Closes [#7](https://github.com/loonghao/ipckit/issues/7)) ([73f47be](https://github.com/loonghao/ipckit/commit/73f47bebba71d3f73395bf00b17d35a21503f66b))
* enable abi3-py38 for Python 3.8-3.13 compatibility ([84f1987](https://github.com/loonghao/ipckit/commit/84f1987e6a71d5902ad4ed92a61ed96303d0b194))
* use Unix Domain Socket for bidirectional IPC on Unix ([86066a8](https://github.com/loonghao/ipckit/commit/86066a80c81cdd8e5075aff5c9e9679a91f605ee))


### Bug Fixes

* release GIL during blocking pipe operations and fix test assertions for Unix ([ba9b68a](https://github.com/loonghao/ipckit/commit/ba9b68a6f24fd608e1eaafb5bfeca6641976073b))
* remove unused imports and variables in pipe.rs ([d9d3cba](https://github.com/loonghao/ipckit/commit/d9d3cbacad000b339b4abd1dc51b267f33d4a2b4))
* resolve clippy warnings and add pre-commit config ([a24de7b](https://github.com/loonghao/ipckit/commit/a24de7bf7a5bec82cccb9b1579dc1a2c9097b60e))
* resolve compilation warnings and GIL blocking issues ([c7777ec](https://github.com/loonghao/ipckit/commit/c7777ec08bd640fd134aa31f131d692b57291157))
* simplify pr-ready shell script for better compatibility ([f491ce8](https://github.com/loonghao/ipckit/commit/f491ce81d5433f4f78d6990d0e3d74d98762e4e7))
* use Mutex for thread-safe AnonymousPipe access ([93b77fb](https://github.com/loonghao/ipckit/commit/93b77fbd84375316c9dd7314ea4e792659f99e02))
