{
  description = "Explore git internals, the plumbing";

  # Suggest binary cache for faster builds
  nixConfig = {
    extra-substituters = [ "https://git-plumber.cachix.org" ];
    extra-trusted-public-keys = [ "git-plumber.cachix.org-1:A40lddBYiPFacXEF8iHiiOkuJSHBw2D5IeIEr98Velg=" ];
  };

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rustfmt" ];
        };

        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };

        # Read version from Cargo.toml and add git info for development builds
        cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
        rev = self.shortRev or self.dirtyShortRev or "dirty";
        date = self.lastModifiedDate or self.lastModified or "19700101";
        version = cargoToml.package.version 
          + (if (self.dirtyShortRev or null) != null || (self.shortRev or null) == null 
             then "-dev${builtins.substring 0 8 date}_${rev}"
             else "");

      in
      {
        packages = {
          default = self.packages.${system}.git-plumber;
          
          git-plumber = rustPlatform.buildRustPackage rec {
            pname = "git-plumber";
            inherit version;
            
            src = ./.;
            
            cargoHash = "sha256-J1zTE9QgdGWGK0/VECvKPlhVeTzH3Td9wj0JBbxST08=";
            
            nativeBuildInputs = with pkgs; [
              pkg-config
            ];
            
            buildInputs = with pkgs; [
              # Add any system dependencies here if needed
            ];

            meta = with pkgs.lib; {
              description = "Explore git internals, the plumbing - A CLI and TUI application for exploring the internals of git repositories";
              homepage = "https://github.com/ejiektpobehuk/git-plumber";
              license = licenses.mit;
              maintainers = [ maintainers.ejiek or "Vlad Petrov <oss@ejiek.id>" ];
              mainProgram = "git-plumber";
              platforms = platforms.all;
            };
          };
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            pkg-config
            # Development tools
            rust-analyzer
            clippy
            rustfmt
            bacon
          ];
          
          shellHook = ''
            echo "🦀 Rust development environment for git-plumber"
            echo "Run 'cargo run' to start the application"
          '';
        };

        # For backwards compatibility
        defaultPackage = self.packages.${system}.default;
        devShell = self.devShells.${system}.default;

        # Nix code formatter
        formatter = pkgs.nixfmt-rfc-style;
      })
    // {
      # Overlays for easier integration in other Nix setups
      overlays = {
        default = self.overlays.git-plumber;
        git-plumber = _: prev: { 
          inherit (self.packages.${prev.stdenv.system}) git-plumber; 
        };
      };
    };
} 