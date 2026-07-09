{
  description = "Vellum Development Environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, utils }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };

        runtimeLibs = with pkgs; [
          libGL
          libxkbcommon
          wayland
          libX11
          libXcursor
          libXi
          libXrandr
        ];

        lyricsgenius = ps: ps.buildPythonPackage rec {
          pname = "lyricsgenius";
          version = "3.7.6";
          pyproject = true;
          
          src = ps.fetchPypi {
            inherit pname version;
            hash = "sha256-zQGrgZEz4o9RSYWmGXH8TcNXUcRSfmF+xJCROQ3cPJ4=";
          };

          nativeBuildInputs = [
            ps.hatchling
          ];

          propagatedBuildInputs = [
            ps.requests
            ps.beautifulsoup4
          ];

          doCheck = false;
        };

        get_lyrics = pkgs.writers.writePython3Bin "get_lyrics" {
          libraries = [
            (lyricsgenius pkgs.python3Packages)
            pkgs.python3Packages.requests
            pkgs.python3Packages.beautifulsoup4
          ];
          doCheck = false;
        } (builtins.readFile ./actions/get_lyrics/main.py);

        search_cover = pkgs.writers.writePython3Bin "search_cover" {
          libraries = [];
          doCheck = false;
        } (builtins.readFile ./actions/search_cover/main.py);

        build-cli = pkgs.writeShellApplication {
          name = "build";
          runtimeInputs = [ pkgs.cargo pkgs.rustc pkgs.git pkgs.clippy pkgs.nix ];
          text = ''
            ROOT=$(git rev-parse --show-toplevel)
            ARGS=()
            TARGET=""
            RELEASE_FLAG=""

            for arg in "$@"; do
              case "$arg" in
                libvellum)  TARGET="libvellum" ;;
                vellum)     TARGET="vellum" ;;
                actions)    TARGET="actions" ;;
                --release)  RELEASE_FLAG="--release" ;;
                *)          ARGS+=("$arg") ;;
              esac
            done

            if [ "$TARGET" = "actions" ]; then
              cd "$ROOT"
              nix build .#get_lyrics --out-link actions/get_lyrics/result
              nix build .#search_cover --out-link actions/search_cover/result
              
              cd "$ROOT/actions"
              cargo clippy --workspace -- -D warnings
              
              CMD=("cargo" "build" "--workspace")
              if [ -n "$RELEASE_FLAG" ]; then
                CMD+=("$RELEASE_FLAG")
              fi
              
              CMD+=("''${ARGS[@]}")
              
              "''${CMD[@]}"
            else
              cd "$ROOT/rust"
              cargo clippy --workspace -- -D warnings
              
              CMD=("cargo" "build")
              if [ -n "$TARGET" ]; then
                CMD+=("-p" "$TARGET")
              fi
              if [ -n "$RELEASE_FLAG" ]; then
                CMD+=("$RELEASE_FLAG")
              fi
              
              CMD+=("''${ARGS[@]}")
              
              "''${CMD[@]}"
            fi
          '';
        };

        check-cli = pkgs.writeShellApplication {
          name = "check";
          runtimeInputs = [ pkgs.cargo pkgs.rustc pkgs.git pkgs.clippy ];
          text = ''
            ROOT=$(git rev-parse --show-toplevel)
            ARGS=()
            LINT=false

            for arg in "$@"; do
              case "$arg" in
                --lint) LINT=true ;;
                *)      ARGS+=("$arg") ;;
              esac
            done

            cd "$ROOT/rust"
            if [ "$LINT" = true ]; then
              cargo clippy --workspace -- "''${ARGS[@]}" -D warnings
            else
              cargo check "''${ARGS[@]}"
            fi

            cd "$ROOT/actions"
            if [ "$LINT" = true ]; then
              cargo clippy --workspace -- "''${ARGS[@]}" -D warnings
            else
              cargo check "''${ARGS[@]}"
            fi
          '';
        };

        vellum-bin = pkgs.writeShellApplication {
          name = "vellum";
          runtimeInputs = [ 
            pkgs.bun
            pkgs.cargo 
            pkgs.rustc 
            pkgs.clippy
            pkgs.rustfmt
            pkgs.cargo-deny
            pkgs.pkg-config 
            pkgs.openssl 
            pkgs.nix
            pkgs.git
          ];
          text = ''
            ROOT=$(git rev-parse --show-toplevel)
            BIN="$ROOT/rust/target/release/vellum"
            COMMAND=''${1:-"help"}
            if [ "$#" -gt 0 ]; then shift; fi

            case "$COMMAND" in
              interface|server|manifest|compile|update|harvest|x|query)
                if [ ! -f "$BIN" ]; then
                  echo "Error: vellum binary not found at $BIN. Run 'build vellum --release' first."
                  exit 1
                fi
                cd "$ROOT" && "$BIN" "$COMMAND" "$@"
                ;;
              test)
                TEST_ARGS=()
                for arg in "$@"; do
                  case "$arg" in
                    --lint) cargo clippy --all-targets --all-features -- -D warnings ;;
                    --fmt)  cargo fmt --all -- --check ;;
                    --deny) cargo deny check ;;
                    *)      TEST_ARGS+=("$arg") ;;
                  esac
                done

                cd "$ROOT/rust"
                cargo test "''${TEST_ARGS[@]}"
                
                cd "$ROOT/actions"
                cargo test "''${TEST_ARGS[@]}"
                ;;
              help|--help|-h)
                echo "Vellum CLI Commands:"
                echo "  interface       : Run system installed interface"
                echo "  server          : Start Backend Rust Server"
                echo "  compile         : Compile metadata locks"
                echo "  update          : Update library"
                echo "  query           : Run SQL queries against the library"
                echo "  harvest         : Harvest raw metadata to JSON"
                echo "  x               : Run defined actions via runtime router"
                echo "    --lint        : Run clippy with -D warnings"
                echo "    --fmt         : Run fmt check"
                echo "    --deny        : Run cargo-deny check"
                ;;
              *)
                echo "Error: Unknown command '$COMMAND'"
                exit 1
                ;;
            esac
          '';
        };

        devPackages = with pkgs; [
          bun
          pkg-config
          openssl
          build-cli
          check-cli
          vellum-bin
          cargo
          rustc
          rust-analyzer
          clippy
          rustfmt
          cargo-deny
          glib
          gtk3
        ] ++ runtimeLibs;
      in
      {
        packages.get_lyrics = get_lyrics;
        packages.search_cover = search_cover;

        devShells.default = pkgs.mkShell {
          buildInputs = devPackages;
          shellHook = ''
            export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath runtimeLibs}:$LD_LIBRARY_PATH"
            export PATH="$PWD/interfaces/web-app/node_modules/.bin:$PATH"
          '';
        };
      }
    );
}
