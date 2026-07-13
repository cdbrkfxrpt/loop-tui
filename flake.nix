{
  description = "loop — a simple GTD-based todo-list TUI";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      crane,
      rust-overlay,
      flake-utils,
      ...
    }:
    let
      supportedSystems = [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-linux"
      ];
    in
    flake-utils.lib.eachSystem supportedSystems (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        inherit (pkgs) lib;

        # ==============================================================================
        # Rust Toolchain
        # ==============================================================================

        # import sorting (and other niceties) only land in nightly rustfmt.
        rustfmt = pkgs.rust-bin.nightly.latest.rustfmt;

        # Stable builds the crate. The `minimal` profile omits rustfmt on purpose, so
        # the nightly rustfmt above is the only `rustfmt` on PATH — no collision to
        # resolve in the dev shell.
        rustBuildToolchain = pkgs.rust-bin.stable.latest.minimal;
        rustDevToolchain = rustBuildToolchain.override {
          extensions = [
            "clippy"
            "rust-analyzer"
            "rust-src"
          ];
        };

        # ==============================================================================
        # Source Filtering and Build Configuration
        # ==============================================================================

        craneLib = (crane.mkLib pkgs).overrideToolchain (_: rustBuildToolchain);

        # crane's default filter keeps only Cargo files and `.rs` sources, but the
        # banner is pulled in with `include_str!("loop.txt")`. Keep `.txt` files too,
        # otherwise the art is missing at compile time.
        src = lib.cleanSourceWith {
          src = ./.;
          name = "loop-source";
          filter = path: type: (lib.hasSuffix ".txt" path) || (craneLib.filterCargoSources path type);
        };

        commonArgs = {
          inherit src;
          pname = "loop";
          strictDeps = true;
        };

        # Build the dependency graph once and cache it, so everyday rebuilds only
        # recompile our own crate. This incremental split is crane's headline feature.
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # ==============================================================================
        # Packages
        # ==============================================================================

        loop = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
            meta.mainProgram = "loop";
          }
        );
      in
      {
        packages.default = loop;

        devShells.default = pkgs.mkShell {
          inputsFrom = [ loop ];
          packages = [
            rustfmt
            rustDevToolchain
          ]
          ++ (with pkgs; [
            go-grip
            cargo-sort
            cargo-machete
          ]);

          shellHook = ''
            export RUST_SRC_PATH="${rustDevToolchain}/lib/rustlib/src/rust/library"
          '';
        };

        formatter = pkgs.nixfmt-rfc-style;
      }
    );
}
