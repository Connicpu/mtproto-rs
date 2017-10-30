use num_traits::cast::cast;
use num_traits::int::PrimInt;

use error::{self, ErrorKind};


pub(crate) fn safe_int_cast<T: PrimInt + Copy, U: PrimInt>(n: T) -> error::Result<U> {
    cast(n).ok_or_else(|| {
        let upcasted = cast::<T, u64>(n).unwrap();    // Shouldn't panic
        ErrorKind::IntegerCast(upcasted).into()
    })
}
