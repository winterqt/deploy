{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  inputs.flake-utils.url = "github:numtide/flake-utils";

  outputs = { self, nixpkgs, flake-utils }: flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = nixpkgs.legacyPackages.${system};
    in
    {
      packages.deploy = pkgs.callPackage
        ({ lib, stdenv, rustPlatform, pkg-config, openssl, SystemConfiguration, installShellFiles }:
          rustPlatform.buildRustPackage {
            pname = "deploy";
            version = (lib.importTOML ./Cargo.toml).package.version;

            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;

            nativeBuildInputs = [ pkg-config installShellFiles ];
            buildInputs = [ openssl ] ++ lib.optional stdenv.isDarwin SystemConfiguration;

            postInstall = ''
              installShellCompletion $releaseDir/build/deploy-*/out/deploy.{bash,fish} \
                --zsh $releaseDir/build/deploy-*/out/_deploy
            '';
          })
        { inherit (pkgs.darwin.apple_sdk.frameworks) SystemConfiguration; };
      defaultPackage = self.packages.${system}.deploy;

      devShell = pkgs.mkShell {
        nativeBuildInputs = with pkgs;
          [ rustc cargo clippy pkg-config openssl ] ++ lib.optionals stdenv.isDarwin [ libiconv darwin.apple_sdk.frameworks.SystemConfiguration ];
      };
    }
  );
}
