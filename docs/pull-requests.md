# Pull requests

**Every rule in this file binds each pull request and each push to one.** CI skips a draft pull request and runs on a ready one.

1. If the branch has no pull request, open one with `gh pr create --draft`.
2. You MAY push to a draft pull request at any time.
3. When the working commands pass, run `gh pr ready`.
4. Run `gh pr checks --watch --interval 60`.

## Issues

**If a pull request fixes a GitHub issue, end its description with `Closes #<number>` for each fixed issue.** The merge closes each issue that a `Closes` line names.

**If a pull request addresses a GitHub issue only in part, name the issue in its description with `Part of #<number>`.** In the same description, state what the issue still needs.

## Release labels

**A merge to `main` releases nothing.** The project owner cuts a release with the Release workflow's `Run workflow` button on `main`.

**The release takes the largest bump among the labels of the pull requests merged since the last version on crates.io.**

| Label | The change | Example |
| --- | --- | --- |
| none | breaks no program that compiled and ran | `v0.1.4` to `v0.1.5` |
| `release:minor` | before 1.0, makes a program that compiled and ran fail to compile or run | `v0.1.5` to `v0.2.0` |
| `release:major` | is the release that the project owner names as 1.0 | `v0.2.0` to `v1.0.0` |

**If a fix changes emitted Rust that rustc refused, leave the label off.** That fix is a patch.

**The person who merges the pull request applies the release label.** NEVER apply a release label to a pull request you do not merge.

## CI failures

**The `front-end instruction counts` job posts a comparison of instruction counts and counters, and NEVER fails on a rise in an instruction count.** The job fails when a corpus program's `allocator.allocated_bytes`, `allocator.allocations` or `allocator.peak_live_bytes` rises more than 1% and no `Counters-Accepted:` trailer in a commit of the pull request names the program. The instruction counts leave out the allocator's own work, and the method text in the job's run summary, linked as `Every stage` from the counts comment, says what else they leave out, so those three counters are what show allocation growth. The job reads the base counts from the `front-end base counts` job, which runs beside it. If either job fails in any other step, its tooling broke: treat the failure as a CI failure.

**If the job fails on allocation growth, stop and report the programs and counters it names to the project owner.** NEVER add a `Counters-Accepted:` trailer before the project owner approves the rise.

**To read a program's counters, such as its `allocator.allocations`, read the job's comment on a ready pull request.** Before the pull request is ready, run `cargo xtask bench measure --save <file>` on Linux and read the program's line in `<file>`. Compare a local count only with another local count, and CI's only with CI's: the two environments give different counts for one tree.

**On a pull request, the `File sizes` and `Design rules` steps refuse a new or raised entry in a file under `xtask/baselines/`.** A `Baseline-Accepted: <baseline> <subject>` trailer in a commit of the pull request lets the entry pass. `<baseline>` is the baseline's file name without `.txt`, such as `production_lines`, and one trailer MAY list several subjects of that baseline. If the base has no baseline file at that path, every entry counts as new. If the step fails on such an entry, stop and report the entry to the project owner. NEVER add a `Baseline-Accepted:` trailer before the project owner approves the entry.

**If CI fails, run these steps:**

1. Check whether `main` fails the same check, as the next section says. If it does, stop and show the failing output to the project owner.
2. Reproduce the failure with the narrowest working command. If no working command reproduces the failure, read the cause from the CI log.
3. Fix the cause, push, and run `gh pr checks --watch --interval 60` again.

**NEVER reproduce a CI failure with a command that `AGENTS.md` bans locally.**

**If the `source-format` flake check fails, lay out each file it names with `cargo run --quiet -- fmt <file>`.** The check names each file by its Nix store path. Run the command on the same path in the repository.

**If the `commit builds` job fails, rewrite the commit that its error names:** a later fix commit leaves that commit broken.

**If the same failure survives three fixes, stop and show the failing output to the project owner.**

## The check on `main`

1. Run `gh run list --branch main --workflow CI --status completed --limit 5`.
2. Take the newest run whose conclusion is not `cancelled` and whose title is not a release subject such as `Release v0.8.22`: CI skips every job of a release commit. If no run qualifies, treat `main` as passing.
3. Run `gh run view <id> --json jobs --jq '.jobs[] | {name, conclusion}'` on that run.
4. If the job that failed on the pull request shows `failure` there, `main` fails the same check.

## Probe binaries

**The `flake` job pushes the compiler of each ready pull request and each `main` commit except a release commit to the project's Cachix.** A pull request's binary holds the pull request merged into its base. A release commit changes only the version, so the block below takes a release's first parent as the base.

**If you probe a pull request's unedited code, download the binaries of the pull request and its base with this block:**

```sh
git fetch origin refs/pull/<number>/merge
merge=$(git rev-parse FETCH_HEAD)
base=$(git rev-parse "$merge^1")
[[ $(git log -1 --format=%s "$base") =~ ^Release\ v[0-9]+\.[0-9]+\.[0-9]+$ ]] && base=$(git rev-parse "$base^1")
merge_path=$(nix eval --raw "git+file://$PWD?rev=$merge#default")
base_path=$(nix eval --raw "git+file://$PWD?rev=$base#default")
nix-store --realise "$merge_path" "$base_path"
```

**Run each binary as `"$merge_path/bin/knave"` or `"$base_path/bin/knave"`.** Nix checks the signature of each download.

**If `nix-store --realise` reports `no substituter that can build it`, run `gh pr checks <number> --watch --interval 60`, then rerun the block.** If the second run reports the same error, stop and report the commit to the project owner. NEVER build a release binary to replace the download.

**A downloaded binary has no `counters` feature.** Read counters from the `front-end instruction counts` job.

### Without the substituter

**If `nix config show substituters` does not list `https://rslisp.cachix.org`, run this block in place of `nix eval` and `nix-store --realise`:**

```sh
cache=https://rslisp.cachix.org
for commit in "$merge" "$base"; do
  store_path=$(nix eval --raw "git+file://$PWD?rev=$commit#default")
  name=$(basename "$store_path")
  narinfo=$(curl --fail --silent --show-error "$cache/${name%%-*}.narinfo") || break
  nar=$(sed -n 's/^URL: //p' <<< "$narinfo")
  probe=$PWD/target/scratch/probe-$commit
  rm -rf "$probe"
  curl --fail --silent --show-error "$cache/$nar" | zstd --decompress | nix-store --restore "$probe"
  [ "sha256:$(nix-hash --type sha256 --base32 "$probe")" = "$(sed -n 's/^NarHash: //p' <<< "$narinfo")" ] ||
    echo "NarHash mismatch: $commit"
done
```

**This block checks the NarHash and no signature.** If the block prints `NarHash mismatch`, stop and report the commit to the project owner.

**If `curl` reports a 404, run `gh pr checks <number> --watch --interval 60`, then rerun the block.** If the second run reports a 404, stop and report the commit to the project owner. NEVER build a release binary to replace the download.

**Run each binary as `target/scratch/probe-<commit>/bin/.knave-wrapped` inside the dev shell:** `bin/knave` points at a store path that this block does not download.

## Debug binaries

**The `agreement` job uploads the debug `knave` of each ready pull request.** The artifact's name is `knave-debug-<head commit>`. GitHub keeps the artifact for three days.

**The binary holds the head merged with the base branch as of the CI run.** The binary lacks every commit that the base branch gained after the run started. The artifact's `base` file names the commit of the base branch that the binary holds.

**The binary runs only on Linux x86-64, in a dev shell of the same `flake.lock`.** The binary loads its C library from the Nix store paths of the CI run.

**If you probe a ready pull request with a debug `knave`, first run this block in the dev shell:**

```sh
pull_request=<number>
read -r fork head base < <(gh pr view "$pull_request" --json isCrossRepository,headRefOid,baseRefName --jq '"\(.isCrossRepository) \(.headRefOid) \(.baseRefName)"')
directory=target/ci-debug/$head
if [ -z "$head" ]; then echo "Pull request $pull_request not found"; false
elif [ "$fork" = true ]; then echo "Pull request $pull_request comes from a fork"; false
elif ! run=$(gh run list --commit "$head" --workflow CI --event pull_request --limit 1 --json databaseId --jq '.[0].databaseId') || [ -z "$run" ]; then echo "No CI run for $head"; false
elif ! rm -rf "$directory" || ! gh run download "$run" --name "knave-debug-$head" --dir "$directory"; then echo "Run $run has no debug knave for $head. Run status: $(gh run view "$run" --json status --jq .status)"; false
elif ! chmod +x "$directory/knave" || ! "$directory/knave" --version > /dev/null; then echo "The debug knave does not start on this machine"; false
elif ! git fetch --quiet origin "$base"; then echo "Cannot fetch $base from origin"; false
else
  git merge-base --is-ancestor FETCH_HEAD "$(cat "$directory/base")" || echo "Warning: the debug knave lacks commits that $base gained since the CI run"
  realpath "$directory/knave"
fi
```

**NEVER download the debug `knave` of a pull request from a fork.** A fork's run can upload any file under the artifact's name. The block refuses a pull request from a fork.

**Run each probe with the path that the block prints.**

**Run `knave run` and `knave build` on a copy of the program under `target/scratch/`.** Each command writes a `target/` directory beside the program.

**If the block warns that the binary lacks base commits, check whether a probe needs one.** If a probe does, build the compiler with `cargo build --bin knave`.

**If the `ci.yml` of a run's merge commit lacks the upload step, the run uploads no debug `knave`.** Check the merge commit of the pull request with `git fetch origin refs/pull/<number>/merge && git grep --quiet knave-debug FETCH_HEAD -- .github/workflows/ci.yml`.

**If the block reports no debug `knave` for an unfinished run and that check passes, wait for the run.** Run `gh pr checks <number> --watch --interval 60`, then rerun the block once.

**Otherwise, if the block or its rerun exits nonzero, build the compiler with `cargo build --bin knave`.** Probe `target/debug/knave`.

## Callgrind profiles

**The `front-end instruction counts` job uploads the merged pull request's profiles as the `callgrind-profiles` artifact.** GitHub keeps the artifact for seven days. The artifact holds no profiles of the base.

**If you need a ready pull request's profiles, NEVER run callgrind locally.** Download the profiles:

```sh
run=$(gh run list --branch <branch> --workflow CI --event pull_request --limit 1 --json databaseId --jq '.[0].databaseId')
gh run download "$run" --name callgrind-profiles --dir target/scratch/profiles
```

**Read a downloaded profile with `callgrind_annotate <profile>`.**

**If `gh run download` finds no artifact, run `gh pr checks <number> --watch --interval 60`, then rerun the block.** If the second run finds no artifact, stop and report the pull request to the project owner.

## Mutants

**Run the hand-written mutants of a pull request with the `Pull request mutants` workflow.** The workflow tests each mutant on GitHub's runners and reports a verdict for each.

**A manifest is a JSON array with one object per mutant.** Each object has these fields:

| Field | Value |
| --- | --- |
| `name` | A name that tells the mutant apart in the results, with no tab or newline |
| `file` | The path of a tracked file outside `.github/`, written from the repository root with no `./` or `..` |
| `old` | Text that occurs exactly once in `file` at the tested commit |
| `new` | The text that replaces `old` |
| `tests` | A list of test sets, each a non-empty list of `cargo test` arguments with no tab or newline |

**The workflow runs the test sets of a mutant in order and stops at the first set that fails.** Leading `NAME=value` items of a test set become environment variables, such as `INSTA_GLOB_FILTER=**/<fixture file name>`.

```json
[{"name": "a hidden definition is a candidate", "file": "src/lint/definition_reach.rs", "old": "!item.hidden && ", "new": "", "tests": [["--test", "stages", "lint_fixtures"], ["--lib", "lint::"]]}]
```

**Dispatch the workflow on the pull request's head branch, NEVER on `main`.** `main`'s jobs restore the build cache of a run on `main`. The `plan` job refuses any other branch. If the head branch was cut before the workflow reached `main`, rebase the branch onto `main` first.

**To run a manifest, follow these steps:**

1. Dispatch the workflow on the pull request's head commit with a tag that no other run uses:

   ```sh
   branch=$(gh pr view <number> --json headRefName --jq .headRefName)
   commit=$(gh pr view <number> --json headRefOid --jq .headRefOid)
   tag=mutants-$(date +%s)
   gh workflow run pr-mutants.yml --ref "$branch" -f pull_request=<number> -f commit="$commit" -f tag="$tag" -f mutants="$(gzip -c <manifest> | base64 -w0)"
   ```

2. Find the run's ID with `gh run list --workflow pr-mutants.yml --limit 20 --json databaseId,displayTitle --jq ".[] | select(.displayTitle | endswith(\" $tag\")) | .databaseId"`. If the command prints nothing, rerun it after 60 seconds, at most six times. If the sixth run prints nothing, report the tag to the project owner.
3. Run `gh run watch <run id> --interval 60 --exit-status` as a background shell command. The command exits when the run ends.
4. Download the results with `gh run download <run id> --name mutant-results --dir target/scratch/mutants-<run id>`.
5. Read `summary.md` in that directory. It names each mutant's verdict and log file.

**If `gh workflow run` refuses the manifest as too large, split the manifest and dispatch each part with its own tag.**

**If the download finds no `mutant-results` artifact, read the failure with `gh run view <run id> --log-failed`.** The `plan` job fails in each of these cases:

- The run is on the default branch, or the workflow cannot tell the default branch.
- The pull request's head branch is the default branch.
- The run is on any ref but the pull request's head branch.
- The commit is not a full SHA of a commit of the pull request.
- The pull request comes from a fork.
- `test-seconds` is less than 1.
- The manifest is not a gzip compressed, base64 encoded JSON array.
- The manifest is empty, or an entry breaks the manifest format.

**The run fails when a mutant is missed, unapplied or invalid, or when a shard does not finish.** Act on each verdict:

| Verdict | Meaning | Action |
| --- | --- | --- |
| `caught` | A test set failed | None |
| `missed` | Every test set passed | Add a test that fails on the mutant, or show that the mutant does not change behavior |
| `timeout` | A test set ran longer than `test-seconds`, 300 by default, or its build ran longer than 25 minutes | Read the log. If the tests hang on the mutant, count the mutant as caught. Otherwise, dispatch the mutant again with a larger `test-seconds`. |
| `unviable` | The code does not compile with the mutant | Rewrite the mutant so that it compiles |
| `invalid` | A test set fails without the mutant, or `cargo test` refused it without a compile error | Fix the test set and dispatch the mutant again |
| `unapplied` | The entry cannot become a change to its file | Fix the entry as its verdict says and dispatch it again |
