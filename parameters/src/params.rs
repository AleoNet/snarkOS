// Copyright (C) 2019-2020 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

use snarkos_algorithms::crh::sha256::sha256;
use snarkos_errors::parameters::ParametersError;
use snarkos_models::parameters::Parameters;

use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

#[cfg(any(test, feature = "remote"))]
use curl::easy::Easy;

#[cfg(any(test, feature = "remote"))]
pub const REMOTE_URL: &str = "https://snarkos-testnet.s3-us-west-2.amazonaws.com";

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

                let buffer = if relative_path.exists() {
                    // Attempts to load the parameter file locally with a relative path.
                    fs::read(relative_path)?.to_vec()
                } else if absolute_path.exists() {
                    // Attempts to load the parameter file locally with an absolute path.
                    fs::read(absolute_path)?.to_vec()
                } else {
                    // Downloads the missing parameters and stores it in the local directory for use.
                    eprintln!(
                        "\nWARNING - \"{}\" does not exist. snarkOS will download this file remotely and store it locally. Please ensure \"{}\" is stored in {:?}.\n",
                        filename, filename, file_path
                    );
                    let output = Self::load_remote()?;
                    match Self::store_bytes(&output, &relative_path, &absolute_path, &file_path) {
                        Ok(()) => output,
                        Err(_) => {
                            eprintln!(
                                "\nWARNING - Failed to store \"{}\" locally. Please download this file manually and ensure it is stored in {:?}.\n",
                                filename, file_path
                            );
                            output
                        }
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
                buffer: &[u8],
                relative_path: &PathBuf,
                absolute_path: &PathBuf,
                file_path: &PathBuf,
            ) -> Result<(), ParametersError> {
                println!("{} - Storing parameters ({:?})", module_path!(), file_path);
                // Attempt to write the parameter buffer to a file.
                if let Ok(mut file) = File::create(relative_path) {
                    file.write_all(&buffer)?;
                    drop(file);
                } else if let Ok(mut file) = File::create(absolute_path) {
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
    16420
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

// CRH
impl_params!(
    EncryptedRecordCRHParameters,
    encrypted_record_crh_test,
    "encrypted_record_crh",
    270532
);
impl_params!(
    InnerSNARKVKCRHParameters,
    inner_snark_vk_crh_test,
    "inner_snark_vk_crh",
    3581604
);
impl_params!(LocalDataCRHParameters, local_data_crh_test, "local_data_crh", 65604);
impl_params!(ProgramVKCRHParameters, program_vk_crh_test, "program_vk_crh", 1742404);

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
    32772
);

// POSW SNARK
impl_params_remote!(PoswSNARKPKParameters, "posw_snark_pk", 171163800);
impl_params!(PoswSNARKVKParameters, posw_snark_vk_test, "posw_snark_vk", 40807);

// Program SNARK
impl_params!(
    NoopProgramSNARKPKParameters,
    noop_program_snark_pk_test,
    "noop_program_snark_pk",
    172752
);
impl_params!(
    NoopProgramSNARKVKParameters,
    noop_program_snark_vk_test,
    "noop_program_snark_vk",
    1068
);

// Inner SNARK
impl_params_remote!(InnerSNARKPKParameters, "inner_snark_pk", 123968736);
impl_params!(InnerSNARKVKParameters, inner_snark_vk_test, "inner_snark_vk", 2329);

// Outer SNARK
impl_params_remote!(OuterSNARKPKParameters, "outer_snark_pk", 250168080);
impl_params!(OuterSNARKVKParameters, outer_snark_vk_test, "outer_snark_vk", 4443);
