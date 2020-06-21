// https://github.com/rust-lang/rust/issues/57966 forces us to export these and
// import them via `use crate::` syntax. It'd be nice if we were able to avoid any
// macro_use/macro_export and just import the macro
#[macro_export]
macro_rules! impl_sw_curve_serializer {
    ($params: ident) => {
        // Projective Group point implementations delegate to the Affine version
        impl<P: $params> CanonicalSerialize for GroupProjective<P> {
            #[allow(unused_qualifications)]
            #[inline]
            fn serialize<W: snarkos_utilities::io::Write>(
                &self,
                writer: &mut W,
            ) -> Result<(), snarkos_errors::serialization::SerializationError> {
                CanonicalSerialize::serialize(&GroupAffine::<P>::from(*self), writer)
            }

            #[allow(unused_qualifications)]
            fn serialize_uncompressed<W: snarkos_utilities::io::Write>(
                &self,
                writer: &mut W,
            ) -> Result<(), snarkos_errors::serialization::SerializationError> {
                CanonicalSerialize::serialize_uncompressed(&GroupAffine::<P>::from(*self), writer)
            }

            #[inline]
            fn serialized_size(&self) -> usize {
                GroupAffine::<P>::from(*self).serialized_size()
            }

            #[inline]
            fn uncompressed_size(&self) -> usize {
                GroupAffine::<P>::from(*self).uncompressed_size()
            }
        }

        impl<P: $params> CanonicalDeserialize for GroupProjective<P> {
            #[allow(unused_qualifications)]
            fn deserialize<R: snarkos_utilities::io::Read>(
                reader: &mut R,
            ) -> Result<Self, snarkos_errors::serialization::SerializationError> {
                let el: GroupAffine<P> = CanonicalDeserialize::deserialize(reader)?;
                Ok(el.into())
            }

            #[allow(unused_qualifications)]
            fn deserialize_uncompressed<R: snarkos_utilities::io::Read>(
                reader: &mut R,
            ) -> Result<Self, snarkos_errors::serialization::SerializationError> {
                let el: GroupAffine<P> = CanonicalDeserialize::deserialize_uncompressed(reader)?;
                Ok(el.into())
            }
        }

        impl<P: $params> ConstantSerializedSize for GroupProjective<P> {
            const SERIALIZED_SIZE: usize = <P::BaseField as ConstantSerializedSize>::SERIALIZED_SIZE;
            const UNCOMPRESSED_SIZE: usize = 2 * <P::BaseField as ConstantSerializedSize>::SERIALIZED_SIZE;
        }

        impl<P: $params> CanonicalSerialize for GroupAffine<P> {
            #[allow(unused_qualifications)]
            #[inline]
            fn serialize<W: snarkos_utilities::io::Write>(
                &self,
                writer: &mut W,
            ) -> Result<(), snarkos_errors::serialization::SerializationError> {
                if self.is_zero() {
                    let flags = snarkos_utilities::serialize::SWFlags::infinity();
                    // Serialize 0.
                    P::BaseField::zero().serialize_with_flags(writer, flags)
                } else {
                    let flags = snarkos_utilities::serialize::SWFlags::from_y_sign(self.y > -self.y);
                    self.x.serialize_with_flags(writer, flags)
                }
            }

            #[inline]
            fn serialized_size(&self) -> usize {
                Self::SERIALIZED_SIZE
            }

            #[allow(unused_qualifications)]
            #[inline]
            fn serialize_uncompressed<W: snarkos_utilities::io::Write>(
                &self,
                writer: &mut W,
            ) -> Result<(), snarkos_errors::serialization::SerializationError> {
                let flags = if self.is_zero() {
                    snarkos_utilities::serialize::SWFlags::infinity()
                } else {
                    snarkos_utilities::serialize::SWFlags::default()
                };
                CanonicalSerialize::serialize(&self.x, writer)?;
                self.y.serialize_with_flags(writer, flags)?;
                Ok(())
            }

            #[inline]
            fn uncompressed_size(&self) -> usize {
                Self::UNCOMPRESSED_SIZE
            }
        }

        impl<P: $params> ConstantSerializedSize for GroupAffine<P> {
            const SERIALIZED_SIZE: usize = <P::BaseField as ConstantSerializedSize>::SERIALIZED_SIZE;
            const UNCOMPRESSED_SIZE: usize = 2 * <P::BaseField as ConstantSerializedSize>::SERIALIZED_SIZE;
        }

        impl<P: $params> CanonicalDeserialize for GroupAffine<P> {
            #[allow(unused_qualifications)]
            fn deserialize<R: snarkos_utilities::io::Read>(
                reader: &mut R,
            ) -> Result<Self, snarkos_errors::serialization::SerializationError> {
                let (x, flags): (P::BaseField, snarkos_utilities::serialize::SWFlags) =
                    CanonicalDeserializeWithFlags::deserialize_with_flags(reader)?;
                if flags.is_infinity() {
                    Ok(Self::zero())
                } else {
                    let p = GroupAffine::<P>::get_point_from_x(x, flags.is_positive().unwrap())
                        .ok_or(snarkos_errors::serialization::SerializationError::InvalidData)?;
                    if !p.is_in_correct_subgroup_assuming_on_curve() {
                        return Err(snarkos_errors::serialization::SerializationError::InvalidData);
                    }
                    Ok(p)
                }
            }

            #[allow(unused_qualifications)]
            fn deserialize_uncompressed<R: snarkos_utilities::io::Read>(
                reader: &mut R,
            ) -> Result<Self, snarkos_errors::serialization::SerializationError> {
                let x: P::BaseField = CanonicalDeserialize::deserialize(reader)?;
                let (y, flags): (P::BaseField, snarkos_utilities::serialize::SWFlags) =
                    CanonicalDeserializeWithFlags::deserialize_with_flags(reader)?;

                let p = GroupAffine::<P>::new(x, y, flags.is_infinity());
                if !p.is_in_correct_subgroup_assuming_on_curve() {
                    return Err(snarkos_errors::serialization::SerializationError::InvalidData);
                }
                Ok(p)
            }
        }
    };
}

#[macro_export]
macro_rules! impl_edwards_curve_serializer {
    ($params: ident) => {
        impl<P: $params> CanonicalSerialize for GroupProjective<P> {
            #[allow(unused_qualifications)]
            #[inline]
            fn serialize<W: snarkos_utilities::io::Write>(
                &self,
                writer: &mut W,
            ) -> Result<(), snarkos_errors::serialization::SerializationError> {
                CanonicalSerialize::serialize(&GroupAffine::<P>::from(*self), writer)
            }

            #[allow(unused_qualifications)]
            fn serialize_uncompressed<W: snarkos_utilities::io::Write>(
                &self,
                writer: &mut W,
            ) -> Result<(), snarkos_errors::serialization::SerializationError> {
                CanonicalSerialize::serialize_uncompressed(&GroupAffine::<P>::from(*self), writer)
            }

            #[inline]
            fn serialized_size(&self) -> usize {
                GroupAffine::<P>::from(*self).serialized_size()
            }

            #[inline]
            fn uncompressed_size(&self) -> usize {
                GroupAffine::<P>::from(*self).uncompressed_size()
            }
        }

        impl<P: $params> ConstantSerializedSize for GroupProjective<P> {
            const SERIALIZED_SIZE: usize = <P::BaseField as ConstantSerializedSize>::SERIALIZED_SIZE;
            const UNCOMPRESSED_SIZE: usize = 2 * <P::BaseField as ConstantSerializedSize>::SERIALIZED_SIZE;
        }

        impl<P: $params> CanonicalDeserialize for GroupProjective<P> {
            #[allow(unused_qualifications)]
            fn deserialize<R: snarkos_utilities::io::Read>(
                reader: &mut R,
            ) -> Result<Self, snarkos_errors::serialization::SerializationError> {
                let el: GroupAffine<P> = CanonicalDeserialize::deserialize(reader)?;
                Ok(el.into())
            }

            #[allow(unused_qualifications)]
            fn deserialize_uncompressed<R: snarkos_utilities::io::Read>(
                reader: &mut R,
            ) -> Result<Self, snarkos_errors::serialization::SerializationError> {
                let el: GroupAffine<P> = CanonicalDeserialize::deserialize_uncompressed(reader)?;
                Ok(el.into())
            }
        }

        impl<P: $params> CanonicalSerialize for GroupAffine<P> {
            #[allow(unused_qualifications)]
            #[inline]
            fn serialize<W: snarkos_utilities::io::Write>(
                &self,
                writer: &mut W,
            ) -> Result<(), snarkos_errors::serialization::SerializationError> {
                if self.is_zero() {
                    let flags = snarkos_utilities::serialize::EdwardsFlags::default();
                    // Serialize 0.
                    P::BaseField::zero().serialize_with_flags(writer, flags)
                } else {
                    let flags = snarkos_utilities::serialize::EdwardsFlags::from_y_sign(self.y > -self.y);
                    self.x.serialize_with_flags(writer, flags)
                }
            }

            #[inline]
            fn serialized_size(&self) -> usize {
                Self::SERIALIZED_SIZE
            }

            #[allow(unused_qualifications)]
            #[inline]
            fn serialize_uncompressed<W: snarkos_utilities::io::Write>(
                &self,
                writer: &mut W,
            ) -> Result<(), snarkos_errors::serialization::SerializationError> {
                self.x.serialize_uncompressed(writer)?;
                self.y.serialize_uncompressed(writer)?;
                Ok(())
            }

            #[inline]
            fn uncompressed_size(&self) -> usize {
                Self::UNCOMPRESSED_SIZE
            }
        }

        impl<P: $params> ConstantSerializedSize for GroupAffine<P> {
            const SERIALIZED_SIZE: usize = <P::BaseField as ConstantSerializedSize>::SERIALIZED_SIZE;
            const UNCOMPRESSED_SIZE: usize = 2 * <P::BaseField as ConstantSerializedSize>::SERIALIZED_SIZE;
        }

        impl<P: $params> CanonicalDeserialize for GroupAffine<P> {
            #[allow(unused_qualifications)]
            fn deserialize<R: snarkos_utilities::io::Read>(
                reader: &mut R,
            ) -> Result<Self, snarkos_errors::serialization::SerializationError> {
                let (x, flags): (P::BaseField, snarkos_utilities::serialize::EdwardsFlags) =
                    CanonicalDeserializeWithFlags::deserialize_with_flags(reader)?;
                if x == P::BaseField::zero() {
                    Ok(Self::zero())
                } else {
                    let p = GroupAffine::<P>::get_point_from_x(x, flags.is_positive())
                        .ok_or(snarkos_errors::serialization::SerializationError::InvalidData)?;
                    if !p.is_in_correct_subgroup_assuming_on_curve() {
                        return Err(snarkos_errors::serialization::SerializationError::InvalidData);
                    }
                    Ok(p)
                }
            }

            #[allow(unused_qualifications)]
            fn deserialize_uncompressed<R: snarkos_utilities::io::Read>(
                reader: &mut R,
            ) -> Result<Self, snarkos_errors::serialization::SerializationError> {
                let x: P::BaseField = CanonicalDeserialize::deserialize(reader)?;
                let y: P::BaseField = CanonicalDeserialize::deserialize(reader)?;

                let p = GroupAffine::<P>::new(x, y);
                if !p.is_in_correct_subgroup_assuming_on_curve() {
                    return Err(snarkos_errors::serialization::SerializationError::InvalidData);
                }
                Ok(p)
            }
        }
    };
}
