{
  description = "knave, a small Lisp for humans and agents that infers types and ownership and compiles to readable Rust";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    {
      self,
      nixpkgs,
      crane,
    }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];

      forEachSystem =
        build: nixpkgs.lib.genAttrs supportedSystems (system: build nixpkgs.legacyPackages.${system});

      ciPackages =
        pkgs:
        [
          pkgs.rustc
          pkgs.cargo
          pkgs.clippy
          pkgs.rustfmt
          pkgs.cargo-llvm-cov
          pkgs.git
        ]
        ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [ pkgs.valgrind ];

      developmentPackages =
        pkgs:
        ciPackages pkgs
        ++ [
          pkgs.cargo-insta
          pkgs.nixfmt
          pkgs.rust-analyzer
          pkgs.sccache
        ];

      moldLinker =
        pkgs:
        pkgs.lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
          nativeBuildInputs = [ pkgs.mold ];
          "CARGO_TARGET_${pkgs.stdenv.hostPlatform.rust.cargoEnvVarTarget}_RUSTFLAGS" =
            "-C link-arg=-fuse-ld=mold";
        };

      baselineManifest = builtins.fromTOML (builtins.readFile ./xtask/baseline/Cargo.toml);

      baselineMerger =
        pkgs:
        let
          inherit (baselineManifest.package) edition;
          library = builtins.replaceStrings [ "-" ] [ "_" ] baselineManifest.package.name;
          binary = (builtins.head baselineManifest.bin).name;
        in
        pkgs.runCommandCC binary
          {
            nativeBuildInputs = [ pkgs.rustc ];
            meta.mainProgram = binary;
          }
          ''
            sources=${./xtask/baseline/src}
            rustc --edition ${edition} -O --crate-type lib --crate-name ${library} "$sources/lib.rs" -o lib${library}.rlib
            mkdir -p $out/bin
            rustc --edition ${edition} -O "$sources/main.rs" --extern ${library}=lib${library}.rlib -o $out/bin/${binary}
          '';

      baselineMergerCheck =
        pkgs:
        pkgs.runCommand "baseline-merger" { } ''
          printf 'a 383\n' > base
          printf 'a 380\n' > ours
          printf 'a 381\n' > theirs
          ${pkgs.lib.getExe (baselineMerger pkgs)} base ours theirs baseline.txt
          [[ $(<ours) == 'a 378' ]] || { echo "the merger wrote: $(<ours)" >&2; exit 1; }
          touch $out
        '';

      cargoManifest = builtins.fromTOML (builtins.readFile ./Cargo.toml);
      cratePackage = cargoManifest.package;
      crateVersion = cargoManifest.workspace.package.version;
      installedProfileName = "release";

      coverageTools = pkgs: {
        LLVM_COV = "${pkgs.rustc.llvmPackages.llvm}/bin/llvm-cov";
        LLVM_PROFDATA = "${pkgs.rustc.llvmPackages.llvm}/bin/llvm-profdata";
      };

      agentDocs =
        pkgs:
        pkgs.runCommand "agent-docs" { } ''
          fail() {
            echo "$1" >&2
            exit 1
          }
          guide=${./AGENTS.md}
          references=${./docs}
          budget=25000
          size=$(wc -c < "$guide")
          (( size <= budget )) ||
            fail "AGENTS.md is $size bytes, over its budget of $budget. Move a rule that only one kind of task needs into a reference file under docs/."
          if grep --quiet '^@' "$guide"; then
            fail "AGENTS.md imports a file with @, which loads the file in every session. Name the file in the Reference files table instead."
          fi
          for file in $(cd "$references" && find . -name '*.md' | sed 's#^\./##'); do
            grep --fixed-strings --quiet "| \`docs/$file\` |" "$guide" ||
              fail "The Reference files table in AGENTS.md has no row for docs/$file."
          done
          for file in $(grep --only-matching '^| `docs/[^`]*`' "$guide" | sed 's#^| `docs/##; s#`$##'); do
            [[ -f $references/$file ]] ||
              fail "The Reference files table in AGENTS.md names docs/$file, which does not exist."
          done
          touch $out
        '';

      cargoWithBuildSlots =
        pkgs: realCargo:
        pkgs.writeShellApplication {
          name = "cargo";
          excludeShellChecks = [ "SC2016" ];
          text = ''
            real_cargo=${pkgs.lib.getExe' realCargo "cargo"}
            flock=${pkgs.lib.getExe pkgs.flock}
            ps=${if pkgs.stdenv.hostPlatform.isLinux then pkgs.lib.getExe' pkgs.procps "ps" else "/bin/ps"}
            awk=${pkgs.lib.getExe pkgs.gawk}
            slot_directory=$HOME/.cache/knave/build-slots
            slot_limit=''${KNAVE_BUILD_SLOTS:-2}
            time_limit=''${KNAVE_BUILD_TIME_LIMIT:-}
            timed_out_status=124
            seconds_before_kill=5
            seconds_between_attempts=1

            main() {
              if [[ -n ''${CI:-} || -n ''${KNAVE_BUILD_SLOT_HELD:-} ]] || ! command_compiles "$@"; then
                exec "$real_cargo" "$@"
              fi
              require_positive_whole_number KNAVE_BUILD_SLOTS "$slot_limit"
              if [[ -n $time_limit ]]; then
                require_positive_whole_number KNAVE_BUILD_TIME_LIMIT "$time_limit"
              fi
              mkdir -p "$slot_directory"
              wait_for_slot
              # Keep the lock from cargo: a daemon that a build starts, such as sccache, would inherit it and hold the slot.
              run_in_slot "$@" {slot_lock}>&-
            }

            run_in_slot() {
              if [[ -z $time_limit ]]; then
                KNAVE_BUILD_SLOT_HELD=1 "$real_cargo" "$@"
                return
              fi
              run_within_time_limit "$@"
            }

            run_within_time_limit() {
              local cargo_process timer finished="" status=0
              start_cargo_in_callers_process_group "$@"
              cargo_process=$!
              sleep "$time_limit" &
              timer=$!
              wait -n -p finished "$cargo_process" "$timer" || status=$?
              if [[ $finished == "$cargo_process" ]]; then
                kill "$timer" 2>/dev/null || true
                return "$status"
              fi
              stop_process_tree "$cargo_process"
              wait "$cargo_process" || true
              echo "cargo: stopped by KNAVE_BUILD_TIME_LIMIT=$time_limit, which counts seconds from when the command took a build slot" >&2
              return "$timed_out_status"
            }

            start_cargo_in_callers_process_group() {
              # A job started with & ignores SIGINT and reads /dev/null, so restore both: Ctrl-C and a program's input reach cargo as they would in the foreground.
              (
                trap - INT QUIT
                KNAVE_BUILD_SLOT_HELD=1 exec "$real_cargo" "$@"
              ) <&0 &
            }

            stop_process_tree() {
              local tree waited
              mapfile -t tree < <(process_tree "$1")
              kill -TERM "''${tree[@]}" 2>/dev/null || true
              for ((waited = 0; waited < seconds_before_kill; waited++)); do
                any_running "''${tree[@]}" || return 0
                sleep 1
              done
              kill -KILL "''${tree[@]}" 2>/dev/null || true
            }

            process_tree() {
              "$ps" -A -o pid= -o ppid= | "$awk" -v root="$1" '
                { parent[$1] = $2 }
                END {
                  tree[root] = 1
                  do {
                    grown = 0
                    for (process in parent)
                      if (!(process in tree) && (parent[process] in tree)) { tree[process] = 1; grown = 1 }
                  } while (grown)
                  for (process in tree) print process
                }'
            }

            any_running() {
              local processes
              processes=$(IFS=,; echo "$*")
              "$ps" -o stat= -p "$processes" | "$awk" '!/^Z/ { found = 1 } END { exit !found }'
            }

            command_compiles() {
              case $(subcommand "$@") in
                "" | add | clean | config | fetch | fmt | generate-lockfile | help | info | init | locate-project | login | \
                  logout | metadata | new | owner | pkgid | read-manifest | remove | report | search | tree | update | \
                  vendor | verify-project | version | yank) return 1 ;;
              esac
            }

            subcommand() {
              while (($# > 0)); do
                case $1 in
                  --color | --config | --explain | -C | -Z) shift "$(($# > 1 ? 2 : 1))" ;;
                  -*) shift ;;
                  *)
                    echo "$1"
                    return
                    ;;
                esac
              done
            }

            require_positive_whole_number() {
              if [[ ! $2 =~ ^[1-9][0-9]*$ ]]; then
                echo "$1 must be a positive whole number, not '$2'" >&2
                exit 2
              fi
            }

            wait_for_slot() {
              join_queue
              until take_free_slot; do
                tell_user_about_wait
                sleep "$seconds_between_attempts"
              done
              exec {queue_lock}>&-
            }

            join_queue() {
              exec {queue_lock}>"$slot_directory/queue-of-$slot_limit"
              if ! "$flock" -n "$queue_lock"; then
                tell_user_about_wait
                "$flock" "$queue_lock"
              fi
            }

            tell_user_about_wait() {
              if [[ -z ''${told_user_about_wait:-} ]]; then
                echo "cargo: waiting for one of $slot_limit build slots" >&2
                told_user_about_wait=yes
              fi
            }

            take_free_slot() {
              local slot
              for ((slot = 1; slot <= slot_limit; slot++)); do
                exec {slot_lock}>"$slot_directory/$slot"
                if "$flock" -n "$slot_lock"; then
                  return
                fi
                exec {slot_lock}>&-
              done
              return 1
            }

            main "$@"
          '';
        };

      slotLimitedCargo = pkgs: cargoWithBuildSlots pkgs pkgs.cargo;

      devShellTools =
        pkgs: defaultShell:
        let
          installedOutputs = map (input: input.out or input) defaultShell.nativeBuildInputs;
          slotCargo = slotLimitedCargo pkgs;
          benchmarkToolsCheck = pkgs.lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
            check_benchmark_tools ${pkgs.lib.makeBinPath installedOutputs}
          '';
        in
        pkgs.runCommand "dev-shell-tools" { } ''
          fail() {
            echo "$1" >&2
            exit 1
          }
          check_shell() {
            local analyzer pattern="RUST_SRC_PATH-'([^']*)'"
            analyzer=$(PATH=$1 && command -v rust-analyzer) || fail "a dev shell has no rust-analyzer"
            [[ $analyzer == /nix/store/* ]] || fail "a dev shell takes rust-analyzer from $analyzer"
            [[ $(< "$analyzer") =~ $pattern ]] || fail "rust-analyzer sets no RUST_SRC_PATH"
            [[ -f ''${BASH_REMATCH[1]}/core/src/lib.rs ]] || fail "RUST_SRC_PATH holds no core sources"
            ! (PATH=$1 && command -v rustup) || fail "a dev shell provides rustup"
            [[ $(PATH=$1 && command -v cargo) == ${slotCargo}/bin/cargo ]] ||
              fail "a dev shell runs cargo outside its build slots"
          }
          check_benchmark_tools() {
            (PATH=$1 && command -v valgrind) >/dev/null || fail "a Linux dev shell has no valgrind"
          }
          check_shell ${pkgs.lib.makeBinPath defaultShell.nativeBuildInputs}
          ${benchmarkToolsCheck}
          touch $out
        '';

      buildSlots =
        pkgs:
        let
          recordingCargo = pkgs.writeShellScriptBin "cargo" ''
            echo $$ >"$calls/.pid-$$"
            mv "$calls/.pid-$$" "$calls/$1-''${2:-}"
            case $1 in
              build)
                until [[ -e $calls/finish-$2 ]]; do sleep 0.1; done
                ;;
              failing) exit 3 ;;
              sleep) sleep "$2" ;;
              hang)
                sleep 300 &
                echo $! >"$calls/hung-child"
                wait
                ;;
              stubborn)
                (
                  trap "" TERM
                  exec sleep 300
                ) &
                echo $! >"$calls/stubborn-child"
                wait
                ;;
              exit-124) exit 124 ;;
              read-input)
                read -r line
                echo "$line" >"$calls/input"
                ;;
              xtask) timeout 5 "$slot_limited_cargo" check inside-xtask ;;
              daemon)
                sleep 300 >/dev/null 2>&1 &
                echo $! >"$calls/daemon-process"
                ;;
            esac
          '';
          cargo = pkgs.lib.getExe' (cargoWithBuildSlots pkgs recordingCargo) "cargo";
          flock = pkgs.lib.getExe pkgs.flock;
          ps = pkgs.lib.getExe' pkgs.procps "ps";
          script = pkgs.lib.getExe' pkgs.util-linux "script";
          setsid = pkgs.lib.getExe' pkgs.util-linux "setsid";
        in
        pkgs.runCommand "build-slots" { } ''
          fail() {
            echo "$1" >&2
            exit 1
          }
          wait_for_call() {
            for _ in {1..100}; do
              [[ -e $calls/$1 ]] && return
              sleep 0.1
            done
            fail "cargo $1 never ran"
          }
          expect_no_call() {
            sleep 2
            [[ ! -e $calls/$1 ]] || fail "cargo $1 ran with no free slot"
          }
          finish() {
            touch "$calls/finish-$1"
          }
          running() {
            ${ps} -o stat= -p "$1" | grep --quiet --invert-match '^Z'
          }
          expect_stopped() {
            for _ in {1..100}; do
              running "$1" || return 0
              sleep 0.1
            done
            fail "$2"
          }
          limited_run_in_own_group() {
            (
              trap - INT QUIT
              exec ${setsid} env KNAVE_BUILD_SLOTS=1 KNAVE_BUILD_TIME_LIMIT=60 ${cargo} build "$1"
            ) &
            wait_for_call "build-$1"
          }
          expect_group_signal_stops_limited_run() {
            limited_run_in_own_group "group-$1"
            kill "-$1" -- "-$!"
            expect_stopped "$(<"$calls/build-group-$1")" "SIG$1 to the caller's group left a limited build running"
            timeout 5 env KNAVE_BUILD_SLOTS=1 ${cargo} check "after-group-$1" || fail "SIG$1 to a limited run kept its slot"
          }
          on_terminal() {
            timeout 20 ${script} --quiet --return --command "env KNAVE_BUILD_SLOTS=1 KNAVE_BUILD_TIME_LIMIT=60 ${cargo} $1" /dev/null >/dev/null
          }
          export HOME=$PWD/home calls=$PWD/calls slot_limited_cargo=${cargo}
          unset CI KNAVE_BUILD_SLOTS KNAVE_BUILD_SLOT_HELD
          mkdir -p "$HOME" "$calls"

          ${cargo} build first &
          ${cargo} build second &
          wait_for_call build-first
          wait_for_call build-second

          ${cargo} build third 2>waiting &
          third=$!
          expect_no_call build-third
          grep --quiet 'waiting for one of 2 build slots' waiting || fail "a waiting command says nothing"

          ${cargo} check --color always 2>/dev/null &
          blocked_check=$!
          expect_no_call check---color

          timeout 5 ${cargo} fmt --all || fail "fmt waited for a slot"
          timeout 5 ${cargo} --version || fail "--version waited for a slot"
          timeout 5 ${cargo} --color always metadata || fail "metadata waited for a slot"
          timeout 5 ${cargo} --color || fail "a flag with no value waited for a slot"
          timeout 5 env KNAVE_BUILD_SLOT_HELD=1 ${cargo} check nested || fail "a nested cargo waited for a slot"
          timeout 5 env CI=true ${cargo} check ci || fail "cargo waited for a slot under CI"
          timeout 5 env KNAVE_BUILD_SLOTS=3 ${cargo} check third-slot || fail "KNAVE_BUILD_SLOTS did not raise the limit"

          limit_status=0
          KNAVE_BUILD_SLOTS=0 timeout 5 ${cargo} check 2>/dev/null || limit_status=$?
          [[ $limit_status == 2 ]] || fail "a slot limit of 0 was not refused"
          finish first
          wait_for_call build-third
          finish third
          wait "$third"
          wait "$blocked_check"

          ${cargo} build killed &
          killed=$!
          wait_for_call build-killed
          ${cargo} build after-kill 2>/dev/null &
          kill -KILL "$killed"
          wait_for_call build-after-kill
          finish killed
          finish second
          finish after-kill
          wait

          KNAVE_BUILD_SLOTS=1 ${cargo} build holder &
          wait_for_call build-holder
          KNAVE_BUILD_SLOTS=1 ${cargo} build earlier 2>/dev/null &
          sleep 2
          ${flock} -n "$HOME/.cache/knave/build-slots/queue-of-1" true && fail "a waiting cargo left the queue open"
          KNAVE_BUILD_SLOTS=1 ${cargo} build later 2>later-waiting &
          sleep 2
          grep --quiet 'waiting for one of 1 build slots' later-waiting || fail "a queued command says nothing"
          finish holder
          wait_for_call build-earlier
          expect_no_call build-later
          finish earlier
          wait_for_call build-later
          finish later
          wait

          status=0
          ${cargo} failing || status=$?
          [[ $status == 3 ]] || fail "cargo exit status $status was not passed on"

          KNAVE_BUILD_SLOTS=1 ${cargo} xtask || fail "a cargo inside a slot waited for a second slot"

          KNAVE_BUILD_SLOTS=1 ${cargo} daemon
          timeout 5 env KNAVE_BUILD_SLOTS=1 ${cargo} check after-daemon || fail "a process that outlived cargo kept the slot"
          kill "$(<"$calls/daemon-process")"

          KNAVE_BUILD_SLOTS=1 ${cargo} build limit-holder &
          wait_for_call build-limit-holder
          KNAVE_BUILD_SLOTS=1 KNAVE_BUILD_TIME_LIMIT=2 ${cargo} sleep 1 2>/dev/null &
          queued=$!
          sleep 3
          finish limit-holder
          wait "$queued" || fail "KNAVE_BUILD_TIME_LIMIT counted the wait for a slot"
          wait

          status=0
          KNAVE_BUILD_TIME_LIMIT=1 timeout 10 ${cargo} sleep 30 2>stopped || status=$?
          [[ $status == 124 ]] || fail "a run past KNAVE_BUILD_TIME_LIMIT exited $status, not 124"
          grep --quiet 'stopped by KNAVE_BUILD_TIME_LIMIT=1,' stopped || fail "a stopped run says nothing"

          status=0
          KNAVE_BUILD_TIME_LIMIT=1 timeout 10 ${cargo} hang 2>/dev/null || status=$?
          [[ $status == 124 ]] || fail "a run with a hung child exited $status, not 124"
          expect_stopped "$(<"$calls/hung-child")" "a child of cargo outlived KNAVE_BUILD_TIME_LIMIT"

          status=0
          KNAVE_BUILD_TIME_LIMIT=1 timeout 20 ${cargo} stubborn 2>/dev/null || status=$?
          [[ $status == 124 ]] || fail "a run with a child that ignores SIGTERM exited $status, not 124"
          expect_stopped "$(<"$calls/stubborn-child")" "a child of cargo that ignores SIGTERM outlived KNAVE_BUILD_TIME_LIMIT"

          status=0
          KNAVE_BUILD_TIME_LIMIT=10 ${cargo} exit-124 2>own-124 || status=$?
          [[ $status == 124 ]] || fail "a cargo exit status of 124 within the limit became $status"
          [[ ! -s own-124 ]] || fail "a cargo that exited 124 itself was reported as stopped by the limit"

          expect_group_signal_stops_limited_run TERM
          expect_group_signal_stops_limited_run INT

          { sleep 2; printf '\003'; sleep 3; } | on_terminal "build ctrl-c" || true
          expect_stopped "$(<"$calls/build-ctrl-c")" "Ctrl-C on the terminal left a limited build running"

          { sleep 2; echo hello; sleep 3; } | on_terminal "read-input terminal" || fail "a limited run could not read the terminal"
          [[ $(<"$calls/input") == hello ]] || fail "a limited run read '$(<"$calls/input")' from the terminal, not hello"

          status=0
          KNAVE_BUILD_TIME_LIMIT=10 ${cargo} failing 2>within-limit || status=$?
          [[ $status == 3 ]] || fail "a run within KNAVE_BUILD_TIME_LIMIT exited $status, not 3"
          [[ ! -s within-limit ]] || fail "a run within KNAVE_BUILD_TIME_LIMIT printed $(<within-limit)"

          KNAVE_BUILD_SLOT_HELD=1 KNAVE_BUILD_TIME_LIMIT=1 ${cargo} sleep 2 || fail "a nested cargo set a limit of its own"

          for invalid in 0 ten 1.5; do
            status=0
            KNAVE_BUILD_TIME_LIMIT=$invalid timeout 5 ${cargo} check 2>refused || status=$?
            [[ $status == 2 ]] || fail "a KNAVE_BUILD_TIME_LIMIT of '$invalid' was not refused"
            grep --quiet "KNAVE_BUILD_TIME_LIMIT must be a positive whole number, not '$invalid'" refused ||
              fail "a refused KNAVE_BUILD_TIME_LIMIT of '$invalid' says nothing"
          done
          touch $out
        '';

      sourceFormat =
        pkgs: package:
        let
          inherit (pkgs) lib;
          listed = lib.filter (line: line != "") (
            lib.splitString "\n" (builtins.readFile ./xtask/source-format-paths.txt)
          );
          pathsAfter = verb: map (lib.removePrefix "${verb} ") (lib.filter (lib.hasPrefix "${verb} ") listed);
          findSources =
            checked:
            let
              directory = ./. + "/${checked}";
              skipped = lib.filter (lib.hasPrefix "${checked}/") (pathsAfter "skip");
              skipping = path: " -not -path '${directory}/${lib.removePrefix "${checked}/" path}/*'";
            in
            "find ${directory} -name '*.knv'${lib.concatMapStrings skipping skipped}";
        in
        pkgs.runCommand "source-format" { } ''
          mapfile -t source_files < <({ ${
            lib.concatMapStringsSep "; " findSources (pathsAfter "check")
          }; } | sort)
          ${lib.getExe package} fmt --check "''${source_files[@]}" ||
            { echo "run knave fmt on the files above to lay them out" >&2; exit 1; }
          touch $out
        '';

      installedProfile =
        pkgs: package:
        let
          wantedSettings = {
            codegen-units = 1;
            lto = "fat";
          };
          declaredSettings = cargoManifest.profile.${installedProfileName} or { };
          driftedSettings = pkgs.lib.filterAttrs (
            setting: value: declaredSettings.${setting} or null != value
          ) wantedSettings;
          renderedSettings = pkgs.lib.concatMapAttrsStringSep ", " (
            setting: value: "${setting} = ${builtins.toJSON value}"
          ) wantedSettings;
          builds = [
            package
            package.cargoArtifacts
          ];
          buildsOnOtherProfiles = builtins.filter (
            build: build.CARGO_PROFILE or null != installedProfileName
          ) builds;
          namesOfBuildsOnOtherProfiles = map (build: build.name) buildsOnOtherProfiles;
          profileKeepsSettings = driftedSettings == { };
          everyBuildUsesProfile = buildsOnOtherProfiles == [ ];
        in
        assert pkgs.lib.assertMsg profileKeepsSettings
          "[profile.${installedProfileName}] in Cargo.toml must set ${renderedSettings}.";
        assert pkgs.lib.assertMsg everyBuildUsesProfile
          "${pkgs.lib.concatStringsSep ", " namesOfBuildsOnOtherProfiles} must set CARGO_PROFILE = \"${installedProfileName}\".";
        pkgs.runCommand "installed-profile" { } "touch $out";
    in
    {
      packages = forEachSystem (pkgs: {
        default =
          let
            craneLib = crane.mkLib pkgs;
            emittedRustTools = pkgs.lib.makeBinPath [
              pkgs.rustc
              pkgs.rustfmt
            ];
            buildArgs = {
              pname = "knave";
              version = crateVersion;
              src = self;
              cargoExtraArgs = "--locked --package knave";
              CARGO_PROFILE = installedProfileName;
              doCheck = false;
            };
          in
          craneLib.buildPackage (
            buildArgs
            // {
              cargoArtifacts = craneLib.buildDepsOnly (buildArgs // { cargoCheckCommand = "true"; });
              nativeBuildInputs = [ pkgs.makeWrapper ];
              postInstall = ''
                wrapProgram $out/bin/knave --prefix PATH : ${emittedRustTools}
              '';
              meta = {
                description = cratePackage.description;
                mainProgram = "knave";
                license = pkgs.lib.getLicenseFromSpdxId cratePackage.license;
                platforms = supportedSystems;
              };
            }
          );
      });

      devShells = forEachSystem (pkgs: {
        default = pkgs.mkShell (
          {
            packages = [ (slotLimitedCargo pkgs) ] ++ developmentPackages pkgs;
            RUSTC_WRAPPER = pkgs.writeShellScript "sccache-with-stable-tmpdir" ''
              export TMPDIR="$HOME/.cache/sccache-tmp"
              mkdir -p "$TMPDIR"
              exec ${pkgs.lib.getExe pkgs.sccache} "$@"
            '';
            shellHook = ''
              if [[ -z ''${CI:-} ]]; then
                export CARGO_BUILD_JOBS=''${CARGO_BUILD_JOBS:-2}
                export CARGO_INCREMENTAL=0
              fi
              baseline_driver='${pkgs.lib.getExe (baselineMerger pkgs)} %O %A %B %P; status=$?; [ $status -le 2 ] && exit $status; git merge-file -L ours -L base -L theirs %A %O %B'
              if [[ -d "$(git rev-parse --show-toplevel 2> /dev/null)/xtask/baselines" ]] &&
                [[ $(git config merge.baseline.driver) != "$baseline_driver" ]]; then
                git config merge.baseline.name "add both branches' changes to each baseline entry"
                git config merge.baseline.driver "$baseline_driver"
              fi
            '';
          }
          // coverageTools pkgs
          // moldLinker pkgs
        );
        ci = pkgs.mkShell (
          {
            packages = ciPackages pkgs;
          }
          // coverageTools pkgs
        );
        mutation = pkgs.mkShell {
          packages = ciPackages pkgs ++ [ pkgs.cargo-mutants ];
        };
      });

      formatter = forEachSystem (pkgs: pkgs.nixfmt-tree);

      checks = nixpkgs.lib.genAttrs supportedSystems (system: {
        default = self.packages.${system}.default;
        agent-docs = agentDocs nixpkgs.legacyPackages.${system};
        build-slots = buildSlots nixpkgs.legacyPackages.${system};
        baseline-merger = baselineMergerCheck nixpkgs.legacyPackages.${system};
        dev-shell-tools = devShellTools nixpkgs.legacyPackages.${system} self.devShells.${system}.default;
        installed-profile =
          installedProfile nixpkgs.legacyPackages.${system}
            self.packages.${system}.default;
        source-format = sourceFormat nixpkgs.legacyPackages.${system} self.packages.${system}.default;
      });
    };
}
