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

    /// CRH for the encrypted record.
    type EncryptedRecordCRH: CRH;
    type EncryptedRecordCRHGadget: CRHGadget<Self::EncryptedRecordCRH, Self::InnerField>;

    /// CRH for hash of the `Self::InnerSNARK` verification keys.
    /// This is invoked only on the larger curve.
    type InnerSNARKVerificationKeyCRH: CRH;
    type InnerSNARKVerificationKeyCRHGadget: CRHGadget<Self::InnerSNARKVerificationKeyCRH, Self::OuterField>;

    /// CRH and commitment scheme for committing to program input. Invoked inside
    /// `Self::InnerSNARK` and every program SNARK.
    type LocalDataCRH: CRH;
    type LocalDataCRHGadget: CRHGadget<Self::LocalDataCRH, Self::InnerField>;
    type LocalDataCommitment: CommitmentScheme;
    type LocalDataCommitmentGadget: CommitmentGadget<Self::LocalDataCommitment, Self::InnerField>;

    /// CRH for hashes of birth and death verification keys.
    /// This is invoked only on the larger curve.
    type ProgramVerificationKeyCRH: CRH;
    type ProgramVerificationKeyCRHGadget: CRHGadget<Self::ProgramVerificationKeyCRH, Self::OuterField>;

    /// Commitment scheme for committing to hashes of birth and death verification keys
    type ProgramVerificationKeyCommitment: CommitmentScheme;
    /// Used to commit to hashes of verification keys on the smaller curve and to decommit hashes
    /// of verification keys on the larger curve
    type ProgramVerificationKeyCommitmentGadget: CommitmentGadget<Self::ProgramVerificationKeyCommitment, Self::InnerField>
        + CommitmentGadget<Self::ProgramVerificationKeyCommitment, Self::OuterField>;

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
