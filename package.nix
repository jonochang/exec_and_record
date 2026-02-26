# Local package definition for exec_and_record
{
  lib,
  rustPlatform,
}:

rustPlatform.buildRustPackage {
  pname = "exec_and_record";
  version = "0.2.2";

  src = ./.;

  # Generate with `cargo generate-lockfile`
  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  meta = {
    description = "Record terminal commands to video and logs";
    license = with lib.licenses; [ mit ];
    platforms = lib.platforms.unix;
    mainProgram = "exec_and_record";
  };
}
