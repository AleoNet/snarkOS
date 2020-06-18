use crate::{
    error,
    io::{Read, Result as IoResult, Write},
    Vec,
};

pub trait ToBytes {
    /// Serializes `self` into `writer`.
    fn write<W: Write>(&self, writer: W) -> IoResult<()>;
}

pub trait FromBytes: Sized {
    /// Reads `Self` from `reader`.
    fn read<R: Read>(reader: R) -> IoResult<Self>;
}

macro_rules! array_bytes {
    ($N:expr) => {
        impl ToBytes for [u8; $N] {
            #[inline]
            fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
                writer.write_all(self)
            }
        }

        impl FromBytes for [u8; $N] {
            #[inline]
            fn read<R: Read>(mut reader: R) -> IoResult<Self> {
                let mut arr = [0u8; $N];
                reader.read_exact(&mut arr)?;
                Ok(arr)
            }
        }

        impl ToBytes for [u16; $N] {
            #[inline]
            fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
                for num in self {
                    writer.write_all(&num.to_le_bytes())?;
                }
                Ok(())
            }
        }

        impl FromBytes for [u16; $N] {
            #[inline]
            fn read<R: Read>(mut reader: R) -> IoResult<Self> {
                let mut res = [0u16; $N];
                for num in res.iter_mut() {
                    let mut bytes = [0u8; 2];
                    reader.read_exact(&mut bytes)?;
                    *num = u16::from_le_bytes(bytes);
                }
                Ok(res)
            }
        }

        impl ToBytes for [u32; $N] {
            #[inline]
            fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
                for num in self {
                    writer.write_all(&num.to_le_bytes())?;
                }
                Ok(())
            }
        }

        impl FromBytes for [u32; $N] {
            #[inline]
            fn read<R: Read>(mut reader: R) -> IoResult<Self> {
                let mut res = [0u32; $N];
                for num in res.iter_mut() {
                    let mut bytes = [0u8; 4];
                    reader.read_exact(&mut bytes)?;
                    *num = u32::from_le_bytes(bytes);
                }
                Ok(res)
            }
        }

        impl ToBytes for [u64; $N] {
            #[inline]
            fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
                for num in self {
                    writer.write_all(&num.to_le_bytes())?;
                }
                Ok(())
            }
        }

        impl FromBytes for [u64; $N] {
            #[inline]
            fn read<R: Read>(mut reader: R) -> IoResult<Self> {
                let mut res = [0u64; $N];
                for num in res.iter_mut() {
                    let mut bytes = [0u8; 8];
                    reader.read_exact(&mut bytes)?;
                    *num = u64::from_le_bytes(bytes);
                }
                Ok(res)
            }
        }
    };
}

array_bytes!(0);
array_bytes!(1);
array_bytes!(2);
array_bytes!(3);
array_bytes!(4);
array_bytes!(5);
array_bytes!(6);
array_bytes!(7);
array_bytes!(8);
array_bytes!(9);
array_bytes!(10);
array_bytes!(11);
array_bytes!(12);
array_bytes!(13);
array_bytes!(14);
array_bytes!(15);
array_bytes!(16);
array_bytes!(17);
array_bytes!(18);
array_bytes!(19);
array_bytes!(20);
array_bytes!(21);
array_bytes!(22);
array_bytes!(23);
array_bytes!(24);
array_bytes!(25);
array_bytes!(26);
array_bytes!(27);
array_bytes!(28);
array_bytes!(29);
array_bytes!(30);
array_bytes!(31);
array_bytes!(32);

/// Takes as input a sequence of structs, and converts them to a series of
/// bytes. All traits that implement `Bytes` can be automatically converted to
/// bytes in this manner.
#[macro_export]
macro_rules! to_bytes {
    ($($x:expr),*) => ({
        let mut buf = $crate::vec![];
        {$crate::push_to_vec!(buf, $($x),*)}.map(|_| buf)
    });
}

#[macro_export]
macro_rules! push_to_vec {
    ($buf:expr, $y:expr, $($x:expr),*) => ({
        {
            ToBytes::write(&$y, &mut $buf)
        }.and({$crate::push_to_vec!($buf, $($x),*)})
    });

    ($buf:expr, $x:expr) => ({
        ToBytes::write(&$x, &mut $buf)
    })
}

impl ToBytes for u8 {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        writer.write_all(&[*self])
    }
}

impl FromBytes for u8 {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let mut byte = [0u8];
        reader.read_exact(&mut byte)?;
        Ok(byte[0])
    }
}

impl ToBytes for u16 {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        writer.write_all(&self.to_le_bytes())
    }
}

impl FromBytes for u16 {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let mut bytes = [0u8; 2];
        reader.read_exact(&mut bytes)?;
        Ok(u16::from_le_bytes(bytes))
    }
}

impl ToBytes for u32 {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        writer.write_all(&self.to_le_bytes())
    }
}

impl FromBytes for u32 {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let mut bytes = [0u8; 4];
        reader.read_exact(&mut bytes)?;
        Ok(u32::from_le_bytes(bytes))
    }
}

impl ToBytes for u64 {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        writer.write_all(&self.to_le_bytes())
    }
}

impl FromBytes for u64 {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let mut bytes = [0u8; 8];
        reader.read_exact(&mut bytes)?;
        Ok(u64::from_le_bytes(bytes))
    }
}

impl ToBytes for u128 {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        writer.write_all(&self.to_le_bytes())
    }
}

impl FromBytes for u128 {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let mut bytes = [0u8; 16];
        reader.read_exact(&mut bytes)?;
        Ok(u128::from_le_bytes(bytes))
    }
}

impl ToBytes for i64 {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        writer.write_all(&self.to_le_bytes())
    }
}

impl FromBytes for i64 {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let mut bytes = [0u8; 8];
        reader.read_exact(&mut bytes)?;
        Ok(i64::from_le_bytes(bytes))
    }
}

impl ToBytes for () {
    #[inline]
    fn write<W: Write>(&self, _writer: W) -> IoResult<()> {
        Ok(())
    }
}

impl FromBytes for () {
    #[inline]
    fn read<R: Read>(_bytes: R) -> IoResult<Self> {
        Ok(())
    }
}

impl ToBytes for bool {
    #[inline]
    fn write<W: Write>(&self, writer: W) -> IoResult<()> {
        u8::write(&(*self as u8), writer)
    }
}

impl FromBytes for bool {
    #[inline]
    fn read<R: Read>(reader: R) -> IoResult<Self> {
        match u8::read(reader) {
            Ok(0) => Ok(false),
            Ok(1) => Ok(true),
            Ok(_) => Err(error("FromBytes::read failed")),
            Err(err) => Err(err),
        }
    }
}

impl<T: ToBytes> ToBytes for Vec<T> {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        for item in self {
            item.write(&mut writer)?;
        }
        Ok(())
    }
}

impl<'a, T: 'a + ToBytes> ToBytes for &'a [T] {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        for item in *self {
            item.write(&mut writer)?;
        }
        Ok(())
    }
}

impl<'a, T: 'a + ToBytes> ToBytes for &'a T {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        (*self).write(&mut writer)
    }
}

#[cfg(test)]
mod test {
    use super::ToBytes;
    use crate::Vec;
    #[test]
    fn test_macro_empty() {
        let array: Vec<u8> = vec![];
        let bytes: Vec<u8> = to_bytes![array].unwrap();
        assert_eq!(&bytes, &array);
        assert_eq!(bytes.len(), 0);
    }

    #[test]
    fn test_macro() {
        let array1 = [1u8; 32];
        let array2 = [2u8; 16];
        let array3 = [3u8; 8];
        let bytes = to_bytes![array1, array2, array3].unwrap();
        assert_eq!(bytes.len(), 56);

        let mut actual_bytes = Vec::new();
        actual_bytes.extend_from_slice(&array1);
        actual_bytes.extend_from_slice(&array2);
        actual_bytes.extend_from_slice(&array3);
        assert_eq!(bytes, actual_bytes);
    }
}
