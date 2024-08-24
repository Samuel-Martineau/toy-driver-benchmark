{
  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-24.05";

  outputs =
    { self, nixpkgs }:
    {
      devShells =
        nixpkgs.lib.genAttrs
          [
            "x86_64-linux"
            "aarch64-linux"
            "aarch64-darwin"
            "x86_64-darwin"
          ]
          (
            system: with nixpkgs.legacyPackages.${system}; {
              default = mkShell {
                packages = [
                  cargo
                  cargo-watch
                  cargo-expand
                  rustc
                  rust-analyzer
                  rustfmt
                  just
                  pkg-config
                  openssl
                  hexyl
                  postgresql
                  pv
                ];
              };
            }
          );
    };
}
