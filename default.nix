{ pkgs ? import <nixpkgs> {} }:

{
  exec_and_record = pkgs.callPackage ./package.nix { };
}
