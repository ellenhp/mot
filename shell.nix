{ pkgs ? import <nixpkgs> {} }:
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [ rustc cargo cargo-typify gcc rustfmt clippy openssl pkg-config sqlx-cli tokio-console docker-compose sqlite libspatialite protobuf ];

  # Certain Rust tools won't work without this
  # This can also be fixed by using oxalica/rust-overlay and specifying the rust-src extension
  # See https://discourse.nixos.org/t/rust-src-not-found-and-other-misadventures-of-developing-rust-on-nixos/11570/3?u=samuela. for more details.
  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";

  DATABASE_URL = "postgres:///osm";
  LD_LIBRARY_PATH = "${pkgs.libspatialite}/lib";
  PROTOC = "${pkgs.protobuf}/bin/protoc";

  shellHook = "export DOCKER_HOST=unix:///run/user/$UID/podman/podman.sock; export HOST_USER=$USER";
}
