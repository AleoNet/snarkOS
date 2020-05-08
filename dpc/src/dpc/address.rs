use snarkos_models::{
    algorithms::{CommitmentScheme, SignatureScheme, PRF},
    dpc::{AccountScheme, DPCComponents},
};
use snarkos_utilities::bytes::{FromBytes, ToBytes};

use std::io::{Read, Result as IoResult, Write};

#[derive(Derivative)]
#[derivative(Clone(bound = "C: DPCComponents"))]
pub struct Account<C: DPCComponents> {
    pub public_key: AccountPublicKey<C>,
    pub private_key: AccountPrivateKey<C>,
}

impl<C: DPCComponents> AccountScheme for Account<C> {
    type AccountPrivateKey = AccountPrivateKey<C>;
    type AccountPublicKey = AccountPublicKey<C>;
}

impl<C: DPCComponents> ToBytes for Account<C> {
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.public_key.write(&mut writer)?;
        self.private_key.write(&mut writer)
    }
}

impl<C: DPCComponents> FromBytes for Account<C> {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let public_key: AccountPublicKey<C> = FromBytes::read(&mut reader)?;
        let private_key: AccountPrivateKey<C> = FromBytes::read(&mut reader)?;

        Ok(Self {
            public_key,
            private_key,
        })
    }
}

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

#[derive(Derivative)]
#[derivative(Default(bound = "C: DPCComponents"), Clone(bound = "C: DPCComponents"))]
pub struct AccountPrivateKey<C: DPCComponents> {
    pub pk_sig: <C::Signature as SignatureScheme>::PublicKey,
    pub sk_sig: <C::Signature as SignatureScheme>::PrivateKey,
    pub sk_prf: <C::PRF as PRF>::Seed,
    pub metadata: [u8; 32],
    pub r_pk: <C::AddressCommitment as CommitmentScheme>::Randomness,
}

impl<C: DPCComponents> ToBytes for AccountPrivateKey<C> {
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.pk_sig.write(&mut writer)?;
        self.sk_sig.write(&mut writer)?;
        self.sk_prf.write(&mut writer)?;
        self.metadata.write(&mut writer)?;
        self.r_pk.write(&mut writer)
    }
}

impl<C: DPCComponents> FromBytes for AccountPrivateKey<C> {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let pk_sig: <C::Signature as SignatureScheme>::PublicKey = FromBytes::read(&mut reader)?;
        let sk_sig: <C::Signature as SignatureScheme>::PrivateKey = FromBytes::read(&mut reader)?;
        let sk_prf: <C::PRF as PRF>::Seed = FromBytes::read(&mut reader)?;
        let metadata: [u8; 32] = FromBytes::read(&mut reader)?;
        let r_pk: <C::AddressCommitment as CommitmentScheme>::Randomness = FromBytes::read(&mut reader)?;

        Ok(Self {
            pk_sig,
            sk_sig,
            sk_prf,
            metadata,
            r_pk,
        })
    }
}
