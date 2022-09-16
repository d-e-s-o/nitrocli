# Copyright (C) 2020 The Nitrocli Developers
# SPDX-License-Identifier: CC0-1.0
{
  inputs = {
    naersk.url = "github:nix-community/naersk/master";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, utils, naersk }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        naersk-lib = pkgs.callPackage naersk { };
      in
      rec
      {
        packages.default = naersk-lib.buildPackage {
          root = ./../../.;
          nativeBuildInputs = with pkgs; [ hidapi ];
          postInstall = ''
            # copy the manpages
            install -D --mode 0644 ${./../../.}/doc/nitrocli.1 $out/share/man/man1
            # make completions
            mkdir --parents $out/share/bash-completion/completions/
            cargo run --bin=shell-complete bash > $out/share/bash-completion/completions/nitrocli
            mkdir --parents $out/share/zsh/site-functions/
            cargo run --bin=shell-complete zsh > $out/share/zsh/site-functions/_nitrocli
            mkdir --parents $out/share/fish/vendor_completions.d/
            cargo run --bin=shell-complete fish > $out/share/fish/vendor_completions.d/nitrocli.fish
          '';
        };

        apps.default = utils.lib.mkApp {
          drv = packages.default;
          name = "nitrocli";
        };

        devShell = with pkgs; mkShell {
          buildInputs = [ cargo rustc rustfmt pre-commit rustPackages.clippy hidapi ];
          RUST_SRC_PATH = rustPlatform.rustLibSrc;
        };
      });
}
