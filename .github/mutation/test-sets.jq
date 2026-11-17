def test_sets:
  if startswith("src/cli/") then [["--lib", "cli::"], ["--test", "cli"]]
  elif startswith("xtask/src/") then [["-p", "xtask"]]
  elif startswith("expand/src/") then [["-p", "knave-expand", "--lib"], ["--test", "stages"]]
  else [["--lib"], ["--test", "stages"]]
  end;
