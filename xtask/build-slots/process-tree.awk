{ parent[$1] = $2 }

END {
  tree[root] = 1
  do {
    grown = 0
    for (process in parent)
      if (!(process in tree) && (parent[process] in tree)) {
        tree[process] = 1
        grown = 1
      }
  } while (grown)
  separator = ""
  for (process in tree) {
    printf "%s%s", separator, process
    separator = ","
  }
  print ""
}
