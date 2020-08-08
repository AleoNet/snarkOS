use crate::gadgets::utilities::int::{Int128, Int16, Int32, Int64, Int8};
use snarkos_errors::gadgets::SynthesisError;

macro_rules! eq_gadget_impl {
    ($($gadget: ident)*) => ($(
        impl PartialEq for $gadget {
            fn eq(&self, other: &Self) -> bool {
                !self.value.is_none() && !other.value.is_none() && self.value == other.value
            }
        }

        impl Eq for $gadget {}
    )*)
}

eq_gadget_impl!(Int8 Int16 Int32 Int64 Int128);
