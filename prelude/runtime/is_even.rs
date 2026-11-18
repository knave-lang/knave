pub fn is_even<N: Integer>(value: N) -> bool {
    value % N::from_literal(2) == N::from_literal(0)
}
