def first_stage_suites: ["expand_fixtures", "check_fixtures", "emit_fixtures"];

def stage_suites:
  [first_stage_suites[] | ["--test", "stages", "--", "--exact", .]]
  + [["--test", "stages", "--", "--exact"] + [first_stage_suites[] | "--skip", .]];

def diagnostic_suites: [["--test", "diagnostic_codes", "--test", "diagnostic_snapshots"]];

def test_sets:
  if startswith("src/cli/") then [["--lib", "cli::"], ["--test", "cli"], ["--lib"]] + stage_suites + diagnostic_suites
  elif startswith("xtask/src/") then [["-p", "xtask"]]
  elif startswith("expand/src/") then [["-p", "knave-expand", "--lib"]] + stage_suites + [["--lib"], ["--test", "cli"]] + diagnostic_suites
  else [["--lib"]] + stage_suites + [["--test", "cli"]] + diagnostic_suites
  end;
