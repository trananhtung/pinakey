{
  description = "PinaKey — bộ gõ tiếng Việt cho fcitx5 (lõi Rust)";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  # Mã nguồn PinaKey khai báo dưới dạng input KHÔNG-phải-flake. Trước đây dùng `src = ../.`,
  # nhưng flake này nằm ở packaging/ nên khi Nix evaluate (`nix build path:./packaging`) chỉ
  # thư mục packaging/ được copy vào store; `../.` khi đó trỏ RA NGOÀI store path của flake và
  # Nix từ chối → không build được về nguyên tắc. Khai báo repo gốc làm input giúp `src` là một
  # store path hợp lệ, reproducible (ghim trong flake.lock) và pure-eval được. (#164)
  #
  # KHUNG MẪU: để build đúng cây mã ĐANG LÀM VIỆC (local) thay vì main trên GitHub, override input:
  #     nix build path:./packaging --override-input pinakey-src "path:$(git rev-parse --show-toplevel)"
  # Lưu ý build phase chạy cargo trong CMake nên CẦN MẠNG cho crates; muốn build offline thuần Nix
  # thì thay bằng cơ chế vendor (fetchCargoVendor/importCargoLock) — ngoài phạm vi khung mẫu này.
  inputs.pinakey-src.url = "github:trananhtung/pinakey";
  inputs.pinakey-src.flake = false;

  outputs = { self, nixpkgs, pinakey-src }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAll = f: nixpkgs.lib.genAttrs systems (s: f nixpkgs.legacyPackages.${s});
    in {
      packages = forAll (pkgs: {
        default = pkgs.stdenv.mkDerivation {
          pname = "fcitx5-pinakey";
          version = "2.0.0";
          src = pinakey-src;
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
