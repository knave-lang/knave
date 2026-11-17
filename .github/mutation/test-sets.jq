def stage_suites:
  ["expand_fixtures", "check_fixtures", "emit_fixtures"] as $first
  | [$first[] | ["--test", "stages", .]]
    + [["--test", "stages", "--", "--exact"] + [$first[] | "--skip", .]];

def test_sets:
  if startswith("src/cli/") then [["--lib", "cli::"], ["--test", "cli"]]
  elif startswith("xtask/src/") then [["-p", "xtask"]]
  elif startswith("expand/src/") then [["-p", "knave-expand", "--lib"]] + stage_suites
  else [["--lib"]] + stage_suites
  end;
