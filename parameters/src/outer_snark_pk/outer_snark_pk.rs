use snarkos_algorithms::crh::sha256::sha256;
use snarkos_errors::parameters::ParametersError;
use snarkos_models::parameters::Parameters;

use curl::easy::Easy;
use hex;
use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

pub const OUTER_SNARK_PK_REMOTE_URL: &str = "https://snarkos-testnet.s3-us-west-1.amazonaws.com";

pub struct OuterSNARKPKParameters;

impl Parameters for OuterSNARKPKParameters {
    const CHECKSUM: &'static str = include_str!("./outer_snark_pk.checksum");
    const SIZE: u64 = 1806227470;

    /// Loads the outer snark proving key bytes. The method will attempt to locate
    /// the file with a relative path. If it cannot find the path relatively, the
    /// method will attempt to locate the file with an absolute path. Finally,
    /// if it cannot find the path locally, the method will proceed to download
    /// the parameters file remotely and attempt to store it locally.
    fn load_bytes() -> Result<Vec<u8>, ParametersError> {
        // Compose the correct file path for the parameter file.
        let filename = Self::versioned_filename();
        let mut file_path = PathBuf::from(file!());
        file_path.pop();
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

impl OuterSNARKPKParameters {
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

    fn versioned_filename() -> String {
        match Self::CHECKSUM.get(0..7) {
            Some(sum) => format!("outer_snark_pk-{}.params", sum),
            _ => format!("outer_snark_pk.params"),
        }
    }

    fn remote_url() -> String {
        format!("{}/{}", OUTER_SNARK_PK_REMOTE_URL, Self::versioned_filename())
    }

    fn store_bytes(
        buffer: &Vec<u8>,
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
