{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = inputs: with inputs; flake-utils.lib.eachDefaultSystem (system: let
    pkgs = import nixpkgs {
      inherit system;
      overlays = [ rust-overlay.overlays.default ];
    };

    rustpkg = pkgs.rust-bin.stable."1.82.0".default.override {
      extensions = [ "rust-src" ];
    };

    deps = with pkgs; [
      rustpkg
      cargo-audit
      cargo-vet
      mold
    ];

    mkScript = script: { type = "app"; program = builtins.toString script; };
    scripts = import ./.forgejo/scripts.nix { inherit pkgs deps; };
  in {
    apps = builtins.mapAttrs (name: val: mkScript val) scripts;
    devShells.default = pkgs.mkShell {
      buildInputs = deps;
      shellHook = ''
      '';
    };
  });
}
