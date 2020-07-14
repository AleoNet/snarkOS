use crate::{
    algorithms::{CommitmentScheme, EncryptionScheme, SignatureScheme, CRH, PRF},
    curves::PrimeField,
    gadgets::algorithms::{
        CRHGadget,
        CommitmentGadget,
        EncryptionGadget,
        PRFGadget,
        SignaturePublicKeyRandomizationGadget,
    },
};

pub trait DPCComponents: 'static + Sized {
    const NUM_INPUT_RECORDS: usize;
    const NUM_OUTPUT_RECORDS: usize;

    type InnerField: PrimeField;
    type OuterField: PrimeField;

    /// Encryption scheme for account records.
    type AccountEncryption: EncryptionScheme;
    type AccountEncryptionGadget: EncryptionGadget<Self::AccountEncryption, Self::InnerField>;

    /// Commitment scheme for account contents. Invoked only over `Self::InnerField`.
    type AccountCommitment: CommitmentScheme;
    type AccountCommitmentGadget: CommitmentGadget<Self::AccountCommitment, Self::InnerField>;

    /// Signature scheme for delegated compute.
    type AccountSignature: SignatureScheme;
    type AccountSignatureGadget: SignaturePublicKeyRandomizationGadget<Self::AccountSignature, Self::InnerField>;

    /// CRH and commitment scheme for committing to predicate input. Invoked inside
    /// `Self::MainN` and every predicate SNARK.
    type LocalDataCRH: CRH;
    type LocalDataCRHGadget: CRHGadget<Self::LocalDataCRH, Self::InnerField>;
    type LocalDataCommitment: CommitmentScheme;
    type LocalDataCommitmentGadget: CommitmentGadget<Self::LocalDataCommitment, Self::InnerField>;

    /// CRH for hashes of birth and death verification keys.
    /// This is invoked only on the larger curve.
    type PredicateVerificationKeyHash: CRH;
    type PredicateVerificationKeyHashGadget: CRHGadget<Self::PredicateVerificationKeyHash, Self::OuterField>;

    /// Commitment scheme for committing to hashes of birth and death verification keys
    type PredicateVerificationKeyCommitment: CommitmentScheme;
    /// Used to commit to hashes of verification keys on the smaller curve and to decommit hashes
    /// of verification keys on the larger curve
    type PredicateVerificationKeyCommitmentGadget: CommitmentGadget<Self::PredicateVerificationKeyCommitment, Self::InnerField>
        + CommitmentGadget<Self::PredicateVerificationKeyCommitment, Self::OuterField>;

    /// PRF for computing serial numbers. Invoked only over `Self::InnerField`.
    type PRF: PRF;
    type PRFGadget: PRFGadget<Self::PRF, Self::InnerField>;

    /// Commitment scheme for record contents. Invoked only over `Self::InnerField`.
    type RecordCommitment: CommitmentScheme;
    type RecordCommitmentGadget: CommitmentGadget<Self::RecordCommitment, Self::InnerField>;

    /// CRH for computing the serial number nonce. Invoked only over `Self::InnerField`.
    type SerialNumberNonceCRH: CRH;
    type SerialNumberNonceCRHGadget: CRHGadget<Self::SerialNumberNonceCRH, Self::InnerField>;
}
