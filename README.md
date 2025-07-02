# pcp

pcp is a parallel copy/move utility with safety and speed as the main focus

## Goals

- performance: aims to be fast by default
- safety: when copying files you want to be sure nothing gets corrupted or lost
- progress: should be able to track progress and continue when it left off
- cross-platform: long term, pcp should work on Windows, Linux and macOS
- file-input: take a file as input to read directories from

## Installation

### Download the binary

[https://github.com/Skyppex/pcp/releases](https://github.com/Skyppex/pcp/releases)

### Build from source

clone the repo:
```sh
git clone https://github.com/skypex/pcp.git
```

`cd` into the `pcp` directory with `cd pcp`

then build:

```sh
cargo build
# or
cargo build --release
```

the binary can be found in `./target/debug/` or `./target/release/`
respectively.

add that location to `PATH` or copy the binary to a folder in your `PATH`.

then you can run `pcp --help` to see all the options (`pcp.exe --help` on some windows shells)

