{
  description = "A flake for developing the Railway CLI";

  inputs.nixpkgs.url = github:NixOS/nixpkgs;
  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";

  outputs = { self, rust-overlay, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs {
            inherit system overlays;
          };
        in
        {
          devShells.default =
            import ./shell.nix { inherit pkgs; };
        }
      );
}
