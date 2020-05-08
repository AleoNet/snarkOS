use snarkos_models::{algorithms::CommitmentScheme, dpc::DPCComponents};
use snarkos_utilities::bytes::{FromBytes, ToBytes};

use std::io::{Read, Result as IoResult, Write};

#[derive(Derivative)]
#[derivative(
    Default(bound = "C: DPCComponents"),
    Clone(bound = "C: DPCComponents"),
    Debug(bound = "C: DPCComponents")
)]
pub struct AccountPublicKey<C: DPCComponents> {
    pub public_key: <C::AddressCommitment as CommitmentScheme>::Output,
}

impl<C: DPCComponents> ToBytes for AccountPublicKey<C> {
    fn write<W: Write>(&self, writer: W) -> IoResult<()> {
        self.public_key.write(writer)
    }
}

impl<C: DPCComponents> FromBytes for AccountPublicKey<C> {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let public_key: <C::AddressCommitment as CommitmentScheme>::Output = FromBytes::read(&mut reader)?;

        Ok(Self { public_key })
    }
}
