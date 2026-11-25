def seconds:
  if startswith("expand/src/format/") then 450
  elif startswith("src/types/") then 190
  elif startswith("src/ownership/") then 150
  elif startswith("src/emit/") then 135
  elif startswith("src/cli/") then 50
  elif startswith("xtask/agree/") then 90
  elif startswith("xtask/") then 5
  elif startswith("expand/src/") then 80
  else 120
  end;

def estimated_seconds: [.[] | .file | seconds];

def estimated_total: estimated_seconds | add // 0;

def shard_count($budget; $most):
  estimated_total as $total
  | if $total == 0 then 0 else [[($total / $budget | ceil), 1] | max, $most] | min end;

def shard_assignment($count):
  estimated_seconds as $weights
  | reduce ([range($weights | length)] | sort_by(-$weights[.]))[] as $index
      ({loads: [range($count) | 0], shards: []};
        (.loads | index(min)) as $shard
        | .loads[$shard] += $weights[$index]
        | .shards[$index] = $shard)
  | .shards;
