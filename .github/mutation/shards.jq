def seconds:
  if startswith("expand/src/format/") then 370
  elif startswith("src/emit/") then 260
  elif startswith("src/cli/") then 50
  elif startswith("xtask/") then 5
  elif startswith("expand/src/") then 40
  else 60
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
