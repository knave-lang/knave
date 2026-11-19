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
          text = ''
            real_cargo=${pkgs.lib.getExe' realCargo "cargo"}
            flock=${pkgs.lib.getExe pkgs.flock}
            ps=${pkgs.lib.getExe' pkgs.unixtools.procps "ps"}
            awk=${pkgs.lib.getExe pkgs.gawk}
            process_tree_program=${./xtask/build-slots/process-tree.awk}
            survivors_program=${./xtask/build-slots/survivors.awk}
            slot_directory=$HOME/.cache/knave/build-slots
            slot_limit=''${KNAVE_BUILD_SLOTS:-2}
            time_limit=''${KNAVE_BUILD_TIME_LIMIT:-}
            timed_out_status=124
            longest_number=9
            read_timed_out_status=129
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
              local to_timer timer_process status=0 timer_status=0
              coproc time_limit_timer { stop_at_time_limit >/dev/null; }
              timer_process=$!
              exec {to_timer}>&"''${time_limit_timer[1]}"
              close_coprocess_pipes "''${time_limit_timer[@]}"
              run_cargo_telling_timer "$to_timer" "$@" || status=$?
              exec {to_timer}>&-
              wait "$timer_process" || timer_status=$?
              if ((timer_status == timed_out_status)); then
                status=$timed_out_status
              fi
              return "$status"
            }

            close_coprocess_pipes() {
              local output=$1 input=$2
              exec {output}<&- {input}>&-
            }

            run_cargo_telling_timer() {
              local to_timer=$1 stderr status=0
              shift
              exec {stderr}>&2
              {
                (
                  echo "$BASHPID" >&"$to_timer"
                  KNAVE_BUILD_SLOT_HELD=1 exec "$real_cargo" "$@" 2>&"$stderr" {stderr}>&-
                )
              } 2>/dev/null || status=$?
              exec {stderr}>&-
              return "$status"
            }

            stop_at_time_limit() {
              local cargo_process waited=0
              read -r cargo_process || return 0
              read -r -t "$time_limit" || waited=$?
              if ((waited >= read_timed_out_status)) && stop_if_running "$cargo_process"; then
                echo "cargo: stopped by KNAVE_BUILD_TIME_LIMIT=$time_limit, which counts seconds from when the command took a build slot" >&2 || true
                return "$timed_out_status"
              fi
            }

            stop_if_running() {
              local identities
              [[ -n $(survivors "$(identify "$1")") ]] || return 1
              trap "" TERM HUP PIPE
              identities=$(freeze_process_tree "$1")
              signal_survivors TERM "$identities"
              thaw "$identities"
              wait_for_survivors_to_stop "$identities"
              signal_survivors KILL "$identities"
            }

            freeze_process_tree() {
              local tree frozen="" identities=""
              tree=$(process_tree "$1")
              until [[ $tree == "$frozen" ]]; do
                frozen=$tree
                identities+=$(identify "$tree")$'\n'
                signal_survivors STOP "$identities"
                tree=$(process_tree "$1")
              done
              echo "$identities"
            }

            thaw() {
              local process frozen=()
              while read -r process _; do
                frozen+=("$process")
              done <<<"$1"
              kill -CONT "''${frozen[@]}" 2>/dev/null || true
            }

            wait_for_survivors_to_stop() {
              local waited
              for ((waited = 0; waited < seconds_before_kill; waited++)); do
                [[ -n $(survivors "$1") ]] || return 0
                sleep 1
              done
            }

            process_tree() {
              "$ps" -A -o pid= -o ppid= | "$awk" -v root="$1" -f "$process_tree_program"
            }

            identify() {
              "$ps" -o pid= -o lstart= -p "$1"
            }

            survivors() {
              "$ps" -A -o pid= -o lstart= -o stat= | "$awk" -v identities="$1" -f "$survivors_program"
            }

            signal_survivors() {
              local processes
              mapfile -t processes < <(survivors "$2")
              if ((''${#processes[@]} > 0)); then
                kill "-$1" "''${processes[@]}" 2>/dev/null || true
              fi
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
              if [[ ! $2 =~ ^[1-9][0-9]{0,$((longest_number - 1))}$ ]]; then
                echo "$1 must be a positive whole number of at most $longest_number digits, not '$2'" >&2
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
          check_sccache_idle_timeout() {
            local expected=$1
            source ${pkgs.writeText "shell-hook" defaultShell.shellHook}
            [[ $SCCACHE_IDLE_TIMEOUT == "$expected" ]] ||
              fail "a dev shell set SCCACHE_IDLE_TIMEOUT to '$SCCACHE_IDLE_TIMEOUT', not '$expected'"
          }
          check_shell ${pkgs.lib.makeBinPath defaultShell.nativeBuildInputs}
          (unset CI SCCACHE_IDLE_TIMEOUT && check_sccache_idle_timeout 120)
          (unset CI && SCCACHE_IDLE_TIMEOUT=30 && check_sccache_idle_timeout 30)
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
              sleep) exec sleep "$2" ;;
              hang)
                sleep 300 &
                echo $! >"$calls/hung-child"
                wait
                ;;
              deep)
                bash -c 'sleep 300 & echo $! >"$calls/grandchild"; wait' &
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
              exit-with) exit "$2" ;;
              say-error) echo "cargo error" >&2 ;;
              forker)
                while :; do
                  sleep 300 &
                  echo $! >>"$calls/forked"
                  sleep 0.002
                done
                ;;
              orphans-on-stop)
                sleep 300 &
                echo $! >"$calls/orphaned-child"
                (timeout 20 bash -c 'while read -r _ _ state _ </proc/'$$'/stat; do [[ $state != T ]] || exec kill -KILL '$$'; done' >/dev/null 2>&1 &)
                wait
                ;;
              die-by) exec env --default-signal kill -s "$2" $$ ;;
              ready-sleep)
                touch "$calls/ready"
                exec sleep "$2"
                ;;
              catch-exit-130)
                trap 'exit 130' INT
                touch "$calls/ready"
                while :; do sleep 0.1; done
                ;;
              catch-int)
                trap 'touch "$calls/caught-int"' INT
                touch "$calls/catch-int-ready"
                until [[ -e $calls/finish-catch-int ]]; do sleep 0.1; done
                ;;
              slow-to-stop)
                trap 'sleep 1; touch "$calls/stopped-cleanly"; exit 0' TERM
                sleep 300 &
                wait
                ;;
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
          ps = pkgs.lib.getExe' pkgs.unixtools.procps "ps";
          script = pkgs.lib.getExe' pkgs.util-linux "script";
          setsid = pkgs.lib.getExe' pkgs.util-linux "setsid";
          python = pkgs.lib.getExe pkgs.python3Minimal;
          waitStatus = ./xtask/build-slots/wait-status.py;
          linuxChecks = pkgs.lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
            for _ in {1..3}; do
              status=0
              KNAVE_BUILD_TIME_LIMIT=1 timeout --signal KILL 20 ${cargo} orphans-on-stop 2>/dev/null || status=$?
              [[ $status == 124 ]] || fail "a run whose cargo died while the limit stopped it exited $status, not 124"
              expect_stopped "$(<"$calls/orphaned-child")" "a child that the limit stopped stayed stopped after its parent died"
            done

            on_terminal() {
              timeout 20 ${script} --quiet --return --command "env KNAVE_BUILD_SLOTS=1 KNAVE_BUILD_TIME_LIMIT=60 ${cargo} $1" /dev/null >/dev/null
            }
            rm -f "$calls/sleep-300"
            { sleep 2; printf '\003'; sleep 3; } | on_terminal "sleep 300" || true
            expect_stopped "$(<"$calls/sleep-300")" "Ctrl-C on the terminal left a limited build running"

            { sleep 2; echo hello; sleep 3; } | on_terminal "read-input terminal" || fail "a limited run could not read the terminal"
            [[ $(<"$calls/input") == hello ]] || fail "a limited run read '$(<"$calls/input")' from the terminal, not hello"
          '';
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
              exec ${setsid} env KNAVE_BUILD_SLOTS=1 KNAVE_BUILD_TIME_LIMIT=60 ${cargo} "$@"
            ) &
            wait_for_call "$1-''${2:-}"
          }
          wait_for_sleep() {
            for _ in {1..100}; do
              [[ $(${ps} -o comm= -p "$1") == sleep ]] && return
              sleep 0.1
            done
            fail "cargo $1 never reached its sleep"
          }
          expect_same_wait_status() {
            local unlimited limited
            unlimited=$(timeout 30 env -u KNAVE_BUILD_TIME_LIMIT ${python} -I ${waitStatus} "$@" 2>/dev/null)
            limited=$(timeout 30 env KNAVE_BUILD_TIME_LIMIT=60 ${python} -I ${waitStatus} "$@" 2>/dev/null)
            [[ $unlimited == "$limited" ]] || fail "''${*: -2} ended with '$unlimited' unlimited but '$limited' under a limit"
          }
          expect_slot_held() {
            ${flock} -n "$HOME/.cache/knave/build-slots/1" true && fail "$1"
            return 0
          }
          expect_group_signal_stops_limited_run() {
            rm -f "$calls/sleep-300"
            limited_run_in_own_group sleep 300
            wait_for_sleep "$(<"$calls/sleep-300")"
            kill "-$1" -- "-$!"
            expect_stopped "$(<"$calls/sleep-300")" "SIG$1 to the caller's group left a limited build running"
            timeout 5 env KNAVE_BUILD_SLOTS=1 ${cargo} check "after-group-$1" || fail "SIG$1 to a limited run kept its slot"
          }
          export HOME=$PWD/home calls=$PWD/calls slot_limited_cargo=${cargo}
          unset CI KNAVE_BUILD_SLOTS KNAVE_BUILD_SLOT_HELD
          ulimit -c 0
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
          KNAVE_BUILD_TIME_LIMIT=60 timeout 10 ${cargo} daemon || fail "a limited cargo waited for a daemon it started"
          kill "$(<"$calls/daemon-process")"
          KNAVE_BUILD_TIME_LIMIT=60 timeout 10 bash -c "${cargo} daemon 2>&1 | cat" ||
            fail "a daemon that a limited cargo started held the caller's standard error open"
          kill "$(<"$calls/daemon-process")"

          for signal in TERM HUP; do
            for delay in 1.{0..2}{0..9}{0,5} 1.300; do
              (
                trap - INT QUIT
                exec ${setsid} env KNAVE_BUILD_TIME_LIMIT=1 ${cargo} forker 2>/dev/null
              ) &
              signalled=$!
              sleep "$delay"
              kill "-$signal" -- "-$signalled" 2>/dev/null || true
              { wait "$signalled" || true; } 2>/dev/null
              sleep 0.5
              ! ${ps} -A -o stat= | grep --quiet '^T' ||
                fail "a group SIG$signal $delay s into a limited run left processes stopped"
              xargs kill -KILL <"$calls/forked" 2>/dev/null || true
              rm -f "$calls/forked"
            done
          done

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
          [[ $(<stopped) == "cargo: stopped by KNAVE_BUILD_TIME_LIMIT=1, which counts seconds from when the command took a build slot" ]] ||
            fail "a stopped run printed '$(<stopped)', not only the limit line"

          status=0
          KNAVE_BUILD_TIME_LIMIT=1 timeout 10 ${cargo} sleep 30 2>&- || status=$?
          [[ $status == 124 ]] || fail "a run past KNAVE_BUILD_TIME_LIMIT with standard error closed exited $status, not 124"

          status=0
          KNAVE_BUILD_TIME_LIMIT=1 timeout 10 ${cargo} hang 2>/dev/null || status=$?
          [[ $status == 124 ]] || fail "a run with a hung child exited $status, not 124"
          expect_stopped "$(<"$calls/hung-child")" "a child of cargo outlived KNAVE_BUILD_TIME_LIMIT"

          status=0
          KNAVE_BUILD_TIME_LIMIT=1 timeout 10 ${cargo} deep 2>/dev/null || status=$?
          [[ $status == 124 ]] || fail "a run with a hung grandchild exited $status, not 124"
          expect_stopped "$(<"$calls/grandchild")" "a grandchild of cargo outlived KNAVE_BUILD_TIME_LIMIT"

          KNAVE_BUILD_TIME_LIMIT=10 ${cargo} say-error 2>said
          [[ $(<said) == "cargo error" ]] || fail "a limited cargo's standard error read '$(<said)', not its own message"

          status=0
          KNAVE_BUILD_TIME_LIMIT=1 timeout --signal KILL 20 ${cargo} forker 2>forker-stopped || status=$?
          [[ $status == 124 ]] || fail "a run that keeps forking exited $status, not 124"
          grep --quiet 'stopped by KNAVE_BUILD_TIME_LIMIT=1,' forker-stopped || fail "a run that keeps forking was not stopped by the limit"
          while read -r forked; do
            ! running "$forked" || fail "a process that cargo forked as the limit fired outlived the limit"
          done <"$calls/forked"

          KNAVE_BUILD_TIME_LIMIT=1 timeout 10 env --default-signal=PIPE bash -c "${cargo} sleep 30 2>&1; echo \$? >broken-pipe-status" | true
          [[ $(<broken-pipe-status) == 124 ]] ||
            fail "a run past KNAVE_BUILD_TIME_LIMIT with standard error on a broken pipe exited $(<broken-pipe-status), not 124"

          status=0
          KNAVE_BUILD_TIME_LIMIT=1 timeout 20 ${cargo} stubborn 2>/dev/null || status=$?
          [[ $status == 124 ]] || fail "a run with a child that ignores SIGTERM exited $status, not 124"
          expect_stopped "$(<"$calls/stubborn-child")" "a child of cargo that ignores SIGTERM outlived KNAVE_BUILD_TIME_LIMIT"

          (
            trap - INT QUIT
            exec ${setsid} env KNAVE_BUILD_TIME_LIMIT=1 ${cargo} stubborn 2>/dev/null
          ) &
          stopping=$!
          sleep 2.5
          kill -INT -- "-$stopping"
          expect_stopped "$(<"$calls/stubborn-child")" "SIGINT during the grace before SIGKILL spared a child of cargo"
          wait "$stopping" || true

          (
            trap - INT QUIT
            exec ${setsid} env KNAVE_BUILD_TIME_LIMIT=1 ${cargo} stubborn 2>/dev/null
          ) &
          stopping=$!
          sleep 2.5
          kill -TERM -- "-$stopping"
          expect_stopped "$(<"$calls/stubborn-child")" "SIGTERM during the grace before SIGKILL spared a child of cargo"
          { wait "$stopping" || true; } 2>/dev/null

          status=0
          KNAVE_BUILD_TIME_LIMIT=10 ${cargo} exit-124 2>own-124 || status=$?
          [[ $status == 124 ]] || fail "a cargo exit status of 124 within the limit became $status"
          [[ ! -s own-124 ]] || fail "a cargo that exited 124 itself was reported as stopped by the limit"

          expect_group_signal_stops_limited_run TERM
          expect_group_signal_stops_limited_run INT
          expect_group_signal_stops_limited_run QUIT

          for code in 130 143 147; do
            KNAVE_BUILD_TIME_LIMIT=10 timeout 5 bash -c "${cargo} exit-with $code; echo \$? >exit-$code" 2>reported-$code ||
              fail "a limited cargo that exited $code hung"
            [[ $(<exit-$code) == "$code" ]] || fail "a limited cargo that exited $code exited $(<exit-$code)"
            ! grep --quiet --extended-regexp 'Terminated|Stopped|Quit|Hangup' reported-$code ||
              fail "a limited cargo that exited $code made the wrapper die by a signal: $(<reported-$code)"
          done
          timeout 5 env KNAVE_BUILD_SLOTS=1 ${cargo} check after-exit-codes || fail "a limited cargo with a high exit code kept its slot"

          for code in {0..255}; do
            expect_same_wait_status -- ${cargo} exit-with "$code"
          done
          for signal in INT QUIT TERM HUP KILL USR1 PIPE ALRM; do
            expect_same_wait_status -- ${cargo} die-by "$signal"
          done
          for signal in INT QUIT TERM HUP; do
            expect_same_wait_status --group-signal "$signal" --ready-file "$calls/ready" --delay 0.3 -- ${cargo} ready-sleep 300
          done
          for signal in INT QUIT; do
            expect_same_wait_status --ignore INT,QUIT --group-signal "$signal" --ready-file "$calls/ready" --delay 0.3 -- ${cargo} ready-sleep 2
          done
          expect_same_wait_status --group-signal INT --ready-file "$calls/ready" --delay 0.3 -- ${cargo} catch-exit-130

          rm -f "$calls/sleep-300"
          ${setsid} env KNAVE_BUILD_SLOTS=1 KNAVE_BUILD_TIME_LIMIT=60 ${cargo} sleep 300 &
          ignoring=$!
          wait_for_call sleep-300
          wait_for_sleep "$(<"$calls/sleep-300")"
          kill -INT -- "-$ignoring"
          sleep 1
          running "$(<"$calls/sleep-300")" || fail "a limited run started with SIGINT ignored stopped on SIGINT"
          kill -TERM -- "-$ignoring"
          wait "$ignoring" || true

          ${linuxChecks}
          limited_run_in_own_group catch-int
          catching=$!
          wait_for_call catch-int-ready
          kill -INT -- "-$catching"
          wait_for_call caught-int
          sleep 1
          expect_slot_held "a program that caught SIGINT lost its slot while it ran"
          finish catch-int
          wait "$catching" || fail "a program that caught SIGINT and finished did not pass on its success"

          mkfifo output
          rm -f "$calls/sleep-300"
          cat output >/dev/null &
          reader=$!
          (
            trap - INT QUIT
            exec ${setsid} bash -c "KNAVE_BUILD_SLOTS=1 KNAVE_BUILD_TIME_LIMIT=60 ${cargo} sleep 300; touch $calls/caller-continued" >output 2>&1
          ) &
          piped=$!
          wait_for_call sleep-300
          wait_for_sleep "$(<"$calls/sleep-300")"
          kill -INT -- "-$piped"
          expect_stopped "$reader" "a reader of a limited run saw no end of output after SIGINT"
          wait "$piped" || true
          [[ ! -e $calls/caller-continued ]] || fail "a limited run stopped by SIGINT exited instead of dying by SIGINT, so its caller went on"

          cat output >/dev/null &
          reader=$!
          KNAVE_BUILD_TIME_LIMIT=60 ${cargo} build wrapper-killed >output 2>&1 &
          wrapper=$!
          wait_for_call build-wrapper-killed
          kill -TERM "$wrapper"
          finish wrapper-killed
          expect_stopped "$reader" "the timer of a killed wrapper held its output open"

          KNAVE_BUILD_TIME_LIMIT=61 ${cargo} sleep 0
          ! ${ps} -A -o args= | grep --quiet --line-regexp '.*/bin/carg[o] sleep 0' || fail "a limited run that finished left its timer running"

          status=0
          KNAVE_BUILD_TIME_LIMIT=1 timeout 20 ${cargo} slow-to-stop 2>/dev/null || status=$?
          [[ $status == 124 ]] || fail "a run that stops slowly on SIGTERM exited $status, not 124"
          [[ -e $calls/stopped-cleanly ]] || fail "the limit killed cargo before it could stop on SIGTERM"

          status=0
          KNAVE_BUILD_TIME_LIMIT=10 ${cargo} failing 2>within-limit || status=$?
          [[ $status == 3 ]] || fail "a run within KNAVE_BUILD_TIME_LIMIT exited $status, not 3"
          [[ ! -s within-limit ]] || fail "a run within KNAVE_BUILD_TIME_LIMIT printed $(<within-limit)"

          KNAVE_BUILD_SLOT_HELD=1 KNAVE_BUILD_TIME_LIMIT=1 ${cargo} sleep 2 || fail "a nested cargo set a limit of its own"

          for invalid in 0 ten 1.5 1000000000; do
            status=0
            KNAVE_BUILD_TIME_LIMIT=$invalid timeout 5 ${cargo} check 2>refused || status=$?
            [[ $status == 2 ]] || fail "a KNAVE_BUILD_TIME_LIMIT of '$invalid' was not refused"
            grep --quiet "KNAVE_BUILD_TIME_LIMIT must be a positive whole number of at most 9 digits, not '$invalid'" refused ||
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
                export SCCACHE_IDLE_TIMEOUT=''${SCCACHE_IDLE_TIMEOUT:-120}
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
        dev-shell-tools = devShellTools nixpkgs.legacyPackages.${system} self.devShells.${system}.default;
        installed-profile =
          installedProfile nixpkgs.legacyPackages.${system}
            self.packages.${system}.default;
        source-format = sourceFormat nixpkgs.legacyPackages.${system} self.packages.${system}.default;
      });
    };
}
