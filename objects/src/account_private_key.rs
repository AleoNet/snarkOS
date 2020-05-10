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
#[derivative(Default(bound = "C: DPCComponents"), Clone(bound = "C: DPCComponents"))]
pub struct AccountPrivateKey<C: DPCComponents> {
    pub pk_sig: <C::Signature as SignatureScheme>::PublicKey,
    pub sk_sig: <C::Signature as SignatureScheme>::PrivateKey,
    pub sk_prf: <C::PRF as PRF>::Seed,
    pub metadata: [u8; 32],
    pub r_pk: <C::AddressCommitment as CommitmentScheme>::Randomness,
    pub is_testnet: bool,
}

impl<C: DPCComponents> AccountPrivateKey<C> {
    /// Creates a new account private key. Defaults to a testnet account
    /// if no network indicator is provided.
    pub fn new<R: Rng>(
        parameters: &C::Signature,
        metadata: &[u8; 32],
        is_testnet: Option<bool>,
        rng: &mut R,
    ) -> Result<Self, AccountError> {
        // Sample SIG key pair.
        let sk_sig = C::Signature::generate_private_key(parameters, rng)?;
        let pk_sig = C::Signature::generate_public_key(parameters, &sk_sig)?;

        // Sample PRF secret key.
        let sk_bytes: [u8; 32] = rng.gen();
        let sk_prf: <C::PRF as PRF>::Seed = FromBytes::read(&sk_bytes[..])?;

        // Sample randomness rpk for the commitment scheme.
        let r_pk = <C::AddressCommitment as CommitmentScheme>::Randomness::rand(rng);

        // Determine if this is a testnet account.
        let is_testnet = match is_testnet {
            Some(is_testnet) => is_testnet,
            None => true, // Defaults to testnet
        };

        // Construct the address secret key.
        Ok(Self {
            pk_sig,
            sk_sig,
            sk_prf,
            metadata: *metadata,
            r_pk,
            is_testnet,
        })
    }
}

impl<C: DPCComponents> ToBytes for AccountPrivateKey<C> {
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.pk_sig.write(&mut writer)?;
        self.sk_sig.write(&mut writer)?;
        self.sk_prf.write(&mut writer)?;
        self.metadata.write(&mut writer)?;
        self.r_pk.write(&mut writer)?;
        self.is_testnet.write(&mut writer)
    }
}

impl<C: DPCComponents> FromBytes for AccountPrivateKey<C> {
    /// Reads in an account private key buffer. Defaults to a testnet account
    /// if no network indicator is provided.
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let pk_sig: <C::Signature as SignatureScheme>::PublicKey = FromBytes::read(&mut reader)?;
        let sk_sig: <C::Signature as SignatureScheme>::PrivateKey = FromBytes::read(&mut reader)?;
        let sk_prf: <C::PRF as PRF>::Seed = FromBytes::read(&mut reader)?;
        let metadata: [u8; 32] = FromBytes::read(&mut reader)?;
        let r_pk: <C::AddressCommitment as CommitmentScheme>::Randomness = FromBytes::read(&mut reader)?;
        let is_testnet: bool = match FromBytes::read(&mut reader) {
            Ok(is_testnet) => is_testnet,
            _ => true, // Defaults to testnet
        };

        Ok(Self {
            pk_sig,
            sk_sig,
            sk_prf,
            metadata,
            r_pk,
            is_testnet,
        })
    }
}

impl<C: DPCComponents> fmt::Display for AccountPrivateKey<C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut private_key = [0u8; 131];
        let prefix = match self.is_testnet {
            true => account_format::PRIVATE_KEY_TESTNET,
            false => account_format::PRIVATE_KEY_MAINNET,
        };
        private_key[0..3].copy_from_slice(&prefix);

        self.sk_sig
            .write(&mut private_key[3..35])
            .expect("sk_sig formatting failed");
        self.sk_prf
            .write(&mut private_key[35..67])
            .expect("sk_prf formatting failed");
        self.metadata
            .write(&mut private_key[67..99])
            .expect("metadata formatting failed");
        self.r_pk
            .write(&mut private_key[99..131])
            .expect("r_pk formatting failed");

        write!(f, "{}", private_key.to_base58())
    }
}

impl<C: DPCComponents> fmt::Debug for AccountPrivateKey<C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}
