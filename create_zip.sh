#!/usr/bin/env bash
nix build
zip -9 -r simeis.zip ./Cargo.* ./result/manual.pdf ./simeis-server ./simeis-data ./.gitignore
