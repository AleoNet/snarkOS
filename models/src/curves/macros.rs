macro_rules! impl_field_into_bigint {
    ($field: ident, $bigint: ident, $params: ident) => {
        impl<P: $params> Into<$bigint> for $field<P> {
            fn into(self) -> $bigint {
                self.into_repr()
            }
        }
    };
}

macro_rules! impl_prime_field_standard_sample {
    ($field: ident, $params: ident) => {
        impl<P: $params> rand::distributions::Distribution<$field<P>> for rand::distributions::Standard {
            #[inline]
            fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> $field<P> {
                loop {
                    let mut tmp = $field(rng.sample(rand::distributions::Standard), PhantomData);
                    // Mask away the unused bits at the beginning.
                    tmp.0
                        .as_mut()
                        .last_mut()
                        .map(|val| *val &= std::u64::MAX >> P::REPR_SHAVE_BITS);

                    if tmp.is_valid() {
                        return tmp;
                    }
                }
            }
        }
    };
}

macro_rules! impl_prime_field_from_int {
    ($field: ident, u128, $params: ident) => {
        impl<P: $params> From<u128> for $field<P> {
            fn from(other: u128) -> Self {
                let upper = (other >> 64) as u64;
                let lower = ((other << 64) >> 64) as u64;
                let mut default_int = P::BigInt::default();
                default_int.0[0] = lower;
                default_int.0[1] = upper;
                Self::from_repr(default_int)
            }
        }
    };
    ($field: ident, $int: ident, $params: ident) => {
        impl<P: $params> From<$int> for $field<P> {
            fn from(other: $int) -> Self {
                Self::from_repr(P::BigInt::from(u64::from(other)))
            }
        }
    };
}

macro_rules! sqrt_impl {
    ($Self:ident, $P:tt, $self:expr) => {{
        use crate::curves::LegendreSymbol::*;
        // https://eprint.iacr.org/2012/685.pdf (page 12, algorithm 5)
        // Actually this is just normal Tonelli-Shanks; since `P::Generator`
        // is a quadratic non-residue, `P::ROOT_OF_UNITY = P::GENERATOR ^ t`
        // is also a quadratic non-residue (since `t` is odd).
        match $self.legendre() {
            Zero => Some(*$self),
            QuadraticNonResidue => None,
            QuadraticResidue => {
                let mut z = $Self::qnr_to_t();
                let mut w = $self.pow($P::T_MINUS_ONE_DIV_TWO);
                let mut x = w * $self;
                let mut b = x * &w;

                let mut v = $P::TWO_ADICITY as usize;
                // t = self^t
                #[cfg(debug_assertions)]
                {
                    let mut check = b;
                    for _ in 0..(v - 1) {
                        check.square_in_place();
                    }
                    if !check.is_one() {
                        panic!("Input is not a square root, but it passed the QR test")
                    }
                }

                while !b.is_one() {
                    let mut k = 0usize;

                    let mut b2k = b;
                    while !b2k.is_one() {
                        // invariant: b2k = b^(2^k) after entering this loop
                        b2k.square_in_place();
                        k += 1;
                    }

                    let j = v - k - 1;
                    w = z;
                    for _ in 0..j {
                        w.square_in_place();
                    }

                    z = w.square();
                    b *= &z;
                    x *= &w;
                    v = k;
                }

                Some(x)
            }
        }
    }};
}

macro_rules! impl_prime_field_serializer {
    ($field: ident, $params: ident, $byte_size: expr) => {
        impl<P: $params> CanonicalSerializeWithFlags for $field<P> {
            #[allow(unused_qualifications)]
            fn serialize_with_flags<W: snarkos_utilities::io::Write, F: snarkos_utilities::serialize::Flags>(
                &self,
                writer: &mut W,
                flags: F,
            ) -> Result<(), snarkos_utilities::serialize::SerializationError> {
                const BYTE_SIZE: usize = $byte_size;

                let (output_bit_size, output_byte_size) =
                    snarkos_utilities::serialize::buffer_bit_byte_size($field::<P>::size_in_bits());
                if F::len() > (output_bit_size - P::MODULUS_BITS as usize) {
                    return Err(snarkos_utilities::serialize::SerializationError::NotEnoughSpace);
                }

                let mut bytes = [0u8; BYTE_SIZE];
                self.write(&mut bytes[..])?;

                bytes[output_byte_size - 1] |= flags.u8_bitmask();

                writer.write_all(&bytes[..output_byte_size])?;
                Ok(())
            }
        }

        impl<P: $params> ConstantSerializedSize for $field<P> {
            const SERIALIZED_SIZE: usize = snarkos_utilities::serialize::buffer_byte_size(
                <$field<P> as crate::curves::PrimeField>::Params::MODULUS_BITS as usize,
            );
            const UNCOMPRESSED_SIZE: usize = Self::SERIALIZED_SIZE;
        }

        impl<P: $params> CanonicalSerialize for $field<P> {
            #[allow(unused_qualifications)]
            #[inline]
            fn serialize<W: snarkos_utilities::io::Write>(
                &self,
                writer: &mut W,
            ) -> Result<(), snarkos_utilities::serialize::SerializationError> {
                self.serialize_with_flags(writer, snarkos_utilities::serialize::EmptyFlags)
            }

            #[inline]
            fn serialized_size(&self) -> usize {
                Self::SERIALIZED_SIZE
            }
        }

        impl<P: $params> CanonicalDeserializeWithFlags for $field<P> {
            #[allow(unused_qualifications)]
            fn deserialize_with_flags<R: snarkos_utilities::io::Read, F: snarkos_utilities::serialize::Flags>(
                reader: &mut R,
            ) -> Result<(Self, F), snarkos_utilities::serialize::SerializationError> {
                const BYTE_SIZE: usize = $byte_size;

                let (output_bit_size, output_byte_size) =
                    snarkos_utilities::serialize::buffer_bit_byte_size($field::<P>::size_in_bits());
                if F::len() > (output_bit_size - P::MODULUS_BITS as usize) {
                    return Err(snarkos_utilities::serialize::SerializationError::NotEnoughSpace);
                }

                let mut masked_bytes = [0; BYTE_SIZE];
                reader.read_exact(&mut masked_bytes[..output_byte_size])?;

                let flags = F::from_u8_remove_flags(&mut masked_bytes[output_byte_size - 1]);

                Ok((Self::read(&masked_bytes[..])?, flags))
            }
        }

        impl<P: $params> CanonicalDeserialize for $field<P> {
            #[allow(unused_qualifications)]
            fn deserialize<R: snarkos_utilities::io::Read>(
                reader: &mut R,
            ) -> Result<Self, snarkos_utilities::serialize::SerializationError> {
                const BYTE_SIZE: usize = $byte_size;

                let (_, output_byte_size) =
                    snarkos_utilities::serialize::buffer_bit_byte_size($field::<P>::size_in_bits());

                let mut masked_bytes = [0; BYTE_SIZE];
                reader.read_exact(&mut masked_bytes[..output_byte_size])?;
                Ok(Self::read(&masked_bytes[..])?)
            }
        }

        impl<P: $params> serde::Serialize for $field<P> {
            fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
            where
                S: serde::ser::Serializer,
            {
                use serde::ser::SerializeTuple;

                let len = self.serialized_size();
                let mut bytes = Vec::with_capacity(len);
                CanonicalSerialize::serialize(self, &mut bytes).map_err(serde::ser::Error::custom)?;

                let mut tup = s.serialize_tuple(len)?;
                for byte in &bytes {
                    tup.serialize_element(byte)?;
                }
                tup.end()
            }
        }

        impl<'de, P: $params> serde::Deserialize<'de> for $field<P> {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct SerVisitor<P>(std::marker::PhantomData<P>);

                impl<'de, P: $params> serde::de::Visitor<'de> for SerVisitor<P> {
                    type Value = $field<P>;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str("a valid field element")
                    }

                    fn visit_seq<S>(self, mut seq: S) -> Result<Self::Value, S::Error>
                    where
                        S: serde::de::SeqAccess<'de>,
                    {
                        let len = <Self::Value as ConstantSerializedSize>::SERIALIZED_SIZE;
                        let bytes: Vec<u8> = (0..len)
                            .map(|_| {
                                seq.next_element()?
                                    .ok_or_else(|| serde::de::Error::custom("could not read bytes"))
                            })
                            .collect::<Result<Vec<_>, _>>()?;

                        let res =
                            CanonicalDeserialize::deserialize(&mut &bytes[..]).map_err(serde::de::Error::custom)?;
                        Ok(res)
                    }
                }

                let visitor = SerVisitor(std::marker::PhantomData);
                deserializer.deserialize_tuple(Self::SERIALIZED_SIZE, visitor)
            }
        }
    };
}

macro_rules! impl_field_from_random_bytes_with_flags {
    ($limbs: expr) => {
        #[inline]
        fn from_random_bytes_with_flags(bytes: &[u8]) -> Option<(Self, u8)> {
            let mut result_bytes = [0u8; $limbs * 8];
            for (result_byte, in_byte) in result_bytes.iter_mut().zip(bytes.iter()) {
                *result_byte = *in_byte;
            }

            let mask: u64 = 0xffffffffffffffff >> P::REPR_SHAVE_BITS;
            // the flags will be at the same byte with the lowest shaven bits or the one after
            let flags_byte_position: usize = 7 - P::REPR_SHAVE_BITS as usize / 8;
            let flags_mask: u8 = ((1 << P::REPR_SHAVE_BITS % 8) - 1) << (8 - P::REPR_SHAVE_BITS % 8);
            // take the last 8 bytes and pass the mask
            let last_bytes = &mut result_bytes[($limbs - 1) * 8..];
            let mut flags: u8 = 0;
            for (i, (b, m)) in last_bytes.iter_mut().zip(&mask.to_le_bytes()).enumerate() {
                if i == flags_byte_position {
                    flags = *b & flags_mask
                }
                *b &= m;
            }

            <Self as CanonicalDeserialize>::deserialize(&mut &result_bytes[..])
                .ok()
                .map(|f| (f, flags))
        }
    };
}
