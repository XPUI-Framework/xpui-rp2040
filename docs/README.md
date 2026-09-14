# Documentation

[`../README.md`](../README.md) is the front page. Every document in this
repository, and what each is for:

| | |
|---|---|
| [hardware.md](hardware.md) | everything past a first flash: what each key does, where the memory goes, the pins, the release profile, and one loop for both boards |
| [tutorial.md](tutorial.md) | your first screen on a board: a board is data, the palette trap, the panel driver seam, and what running it proved — compiled by `docs-test/` |
| [reference.md](reference.md) | every public item: `run` and `run_async`, `ButtonPins`, and the runtime's `init_log`, `init_heap` and `park` — checked against the board's rustdoc, its `rust` fence compiled by `docs-test/` |
| [contributing.md](contributing.md) | why the firmware is its own workspace and the three workspaces that follow, the target, the gate, the five review steps, and how a commit is written |
| [docs-test/README.md](../docs-test/README.md) | the crate with no code that compiles the tutorial, and why it needs the host triple named |
