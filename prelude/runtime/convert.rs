#[inline]
#[track_caller]
pub fn convert<To: Integer, From: Integer>(value: From) -> To {
    let signed = From::try_from(-1_i128).is_ok();
    let converted = if signed {
        TryInto::<i128>::try_into(value)
            .ok()
            .and_then(|wide| To::try_from(wide).ok())
    } else {
        TryInto::<u128>::try_into(value)
            .ok()
            .and_then(|wide| To::try_from(wide).ok())
    };
    match converted {
        Some(converted) => converted,
        None => conversion_overflow::<To, From>(value),
    }
}

#[cold]
#[inline(never)]
#[track_caller]
fn conversion_overflow<To: Integer, From: Integer>(value: From) -> ! {
    panic!(
        "attempt to convert {value} to `{}` with overflow",
        std::any::type_name::<To>()
    )
}
