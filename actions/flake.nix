{
  description = "Vellum Config Scripts Compiler";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, utils }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };

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
        } (builtins.readFile ./get_lyrics/main.py);

        search_cover = pkgs.writers.writePython3Bin "search_cover" {
          libraries = [];
          doCheck = false;
        } (builtins.readFile ./search_cover/main.py);

        build-cli = pkgs.writeShellApplication {
          name = "build";
          runtimeInputs = [ pkgs.cargo pkgs.rustc pkgs.nix ];
          text = ''
            nix build .#get_lyrics --out-link get_lyrics/result
            nix build .#search_cover --out-link search_cover/result
            cargo build --manifest-path cover_palette/Cargo.toml --release
          '';
        };
      in
      {
        packages.default = get_lyrics;
        packages.get_lyrics = get_lyrics;
        packages.search_cover = search_cover;
        packages.build = build-cli;

        apps.build = {
          type = "app";
          program = "${build-cli}/bin/build";
        };

        devShells.default = pkgs.mkShell {
          buildInputs = [ build-cli ];
        };
      }
    );
}
