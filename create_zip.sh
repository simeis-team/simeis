#!/usr/bin/env bash
nix build
rm -f ./simeis.zip
zip -9 -r simeis.zip ./Cargo.* ./result/manual.pdf ./simeis-server ./simeis-data ./.gitignore ./example
