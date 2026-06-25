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

        build-cli = pkgs.writeShellApplication {
          name = "build";
          runtimeInputs = [ pkgs.cargo pkgs.rustc pkgs.git pkgs.clippy ];
          text = ''
            ROOT=$(git rev-parse --show-toplevel)
            ARGS=()
            TARGET=""
            RELEASE_FLAG=""

            for arg in "$@"; do
              case "$arg" in
                libvellum)  TARGET="libvellum" ;;
                vellum)     TARGET="vellum" ;;
                --release)  RELEASE_FLAG="--release" ;;
                *)          ARGS+=("$arg") ;;
              esac
            done

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
          '';
        };

        vellum-bin = pkgs.writeShellApplication {
          name = "vellum";
          runtimeInputs = [ 
            pkgs.bun
            pkgs.nodejs_20
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
              ui)
                cd "$ROOT/web-app" && bun run dev
                ;;
              ui-npm)
                cd "$ROOT/web-app" && npm run dev
                ;;
              server|manifest|compile|update|harvest|x|query)
                if [ ! -f "$BIN" ]; then
                  echo "Error: vellum binary not found at $BIN. Run 'build vellum --release' first."
                  exit 1
                fi
                cd "$ROOT" && "$BIN" "$COMMAND" "$@"
                ;;
              test)
                cd "$ROOT/rust"
                TEST_ARGS=()
                for arg in "$@"; do
                  case "$arg" in
                    --lint) cargo clippy --all-targets --all-features -- -D warnings ;;
                    --fmt)  cargo fmt --all -- --check ;;
                    --deny) cargo deny check ;;
                    *)      TEST_ARGS+=("$arg") ;;
                  esac
                done
                cargo test "''${TEST_ARGS[@]}"
                ;;
              help|--help|-h)
                echo "Vellum CLI Commands:"
                echo "  ui              : Start Svelte UI Dev Server"
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
        devShells.default = pkgs.mkShell {
          buildInputs = devPackages;
          shellHook = ''
            export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath runtimeLibs}:$LD_LIBRARY_PATH"
            export PATH="$PWD/web-app/node_modules/.bin:$PATH"
          '';
        };
      }
    );
}
