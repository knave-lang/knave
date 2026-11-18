function identity(    field, text) {
  text = $1
  for (field = 2; field <= 6; field++)
    text = text " " $field
  return text
}

BEGIN {
  count = split(identities, lines, "\n")
  for (line = 1; line <= count; line++) {
    $0 = lines[line]
    started[identity()] = 1
  }
}

$7 !~ /^Z/ && identity() in started { print $1 }
