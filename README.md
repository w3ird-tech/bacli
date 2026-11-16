# Bitaxe CLI

Bitaxe CLI, or bacli, is a wrapper around the Bitaxe API, enabling the management of a Bitaxe
device in an easy to use way.

## Quickstart

Download the latest release from [GitHub Releases](https://github.com/w3ird-tech/bacli/releases).

## Development

From the repository root, with Nix flakes enabled:

```shell
nix develop
cargo build --locked             # target/debug/bacli
cargo build --release --locked   # target/release/bacli
cargo run -- --help
```

Run checks inside the shell:

```shell
just lint
cargo test --workspace --all-features --locked
```

Without Nix, install stable Rust, OpenSSL, pkg-config, and CMake, then use the same
Cargo commands. Use `cargo +nightly fmt` for formatting.

## Nix builds

Build directly without entering the shell:

```shell
nix build .#dev      # dev build
nix build .#release  # release build (also the default for nix build)
./result/bin/bacli --help
```

Nix supports x86_64/aarch64 Linux and Apple Silicon macOS.

## Support

PRs are more than welcome!

Feeling generous? Leave me a tip! ⚡️w3irdrobot@getalby.com.

Think I'm an asshole but still want to tip? Please donate [to OpenSats](https://opensats.org/).

Want to tell me how you feel? Hit me up [on Nostr](https://njump.me/npub17q5n2z8naw0xl6vu9lvt560lg33pdpe29k0k09umlfxm3vc4tqrq466f2y).
