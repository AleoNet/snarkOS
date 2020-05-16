use crate::account_format;
use snarkos_errors::objects::AccountError;
use snarkos_models::{
    algorithms::{CommitmentScheme, SignatureScheme, PRF},
    dpc::DPCComponents,
};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    rand::UniformRand,
};

use base58::ToBase58;
use rand::Rng;
use std::{
    fmt,
    io::{Read, Result as IoResult, Write},
};

#[derive(Derivative)]
#[derivative(
    Default(bound = "C: DPCComponents"),
    Clone(bound = "C: DPCComponents"),
    PartialEq(bound = "C: DPCComponents"),
    Eq(bound = "C: DPCComponents")
)]
pub struct AccountPrivateKey<C: DPCComponents> {
    pub pk_sig: <C::Signature as SignatureScheme>::PublicKey,
    pub sk_sig: <C::Signature as SignatureScheme>::PrivateKey,
    pub sk_prf: <C::PRF as PRF>::Seed,
    pub metadata: [u8; 32],
    pub r_pk: <C::AccountCommitment as CommitmentScheme>::Randomness,
}

impl<C: DPCComponents> AccountPrivateKey<C> {
    /// Creates a new account private key. Defaults to a testnet account
    /// if no network indicator is provided.
    // TODO: Add testnet account support.
    pub fn new<R: Rng>(
        parameters: &C::Signature,
        metadata: &[u8; 32],
        _is_testnet: Option<bool>,
        rng: &mut R,
    ) -> Result<Self, AccountError> {
        // Sample SIG key pair.
        let sk_sig = C::Signature::generate_private_key(parameters, rng)?;
        let pk_sig = C::Signature::generate_public_key(parameters, &sk_sig)?;

        // Sample PRF secret key.
        let sk_bytes: [u8; 32] = rng.gen();
        let sk_prf: <C::PRF as PRF>::Seed = FromBytes::read(&sk_bytes[..])?;

        // Sample randomness rpk for the commitment scheme.
        let r_pk = <C::AccountCommitment as CommitmentScheme>::Randomness::rand(rng);

        // Construct the account private key.
        Ok(Self {
            pk_sig,
            sk_sig,
            sk_prf,
            metadata: *metadata,
            r_pk,
        })
    }
}

impl<C: DPCComponents> ToBytes for AccountPrivateKey<C> {
    // TODO: Add testnet account support.
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.pk_sig.write(&mut writer)?;
        self.sk_sig.write(&mut writer)?;
        self.sk_prf.write(&mut writer)?;
        self.metadata.write(&mut writer)?;
        self.r_pk.write(&mut writer)
    }
}

impl<C: DPCComponents> FromBytes for AccountPrivateKey<C> {
    /// Reads in an account private key buffer. Defaults to a testnet account
    /// if no network indicator is provided.
    // TODO: Add testnet account support.
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let pk_sig: <C::Signature as SignatureScheme>::PublicKey = FromBytes::read(&mut reader)?;
        let sk_sig: <C::Signature as SignatureScheme>::PrivateKey = FromBytes::read(&mut reader)?;
        let sk_prf: <C::PRF as PRF>::Seed = FromBytes::read(&mut reader)?;
        let metadata: [u8; 32] = FromBytes::read(&mut reader)?;
        let r_pk: <C::AccountCommitment as CommitmentScheme>::Randomness = FromBytes::read(&mut reader)?;

        Ok(Self {
            pk_sig,
            sk_sig,
            sk_prf,
            metadata,
            r_pk,
        })
    }
}

impl<C: DPCComponents> fmt::Display for AccountPrivateKey<C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut private_key = [0u8; 132];
        let prefix = account_format::PRIVATE_KEY_MAINNET;

        private_key[0..4].copy_from_slice(&prefix);

        self.sk_sig
            .write(&mut private_key[4..36])
            .expect("sk_sig formatting failed");
        self.sk_prf
            .write(&mut private_key[36..68])
            .expect("sk_prf formatting failed");
        self.metadata
            .write(&mut private_key[68..100])
            .expect("metadata formatting failed");
        self.r_pk
            .write(&mut private_key[100..132])
            .expect("r_pk formatting failed");

        write!(f, "{}", private_key.to_base58())
    }
}

impl<C: DPCComponents> fmt::Debug for AccountPrivateKey<C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "AccountPrivateKey {{ sk_sig: {:?}, sk_prf: {:?}, metadata: {:?}, r_pk: {:?} }}",
            self.sk_sig, self.sk_prf, self.metadata, self.r_pk,
        )
    }
}
