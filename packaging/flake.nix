{
  description = "PinaKey — bộ gõ tiếng Việt cho fcitx5 (lõi Rust)";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAll = f: nixpkgs.lib.genAttrs systems (s: f nixpkgs.legacyPackages.${s});
    in {
      packages = forAll (pkgs: {
        default = pkgs.stdenv.mkDerivation {
          pname = "fcitx5-pinakey";
          version = "1.1.0";
          src = ../.;
          nativeBuildInputs = with pkgs; [ cmake extra-cmake-modules cargo rustc pkg-config ];
          buildInputs = with pkgs; [ fcitx5 ];
          # Lõi Rust build qua cargo trong CMake; cần mạng cho crates hoặc dùng
          # cargo vendor + offline. Đây là khung mẫu, có thể cần fetchCargoTarball.
          configurePhase = ''
            cmake -S fcitx5 -B build -DCMAKE_INSTALL_PREFIX=$out -DPINAKEY_BUILD_TESTS=OFF
          '';
          buildPhase = "cmake --build build";
          installPhase = "cmake --install build";
        };
      });
    };
}
