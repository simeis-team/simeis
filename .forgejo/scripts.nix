{ pkgs, deps }: {
  check_rust_code = let
    app = pkgs.writeShellApplication {
      name = "rust-check-code";
      runtimeInputs = deps ++ [ pkgs.gcc ];

      text = ''
        set -e
        export CARGO_HOME=$PWD/.cargohome
        cargo check
        cargo clippy --no-deps --frozen -- -D warnings
        cargo fmt --check
        cargo audit
        if ! [ -d supply-chain ]; then
          cargo vet init
        fi
        cargo vet
      '';
    };
  in "${app}/bin/rust-check-code";

  check_todos = let
    ignored=builtins.concatStringsSep "|" [
      ".git"
      ".forgejo"
      "TODO.md"
    ];

    app = pkgs.writeShellApplication {
      name = "check-todos";
      runtimeInputs = [ pkgs.ripgrep pkgs.jq ];
      text = ''
        set -e
        ERRCODE=0
        CURL_OPTS="-s"

        export GITHUB_BASE_REF="main"
        export GITHUB_SERVER_URL="http://0.0.0.0:8083"
        export GITHUB_REPOSITORY="litchi.pi/teaching"

        git fetch origin "$GITHUB_BASE_REF"
        echo ""

        set +e
        FILES_TO_CHECK=$( rg -l "TODO" | rg -v "${ignored}" | tr '\n' ' ')
        git diff "origin/$GITHUB_BASE_REF" -- "$FILES_TO_CHECK" \
          | rg -v "^-" \
          | rg "TODO" > todos_to_check

        rg -o "\(#\d+\)" todos_to_check \
          | rg -o "\d+" \
          | sort -u -n \
          > issues_to_check

        while read -r ISSUE; do
          if ! curl "$CURL_OPTS" \
            "$GITHUB_SERVER_URL/api/v1/repos/$GITHUB_REPOSITORY/issues/$ISSUE" \
            | jq ".state" \
            | grep "\"open\"" \
            2>/dev/null 1>/dev/null
          then
            echo "[!] Issue $ISSUE is closed or missing"
            ERRCODE=1
          fi
        done < issues_to_check
        echo ""

        if rg "TODO" "$FILES_TO_CHECK" | rg -v "\(#\d+\)"; then
          echo "[!] Some TODOs are not linked to an existing issue"
          ERRCODE=1
        fi
        echo ""

        exit $ERRCODE
      '';
    };
    in "${app}/bin/check-todos";
  }
