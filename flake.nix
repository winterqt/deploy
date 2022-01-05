{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  inputs.flake-utils.url = "github:numtide/flake-utils";

  outputs = { self, nixpkgs, flake-utils }: flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = nixpkgs.legacyPackages.${system};
      lib = pkgs.lib;
    in
    {
      packages.deploy = pkgs.rustPlatform.buildRustPackage {
        pname = "deploy";
        version = (lib.importTOML ./Cargo.toml).package.version;

        src = self;
        cargoLock.lockFile = ./Cargo.lock;

        nativeBuildInputs = with pkgs; [ pkg-config installShellFiles ];
        buildInputs = with pkgs; [ openssl ] ++ lib.optional stdenv.isDarwin darwin.apple_sdk.frameworks.SystemConfiguration;

        postInstall = ''
          installShellCompletion $releaseDir/build/deploy-*/out/deploy.{bash,fish} \
            --zsh $releaseDir/build/deploy-*/out/_deploy
        '';
      };

      defaultPackage = self.packages.${system}.deploy;

      devShell = pkgs.mkShell {
        nativeBuildInputs = with pkgs;
          [ rustc cargo clippy pkg-config openssl ] ++ lib.optionals stdenv.isDarwin [ libiconv darwin.apple_sdk.frameworks.SystemConfiguration ];
      };
    }
  );
}
