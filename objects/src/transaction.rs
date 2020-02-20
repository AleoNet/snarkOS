use snarkos_algorithms::crh::double_sha256;
use snarkos_errors::objects::TransactionError;

use base58::FromBase58;
use secp256k1;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fmt, io::Read, str::FromStr};
use wagyu_bitcoin::{
    network::Mainnet, private_key::BitcoinPrivateKey, BitcoinAddress, BitcoinFormat, BitcoinPublicKey,
};
use wagyu_model::{crypto::hash160, PrivateKey};

/// Returns the variable length integer of the given value.
/// https://en.bitcoin.it/wiki/Protocol_documentation#Variable_length_integer
pub fn variable_length_integer(value: u64) -> Vec<u8> {
    match value {
        // bounded by u8::max_value()
        0..=252 => vec![value as u8],
        // bounded by u16::max_value()
        253..=65535 => [vec![0xfd], (value as u16).to_le_bytes().to_vec()].concat(),
        // bounded by u32::max_value()
        65536..=4_294_967_295 => [vec![0xfe], (value as u32).to_le_bytes().to_vec()].concat(),
        // bounded by u64::max_value()
        _ => [vec![0xff], value.to_le_bytes().to_vec()].concat(),
    }
}

/// Decode the value of a variable length integer.
/// https://en.bitcoin.it/wiki/Protocol_documentation#Variable_length_integer
pub fn read_variable_length_integer<R: Read>(mut reader: R) -> Result<usize, TransactionError> {
    let mut flag = [0u8; 1];
    reader.read(&mut flag)?;

    match flag[0] {
        0..=252 => Ok(flag[0] as usize),
        0xfd => {
            let mut size = [0u8; 2];
            reader.read(&mut size)?;
            match u16::from_le_bytes(size) {
                s if s < 253 => Err(TransactionError::InvalidVariableSizeInteger(s as usize)),
                s => Ok(s as usize),
            }
        }
        0xfe => {
            let mut size = [0u8; 4];
            reader.read(&mut size)?;
            match u32::from_le_bytes(size) {
                s if s < 65536 => Err(TransactionError::InvalidVariableSizeInteger(s as usize)),
                s => Ok(s as usize),
            }
        }
        _ => {
            let mut size = [0u8; 8];
            reader.read(&mut size)?;
            match u64::from_le_bytes(size) {
                s if s < 4_294_967_296 => Err(TransactionError::InvalidVariableSizeInteger(s as usize)),
                s => Ok(s as usize),
            }
        }
    }
}

pub struct Vector;

impl Vector {
    /// Read and output a vector with a variable length integer
    pub fn read<R: Read, E, F>(mut reader: R, func: F) -> Result<Vec<E>, TransactionError>
    where
        F: Fn(&mut R) -> Result<E, TransactionError>,
    {
        let count = read_variable_length_integer(&mut reader)?;
        (0..count).map(|_| func(&mut reader)).collect()
    }

    /// Read and output a vector with a variable length integer and the integer itself
    pub fn read_witness<R: Read, E, F>(
        mut reader: R,
        func: F,
    ) -> Result<(usize, Result<Vec<E>, TransactionError>), TransactionError>
    where
        F: Fn(&mut R) -> Result<E, TransactionError>,
    {
        let count = read_variable_length_integer(&mut reader)?;
        Ok((count, (0..count).map(|_| func(&mut reader)).collect()))
    }
}

/// Generate the script_pub_key of a corresponding address
pub fn create_script_pub_key(address: &BitcoinAddress<Mainnet>) -> Result<Vec<u8>, TransactionError> {
    let bytes = &address.to_string().from_base58()?;
    let pub_key_hash = bytes[1..(bytes.len() - 4)].to_vec();

    let mut script = vec![];
    script.push(Opcode::OP_DUP as u8);
    script.push(Opcode::OP_HASH160 as u8);
    script.extend(variable_length_integer(pub_key_hash.len() as u64));
    script.extend(pub_key_hash);
    script.push(Opcode::OP_EQUALVERIFY as u8);
    script.push(Opcode::OP_CHECKSIG as u8);
    Ok(script)
}

/// Represents the commonly used script opcodes
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[allow(non_camel_case_types)]
pub enum Opcode {
    OP_DUP = 0x76,
    OP_HASH160 = 0xa9,
    OP_CHECKSIG = 0xac,
    OP_EQUAL = 0x87,
    OP_EQUALVERIFY = 0x88,
}

impl fmt::Display for Opcode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Opcode::OP_DUP => write!(f, "OP_DUP"),
            Opcode::OP_HASH160 => write!(f, "OP_HASH160"),
            Opcode::OP_CHECKSIG => write!(f, "OP_CHECKSIG"),
            Opcode::OP_EQUAL => write!(f, "OP_EQUAL"),
            Opcode::OP_EQUALVERIFY => write!(f, "OP_EQUALVERIFY"),
        }
    }
}

/// Represents a transaction outpoint
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Outpoint {
    /// The previous transaction hash (32 bytes)
    pub transaction_id: Vec<u8>,
    /// The index of the transaction input (4 bytes)
    pub index: u32,
    /// The script public key associated with spending this input
    pub script_pub_key: Option<Vec<u8>>,
    /// The address of the outpoint
    pub address: Option<BitcoinAddress<Mainnet>>,
}

impl Outpoint {
    /// Returns a new Bitcoin transaction outpoint
    pub fn new(
        transaction_id: Vec<u8>,
        index: u32,
        address: Option<BitcoinAddress<Mainnet>>,
        script_pub_key: Option<Vec<u8>>,
    ) -> Result<Self, TransactionError> {
        let script_pub_key = match address.clone() {
            Some(address) => {
                let script_pub_key = script_pub_key.unwrap_or(create_script_pub_key(&address)?);
                match script_pub_key[0] != Opcode::OP_DUP as u8
                    && script_pub_key[1] != Opcode::OP_HASH160 as u8
                    && script_pub_key[script_pub_key.len() - 1] != Opcode::OP_CHECKSIG as u8
                {
                    true => return Err(TransactionError::InvalidScriptPubKey("P2PKH".into())),
                    false => Some(script_pub_key),
                }
            }
            None => None,
        };

        Ok(Self {
            transaction_id,
            index,
            script_pub_key,
            address,
        })
    }

    pub fn is_coinbase(&self) -> bool {
        [0u8; 32].to_vec() == self.transaction_id && std::u32::MAX == self.index
    }
}

/// Represents a transaction input
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransactionInput {
    /// The outpoint
    pub outpoint: Outpoint,
    /// The transaction input script (variable size)
    pub script_sig: Vec<u8>,
    /// If true, the input has been signed
    pub is_signed: bool,
}

impl TransactionInput {
    /// Returns a new transaction input without the script
    pub fn new(
        transaction_id: Vec<u8>,
        index: u32,
        address: Option<BitcoinAddress<Mainnet>>,
    ) -> Result<Self, TransactionError> {
        if transaction_id.len() != 32 {
            return Err(TransactionError::InvalidTransactionId(transaction_id.len()));
        }

        Ok(Self {
            outpoint: Outpoint::new(transaction_id, index, address, None)?,
            script_sig: vec![],
            is_signed: false,
        })
    }

    /// Read and output a transaction input
    pub fn read<R: Read>(mut reader: &mut R) -> Result<Self, TransactionError> {
        let mut transaction_hash = [0u8; 32];
        let mut vin = [0u8; 4];

        reader.read(&mut transaction_hash)?;
        reader.read(&mut vin)?;

        let outpoint = Outpoint::new(transaction_hash.to_vec(), u32::from_le_bytes(vin), None, None)?;

        let script_sig: Vec<u8> = Vector::read(&mut reader, |s| {
            let mut byte = [0u8; 1];
            s.read(&mut byte)?;
            Ok(byte[0])
        })?;

        Ok(Self {
            outpoint,
            script_sig: script_sig.to_vec(),
            is_signed: !script_sig.is_empty(),
        })
    }

    /// Returns the serialized transaction input.
    pub fn serialize(&self, raw: bool, verify: bool) -> Result<Vec<u8>, TransactionError> {
        let mut input = vec![];
        input.extend(&self.outpoint.transaction_id);
        input.extend(&self.outpoint.index.to_le_bytes());

        match raw {
            true => input.extend(vec![0x00]),
            false => match (self.script_sig.len(), verify) {
                (0, false) | (_, true) => match (&self.outpoint.address, &self.outpoint.script_pub_key) {
                    (Some(_address), None) => return Err(TransactionError::MissingOutpointScriptPublicKey),
                    (_, Some(script_pub_key)) => {
                        input.extend(variable_length_integer(script_pub_key.len() as u64));
                        input.extend(script_pub_key);
                    }
                    (_, _) => input.extend(vec![0x00]),
                },
                (_, _) => {
                    input.extend(variable_length_integer(self.script_sig.len() as u64));
                    input.extend(&self.script_sig);
                }
            },
        };

        Ok(input)
    }
}

/// Represents a transaction output
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TransactionOutput {
    /// The amount (in Satoshi)
    pub amount: u64,
    /// The public key script
    pub script_pub_key: Vec<u8>,
}

impl TransactionOutput {
    /// Returns a Bitcoin transaction output.
    pub fn new(address: &BitcoinAddress<Mainnet>, amount: u64) -> Result<Self, TransactionError> {
        Ok(Self {
            amount,
            script_pub_key: create_script_pub_key(address)?,
        })
    }

    /// Read and output a Bitcoin transaction output
    pub fn read<R: Read>(mut reader: &mut R) -> Result<Self, TransactionError> {
        let mut amount = [0u8; 8];
        reader.read(&mut amount)?;

        let script_pub_key: Vec<u8> = Vector::read(&mut reader, |s| {
            let mut byte = [0u8; 1];
            s.read(&mut byte)?;
            Ok(byte[0])
        })?;

        Ok(Self {
            amount: u64::from_le_bytes(amount),
            script_pub_key,
        })
    }

    /// Returns the serialized transaction output.
    pub fn serialize(&self) -> Result<Vec<u8>, TransactionError> {
        let mut output = vec![];
        output.extend(&self.amount.to_le_bytes());
        output.extend(variable_length_integer(self.script_pub_key.len() as u64));
        output.extend(&self.script_pub_key);
        Ok(output)
    }
}

/// Represents the transaction parameters
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransactionParameters {
    /// The version number (4 bytes)
    pub version: u32,
    /// The transaction inputs
    pub inputs: Vec<TransactionInput>,
    /// The transaction outputs
    pub outputs: Vec<TransactionOutput>,
}

impl TransactionParameters {
    /// Read and output the transaction parameters
    pub fn read<R: Read>(mut reader: &mut R) -> Result<Self, TransactionError> {
        let mut version = [0u8; 4];
        reader.read(&mut version)?;

        let inputs = Vector::read(&mut reader, TransactionInput::read)?;

        let outputs = Vector::read(&mut reader, TransactionOutput::read)?;

        let transaction_parameters = Self {
            version: u32::from_le_bytes(version),
            inputs,
            outputs,
        };

        Ok(transaction_parameters)
    }
}

/// Represents a transaction
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Transaction {
    /// The transaction parameters (version, inputs, outputs)
    pub parameters: TransactionParameters,
}

impl Transaction {
    /// Returns an unsigned transaction given the transaction parameters.
    pub fn new(parameters: &TransactionParameters) -> Result<Self, TransactionError> {
        Ok(Self {
            parameters: parameters.clone(),
        })
    }

    /// Returns a signed transaction given the private key of the sender.
    pub fn sign(&self, private_key: &BitcoinPrivateKey<Mainnet>) -> Result<Self, TransactionError> {
        let mut transaction = self.clone();
        for (vin, input) in self.parameters.inputs.iter().enumerate() {
            let private_key_to_address = private_key.to_address(&BitcoinFormat::P2PKH)?;

            let to_sign = match (&input.outpoint.address, &input.outpoint.script_pub_key) {
                (Some(address), _) => address == &private_key_to_address,
                (_, Some(script_pub_key)) => {
                    hex::encode(create_script_pub_key(&private_key_to_address)?) == hex::encode(script_pub_key)
                }
                (_, _) => false,
            };

            if to_sign && !transaction.parameters.inputs[vin].is_signed {
                // Transaction hash
                let preimage = transaction.p2pkh_hash_preimage(vin, false)?;
                let transaction_hash = Sha256::digest(&Sha256::digest(&preimage)).to_vec();

                // Signature
                let signature = secp256k1::Secp256k1::signing_only()
                    .sign(
                        &secp256k1::Message::from_slice(&transaction_hash)?,
                        &private_key.to_secp256k1_secret_key(),
                    )
                    .serialize_der()
                    .to_vec();

                let signature = [variable_length_integer(signature.len() as u64), signature].concat();

                // Public key
                let public_key = private_key.to_public_key();
                let public_key_bytes = match public_key.is_compressed() {
                    false => public_key.to_secp256k1_public_key().serialize_uncompressed().to_vec(),
                    _ => public_key.to_secp256k1_public_key().serialize().to_vec(),
                };

                let public_key = [vec![public_key_bytes.len() as u8], public_key_bytes].concat();

                transaction.parameters.inputs[vin].script_sig = [signature, public_key].concat();
                transaction.parameters.inputs[vin].is_signed = true;
            }
        }

        Ok(transaction)
    }

    /// Returns a transaction given the transaction bytes.
    /// Note:: Raw transaction hex does not include enough
    pub fn deserialize(transaction: &Vec<u8>) -> Result<Self, TransactionError> {
        Ok(Self {
            parameters: TransactionParameters::read(&mut &transaction[..])?,
        })
    }

    /// Returns the transaction in bytes.
    pub fn serialize(&self) -> Result<Vec<u8>, TransactionError> {
        let mut transaction = self.parameters.version.to_le_bytes().to_vec();

        transaction.extend(variable_length_integer(self.parameters.inputs.len() as u64));
        for input in &self.parameters.inputs {
            transaction.extend(input.serialize(false, false)?);
        }

        transaction.extend(variable_length_integer(self.parameters.outputs.len() as u64));
        for output in &self.parameters.outputs {
            transaction.extend(output.serialize()?);
        }

        Ok(transaction)
    }

    /// Returns the transaction id.
    pub fn to_transaction_id(&self) -> Result<Vec<u8>, TransactionError> {
        Ok(double_sha256(&self.serialize()?))
    }

    /// Return the P2PKH hash preimage of the raw transaction.
    pub fn p2pkh_hash_preimage(&self, vin: usize, verify: bool) -> Result<Vec<u8>, TransactionError> {
        let mut preimage = self.parameters.version.to_le_bytes().to_vec();
        preimage.extend(variable_length_integer(self.parameters.inputs.len() as u64));
        for (index, input) in self.parameters.inputs.iter().enumerate() {
            preimage.extend(input.serialize(index != vin, verify)?);
        }
        preimage.extend(variable_length_integer(self.parameters.outputs.len() as u64));
        for output in &self.parameters.outputs {
            preimage.extend(output.serialize()?);
        }

        Ok(preimage)
    }

    /// Update a transaction's input outpoint
    #[allow(dead_code)]
    pub fn update_outpoint(&self, outpoint: Outpoint) -> Self {
        let mut new_transaction = self.clone();
        for (vin, input) in self.parameters.inputs.iter().enumerate() {
            if &outpoint.transaction_id == &input.outpoint.transaction_id && &outpoint.index == &input.outpoint.index {
                new_transaction.parameters.inputs[vin].outpoint = outpoint.clone();
            }
        }
        new_transaction
    }

    pub fn create_coinbase_transaction(
        block_number: u32,
        block_reward: u64,
        transaction_fees: u64,
        miner_address: &BitcoinAddress<Mainnet>,
    ) -> Result<Self, TransactionError> {
        let coinbase_outpoint = Outpoint {
            transaction_id: [0u8; 32].to_vec(),
            index: std::u32::MAX,
            script_pub_key: None,
            address: None,
        };

        // Any arbitrary data: currently block number
        let coinbase_script_sig = block_number.to_le_bytes().to_vec();
        //        let coinbase_script_sig = [0u8; 16].to_vec();

        let coinbase_input = TransactionInput {
            outpoint: coinbase_outpoint,
            script_sig: coinbase_script_sig,
            is_signed: true,
        };

        let output = TransactionOutput::new(miner_address, block_reward + transaction_fees)?;

        let parameters = TransactionParameters {
            version: 1,
            inputs: vec![coinbase_input],
            outputs: vec![output],
        };

        let transaction = Self { parameters };

        Ok(transaction)
    }

    /// Verify that the inputs can spend the total output amount
    pub fn verify_amounts(&self, valid_input_amounts: u64) -> Result<(), TransactionError> {
        let output_sum: u64 = self.parameters.outputs.iter().map(|output| output.amount).sum();

        match valid_input_amounts > output_sum {
            true => Ok(()),
            false => Err(TransactionError::InsufficientFunds(valid_input_amounts, output_sum)),
        }
    }

    /// Calculate the transaction fee for the transaction
    pub fn calculate_transaction_fee(&self, valid_input_amounts: u64) -> Result<u64, TransactionError> {
        let output_sum: u64 = self.parameters.outputs.iter().map(|output| output.amount).sum();

        self.verify_amounts(valid_input_amounts)?;

        Ok(valid_input_amounts - output_sum)
    }

    /// Verify the p2pkh input signature (use in conjunction with update_outpoint when deserializing transactions)
    pub fn verify_signatures(&self) -> Result<(), TransactionError> {
        for (vin, input) in self.parameters.inputs.iter().enumerate() {
            let script_pub_key = match &input.outpoint.script_pub_key {
                Some(script) => script,
                None => {
                    return if input.outpoint.is_coinbase() {
                        Ok(())
                    } else {
                        Err(TransactionError::MissingOutpointScriptPublicKey)
                    };
                }
            };

            // Handle the p2pkh scripsig

            let mut expected_pub_key_hash = match script_pub_key[0] != Opcode::OP_DUP as u8
                && script_pub_key[1] != Opcode::OP_HASH160 as u8
                && script_pub_key[script_pub_key.len() - 1] != Opcode::OP_CHECKSIG as u8
            {
                true => return Err(TransactionError::InvalidScriptPubKey("P2PKH".into())),
                false => &script_pub_key[2..script_pub_key.len() - 2],
            };

            let expected_pub_key_hash: Vec<u8> = Vector::read(&mut expected_pub_key_hash, |s| {
                let mut byte = [0u8; 1];
                s.read(&mut byte)?;
                Ok(byte[0])
            })?;

            let mut script_sig = &input.script_sig.clone()[..];

            let signature: Vec<u8> = Vector::read(&mut script_sig, |s| {
                let mut byte = [0u8; 1];
                s.read(&mut byte)?;
                Ok(byte[0])
            })?;

            let pub_key: Vec<u8> = Vector::read(&mut script_sig, |s| {
                let mut byte = [0u8; 1];
                s.read(&mut byte)?;
                Ok(byte[0])
            })?;

            let pub_key_hash = hash160(&pub_key);

            let expected_pub_key_hash_string = hex::encode(expected_pub_key_hash);
            let pub_key_hash_string = hex::encode(pub_key_hash);

            match expected_pub_key_hash_string == pub_key_hash_string {
                false => {
                    return Err(TransactionError::InvalidPubKeyHash(
                        expected_pub_key_hash_string,
                        pub_key_hash_string,
                    ));
                }
                _ => {}
            };

            // Verify the signature

            let preimage = self.p2pkh_hash_preimage(vin, true)?;
            let transaction_hash = Sha256::digest(&Sha256::digest(&preimage)).to_vec();
            let message = &secp256k1::Message::from_slice(&transaction_hash)?;

            let expected_signature = secp256k1::Signature::from_der(&signature)?;
            let public_key = BitcoinPublicKey::<Mainnet>::from_str(&hex::encode(&pub_key))?;

            let secp = secp256k1::Secp256k1::new();
            secp.verify(message, &expected_signature, &public_key.to_secp256k1_public_key())?;
        }

        Ok(())
    }

    pub fn is_coinbase(&self) -> bool {
        self.parameters.inputs.len() == 1 && self.parameters.inputs[0].outpoint.is_coinbase()
    }
}

impl FromStr for Transaction {
    type Err = TransactionError;

    fn from_str(transaction: &str) -> Result<Self, Self::Err> {
        Transaction::deserialize(&hex::decode(transaction)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use wagyu_bitcoin::BitcoinFormat;

    pub struct TransactionTestCase<'a> {
        pub version: u32,
        pub inputs: &'a [Input],
        pub outputs: &'a [Output],
        pub expected_signed_transaction: &'a str,
        pub expected_transaction_id: &'a str,
    }

    #[derive(Debug, Clone)]
    pub struct Input {
        pub private_key: &'static str,
        pub transaction_id: &'static str,
        pub index: u32,
    }

    #[derive(Clone)]
    pub struct Output {
        pub address: &'static str,
        pub amount: u64,
    }

    fn test_transaction(
        version: u32,
        inputs: Vec<Input>,
        outputs: Vec<Output>,
        expected_signed_transaction: &str,
        expected_transaction_id: &str,
    ) {
        let mut input_vec = vec![];
        for input in &inputs {
            let private_key = BitcoinPrivateKey::<Mainnet>::from_str(input.private_key).unwrap();
            let address = private_key.to_address(&BitcoinFormat::P2PKH).unwrap();
            let transaction_id = hex::decode(input.transaction_id).unwrap();

            let transaction_input = TransactionInput::new(transaction_id, input.index, Some(address)).unwrap();

            input_vec.push(transaction_input);
        }

        let mut output_vec = vec![];
        for output in outputs {
            let address = BitcoinAddress::<Mainnet>::from_str(output.address).unwrap();
            output_vec.push(TransactionOutput::new(&address, output.amount).unwrap());
        }

        let transaction_parameters = TransactionParameters {
            version,
            inputs: input_vec,
            outputs: output_vec,
        };

        let mut transaction = Transaction::new(&transaction_parameters).unwrap();

        // Sign transaction
        for input in inputs {
            transaction = transaction
                .sign(&BitcoinPrivateKey::from_str(input.private_key).unwrap())
                .unwrap();
        }

        let signed_transaction = hex::encode(&transaction.serialize().unwrap());
        let transaction_id = hex::encode(&transaction.to_transaction_id().unwrap());

        // Verify Transaction
        transaction.verify_signatures().unwrap();

        assert_eq!(expected_signed_transaction, &signed_transaction);
        assert_eq!(expected_transaction_id, &transaction_id);
    }

    mod test_real_mainnet_transactions {
        use super::*;

        const TEST_TRANSACTION: TransactionTestCase = TransactionTestCase {
            // p2pkh to p2pkh - based on https://github.com/bitcoinjs/bitcoinjs-lib/blob/master/test/integration/transactions.js
            version: 1,
            inputs: &[Input {
                private_key: "L1uyy5qTuGrVXrmrsvHWHgVzW9kKdrp27wBC7Vs6nZDTF2BRUVwy",
                transaction_id: "61d520ccb74288c96bc1a2b20ea1c0d5a704776dd0164a396efec3ea7040349d",
                index: 0,
            }],
            outputs: &[Output {
                address: "1cMh228HTCiwS8ZsaakH8A8wze1JR5ZsP",
                amount: 12000,
            }],
            expected_signed_transaction: "010000000161d520ccb74288c96bc1a2b20ea1c0d5a704776dd0164a396efec3ea7040349d000000006a473045022100e503974f108b03da744c8cafb86d820e7df3359a6226630a1f3ec156eac375f40220411cd07f46fba8a6e9fd214ab38a54d8850d7c34e6841b58b3460257c92a123421029f50f51d63b345039a290c94bffd3180c99ed659ff6ea6b1242bca47eb93b59f01e02e0000000000001976a91406afd46bcdfd22ef94ac122aa11f241244a37ecc88ac",
            expected_transaction_id: "ea1fde192f16f79199707d8978b7e24a8c4b4e1e0c50ebdab7db11e9d76bbc95",
        };

        #[test]
        fn basic_test() {
            test_transaction(
                TEST_TRANSACTION.version,
                TEST_TRANSACTION.inputs.to_vec(),
                TEST_TRANSACTION.outputs.to_vec(),
                TEST_TRANSACTION.expected_signed_transaction,
                TEST_TRANSACTION.expected_transaction_id,
            );
        }
    }
}
