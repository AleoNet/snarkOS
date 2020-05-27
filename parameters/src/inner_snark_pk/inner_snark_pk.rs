use snarkos_errors::parameters::ParametersError;
use snarkos_models::parameters::Parameters;

use curl::easy::Easy;
use std::{
    fs::{self, File},
    io::Write,
    panic::{catch_unwind, set_hook},
    path::PathBuf,
};

pub const INNER_SNARK_PK_FILENAME: &str = "inner_snark_pk.params";
pub const INNER_SNARK_PK_REMOTE_URL: &str = "https://snarkos-testnet.s3-us-west-1.amazonaws.com/inner_snark_pk.params";

pub struct InnerSNARKPKParameters;

impl Parameters for InnerSNARKPKParameters {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 517337602;

    /// Loads the inner snark proving key bytes by first attempting to lazily load
    /// from `include_bytes`. If the parameters were not found during compilation
    /// (either because the file was missing or the directory path was incorrect),
    /// the method will attempt to locate the file with a relative path. Finally,
    /// if it cannot find the path locally, the method will proceed to download
    /// the parameters file remotely and attempt to store it locally.
    fn load_bytes() -> Result<Vec<u8>, ParametersError> {
        // Attempts to lazily link the parameters at compile time.
        set_hook(Box::new(|_info| ()));
        let output = catch_unwind(|| {
            lazy_static_include_bytes!(INNER_SNARK_PK, "src/inner_snark_pk/inner_snark_pk.params");
            *INNER_SNARK_PK
        });

        match output {
            Ok(buffer) => Ok(buffer.to_vec()),
            _ => {
                // Compose the correct file path for the parameter file.
                let mut file_path = PathBuf::from(file!());
                file_path.pop();
                file_path.push(INNER_SNARK_PK_FILENAME);

                let relative_path = file_path.strip_prefix("parameters")?.to_path_buf();
                if relative_path.exists() {
                    // Attempts one final try to load the parameter file locally.
                    Ok(fs::read(relative_path)?.to_vec())
                } else {
                    // Downloads the missing parameters and stores it in the local directory for use.
                    eprintln!(
                        "\nWARNING - \"{}\" does not exist. snarkOS will download this file remotely and attempt to store it locally. Please ensure \"{}\" is stored in {:?} and recompile snarkOS.\n",
                        INNER_SNARK_PK_FILENAME, INNER_SNARK_PK_FILENAME, file_path
                    );
                    let output = Self::load_remote()?;
                    Self::store_bytes(&output, &relative_path, &file_path)?;
                    Ok(output)
                }
            }
        }
    }
}

impl InnerSNARKPKParameters {
    pub fn load_remote() -> Result<Vec<u8>, ParametersError> {
        println!("{} - downloading parameters...", module_path!());
        let mut buffer = vec![];
        Self::remote_fetch(&mut buffer)?;
        println!("{} - complete", module_path!());
        Ok(buffer)
    }

    fn store_bytes(buffer: &Vec<u8>, relative_path: &PathBuf, file_path: &PathBuf) -> Result<(), ParametersError> {
        println!("{} - attempting to store parameters ({:?})", module_path!(), file_path);
        // Attempt to write the parameter buffer to a file.
        let mut file = File::create(relative_path)?;
        file.write_all(&buffer)?;
        drop(file);
        Ok(())
    }

    fn remote_fetch(buffer: &mut Vec<u8>) -> Result<(), ParametersError> {
        let mut easy = Easy::new();
        easy.url(INNER_SNARK_PK_REMOTE_URL)?;
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
