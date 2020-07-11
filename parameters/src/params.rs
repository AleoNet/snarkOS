use snarkos_algorithms::crh::sha256::sha256;
use snarkos_errors::parameters::ParametersError;
use snarkos_models::parameters::Parameters;

use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
    path::PathBuf,
};

#[cfg(any(test, feature = "remote"))]
use curl::easy::Easy;

#[cfg(any(test, feature = "remote"))]
pub const REMOTE_URL: &str = "https://snarkos-testnet.s3-us-west-1.amazonaws.com";

macro_rules! impl_params {
    ($name: ident, $test_name: ident, $fname: tt, $size: tt) => {
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct $name;

        impl Parameters for $name {
            const CHECKSUM: &'static str = include_str!(concat!("params/", $fname, ".checksum"));
            const SIZE: u64 = $size;

            fn load_bytes() -> Result<Vec<u8>, ParametersError> {
                let buffer = include_bytes!(concat!("params/", $fname, ".params"));
                let checksum = hex::encode(sha256(buffer));
                match Self::CHECKSUM == checksum {
                    true => Ok(buffer.to_vec()),
                    false => Err(ParametersError::ChecksumMismatch(Self::CHECKSUM.into(), checksum)),
                }
            }
        }

        #[cfg(test)]
        #[test]
        fn $test_name() {
            let parameters = $name::load_bytes().expect("failed to load parameters");
            assert_eq!($name::SIZE, parameters.len() as u64);
        }
    };
}

macro_rules! impl_params_remote {
    ($name: ident, $fname: tt, $size: tt) => {

        pub struct $name;

        impl Parameters for $name {
            const CHECKSUM: &'static str = include_str!(concat!("params/", $fname, ".checksum"));
            const SIZE: u64 = $size;

            fn load_bytes() -> Result<Vec<u8>, ParametersError> {
                // Compose the correct file path for the parameter file.
                let filename = Self::versioned_filename();
                let mut file_path = PathBuf::from(file!());
                file_path.pop();
                file_path.push("params/");
                file_path.push(&filename);

                // Compute the relative path.
                let relative_path = file_path.strip_prefix("parameters")?.to_path_buf();

                // Compute the absolute path.
                let mut absolute_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
                absolute_path.push(&relative_path);

                let mut buffer = Vec::with_capacity(Self::SIZE as usize);
                if relative_path.exists() {
                    // Attempts to load the parameter file locally with a relative path.
                    let file = File::open(relative_path)?;
                    let mut reader = BufReader::new(file);
                    reader.read_to_end(&mut buffer)?;
                } else if absolute_path.exists() {
                    let file = File::open(absolute_path)?;
                    let mut reader = BufReader::new(file);
                    // Attempts to load the parameter file locally with an absolute path.
                    reader.read_to_end(&mut buffer)?;
                } else {
                    // Downloads the missing parameters and stores it in the local directory for use.
                    eprintln!(
                        "\nWARNING - \"{}\" does not exist. snarkOS will download this file remotely and store it locally. Please ensure \"{}\" is stored in {:?}.\n",
                        filename, filename, file_path
                    );
                    buffer = Self::load_remote()?;
                    if let Err(err) = Self::store_bytes(&buffer, &relative_path, &absolute_path, &file_path) {
                        eprintln!(
                            "\nWARNING - Failed to store \"{}\" locally. Please download this file manually and ensure it is stored in {:?}.\nError: {:?}",
                            filename, file_path, err
                        );
                    }
                };

                let checksum = hex::encode(sha256(&buffer));
                match Self::CHECKSUM == checksum {
                    true => Ok(buffer),
                    false => Err(ParametersError::ChecksumMismatch(Self::CHECKSUM.into(), checksum)),
                }
            }
        }

        impl $name {
            #[cfg(any(test, feature = "remote"))]
            pub fn load_remote() -> Result<Vec<u8>, ParametersError> {
                println!("{} - Downloading parameters...", module_path!());
                let mut buffer = vec![];
                let url = Self::remote_url();
                Self::remote_fetch(&mut buffer, &url)?;
                println!("\n{} - Download complete", module_path!());

                // Verify the checksum of the remote data before returning
                let checksum = hex::encode(sha256(&buffer));
                match Self::CHECKSUM == checksum {
                    true => Ok(buffer),
                    false => Err(ParametersError::ChecksumMismatch(Self::CHECKSUM.into(), checksum)),
                }
            }

            #[cfg(not(any(test, feature = "remote")))]
            pub fn load_remote() -> Result<Vec<u8>, ParametersError> {
                Err(ParametersError::RemoteFetchDisabled)
            }

            fn versioned_filename() -> String {
                match Self::CHECKSUM.get(0..7) {
                    Some(sum) => format!("{}-{}.params", $fname, sum),
                    _ => concat!($fname, ".params",).to_string()
                }
            }

            #[cfg(any(test, feature = "remote"))]
            fn remote_url() -> String {
                format!("{}/{}", REMOTE_URL, Self::versioned_filename())
            }

            fn store_bytes(
                buffer: &Vec<u8>,
                relative_path: &PathBuf,
                absolute_path: &PathBuf,
                file_path: &PathBuf,
            ) -> Result<(), ParametersError> {
                println!("{} - Storing parameters ({:?})", module_path!(), file_path);
                // Attempt to write the parameter buffer to a file.
                if let Ok(file) = File::create(relative_path) {
                    let mut file = BufWriter::new(file);
                    file.write_all(&buffer)?;
                    drop(file);
                } else if let Ok(file) = File::create(absolute_path) {
                    let mut file = BufWriter::new(file);
                    file.write_all(&buffer)?;
                    drop(file);
                }
                Ok(())
            }

            #[cfg(any(test, feature = "remote"))]
            fn remote_fetch(buffer: &mut Vec<u8>, url: &str) -> Result<(), ParametersError> {
                let mut easy = Easy::new();
                easy.url(url)?;
                easy.progress(true)?;
                easy.progress_function(|total_download, current_download, _, _| {
                    let percent = (current_download / total_download) * 100.0;
                    let size_in_megabytes = total_download as u64 / 1_048_576;
                    print!(
                        "\r{} - {:.2}% complete ({:#} MB total)",
                        module_path!(),
                        percent,
                        size_in_megabytes
                    );
                    true
                })?;

                let mut transfer = easy.transfer();
                transfer.write_function(|data| {
                    buffer.extend_from_slice(data);
                    Ok(data.len())
                })?;
                Ok(transfer.perform()?)
            }
        }
    }
}

// Commitments
impl_params!(
    AccountCommitmentParameters,
    account_commitment_test,
    "account_commitment",
    417868
);
impl_params!(
    AccountSignatureParameters,
    account_signature_test,
    "account_signature",
    96
);
impl_params!(
    LedgerMerkleTreeParameters,
    ledger_merkle_tree_test,
    "ledger_merkle_tree",
    32804
);
impl_params!(
    LocalDataCommitmentParameters,
    local_data_commitment_test,
    "local_data_commitment",
    280780
);
impl_params!(
    RecordCommitmentParameters,
    record_commitment_test,
    "record_commitment",
    507084
);
impl_params!(
    ValueCommitmentParameters,
    value_commitment_test,
    "value_commitment",
    403244
);

// CRH
impl_params!(LocalDataCRHParameters, local_data_crh_test, "local_data_crh", 65604);
impl_params!(
    PredicateVKCRHParameters,
    predicate_vk_crh_test,
    "predicate_vk_crh",
    1742404
);
impl_params!(
    SerialNumberNonceCRHParameters,
    serial_number_nonce_crh_test,
    "serial_number_nonce_crh",
    258180
);

// Encryption
impl_params!(
    AccountEncryptionParameters,
    account_encryption_test,
    "account_encryption",
    128
);

// POSW SNARK
impl_params!(PoswSNARKPKParameters, posw_snark_pk_test, "posw_snark_pk", 32169122);
impl_params!(PoswSNARKVKParameters, posw_snark_vk_test, "posw_snark_vk", 1165);

// Predicate SNARK
impl_params!(
    PredicateSNARKPKParameters,
    predicate_snark_pk_test,
    "predicate_snark_pk",
    348514
);
impl_params!(
    PredicateSNARKVKParameters,
    predicate_snark_vk_test,
    "predicate_snark_vk",
    1068
);

// Inner SNARK
impl_params_remote!(InnerSNARKPKParameters, "inner_snark_pk", 421629022);
impl_params!(InnerSNARKVKParameters, inner_snark_vk_test, "inner_snark_vk", 2717);

// Outer SNARK
impl_params_remote!(OuterSNARKPKParameters, "outer_snark_pk", 944119354);
impl_params!(OuterSNARKVKParameters, outer_snark_vk_test, "outer_snark_vk", 5022);
