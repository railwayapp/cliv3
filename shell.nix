let
  pkgs = import (fetchTarball "https://github.com/NixOS/nixpkgs/archive/313b84933167.tar.gz") {
    overlays = [
      (import (fetchTarball "https://github.com/oxalica/rust-overlay/archive/d0dc81ffe8ea.tar.gz"))
    ];
  };

in
pkgs.mkShell {
  buildInputs = with pkgs; [
    cmake
    rust-bin.stable.latest.minimal

    # For macOS
    darwin.apple_sdk.frameworks.Security
  ];
}
