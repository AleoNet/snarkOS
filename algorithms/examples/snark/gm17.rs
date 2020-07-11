#![deny(unused_import_braces, unused_qualifications, trivial_casts, trivial_numeric_casts)]
#![deny(unused_qualifications, variant_size_differences, stable_features)]
#![deny(
    non_shorthand_field_patterns,
    unused_attributes,
    unused_imports,
    unused_extern_crates
)]
#![deny(renamed_and_removed_lints, stable_features, unused_allocation, unused_comparisons)]
#![deny(unused_must_use, unused_mut, unused_unsafe, private_in_public, unsafe_code)]

use csv;

// For randomness (during paramgen and proof generation)
use rand::thread_rng;

// For benchmarking
use std::{
    error::Error,
    time::{Duration, Instant},
};

// Bring in some tools for using pairing-friendly curves
// We're going to use the BLS12-377 pairing-friendly elliptic curve.
use snarkos_curves::bls12_377::{Bls12_377, Fr};
use snarkos_models::curves::Field;

// We're going to use the Groth-Maller 17 proving system.
use snarkos_algorithms::snark::gm17::{
    create_random_proof,
    generate_random_parameters,
    prepare_verifying_key,
    verify_proof,
};

use std::{env, fs::OpenOptions, path::PathBuf, process};

mod constraints;
use crate::constraints::Benchmark;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 || args[1] == "-h" || args[1] == "--help" {
        println!("\nHelp: Invoke this as <program> <num_inputs> <num_constraints> <output_file_path>\n");
    }
    let num_inputs: usize = args[1].parse().unwrap();
    let num_constraints: usize = args[2].parse().unwrap();
    let output_file_path = PathBuf::from(args[3].clone());
    let mut wtr = if !output_file_path.exists() {
        println!("Creating output file");
        let f = OpenOptions::new().create(true).append(true).open(output_file_path)?;
        let mut wtr = csv::Writer::from_writer(f);
        wtr.write_record(&["num_inputs", "num_constraints", "setup", "prover", "verifier"])?;
        wtr
    } else if output_file_path.is_file() {
        let f = OpenOptions::new().append(true).open(output_file_path)?;
        csv::Writer::from_writer(f)
    } else {
        println!("Path to output file does not point to a file.");
        process::exit(1);
    };
    // This may not be cryptographically safe, use
    // `OsRng` (for example) in production software.
    let rng = &mut thread_rng();

    // Let's benchmark stuff!
    let samples = if num_constraints > 10000 {
        1
    } else if num_constraints > 4096 {
        2
    } else {
        4
    };
    let mut total_setup = Duration::new(0, 0);
    let mut total_proving = Duration::new(0, 0);
    let mut total_verifying = Duration::new(0, 0);

    // Just a place to put the proof data, so we can
    // benchmark deserialization.
    // let mut proof_vec = vec![];

    for _ in 0..samples {
        // Create parameters for our circuit
        let start = Instant::now();
        let params = {
            let c = Benchmark::<Fr>::new(num_constraints);
            generate_random_parameters::<Bls12_377, _, _>(c, rng)?
        };

        // Prepare the verification key (for proof verification)
        let pvk = prepare_verifying_key(&params.vk);
        total_setup += start.elapsed();

        // proof_vec.truncate(0);
        let start = Instant::now();
        let proof = {
            // Create an instance of our circuit (with the witness)
            let c = Benchmark::new(num_constraints);
            // Create a proof with our parameters.
            create_random_proof(c, &params, rng)?
        };

        total_proving += start.elapsed();

        let inputs: Vec<_> = [Fr::one(); 2].to_vec();

        let start = Instant::now();
        // let proof = Proof::read(&proof_vec[..]).unwrap();
        // Check the proof
        let _ = verify_proof(&pvk, &proof, &inputs).unwrap();
        total_verifying += start.elapsed();
    }

    let setup_avg = total_setup / samples;
    let setup_avg = setup_avg.subsec_nanos() as f64 / 1_000_000_000f64 + (setup_avg.as_secs() as f64);

    let proving_avg = total_proving / samples;
    let proving_avg = proving_avg.subsec_nanos() as f64 / 1_000_000_000f64 + (proving_avg.as_secs() as f64);

    let verifying_avg = total_verifying / samples;
    let verifying_avg = verifying_avg.subsec_nanos() as f64 / 1_000_000_000f64 + (verifying_avg.as_secs() as f64);

    println!(
        "=== Benchmarking Groth16 with {} inputs and {} constraints: ====",
        num_inputs, num_constraints
    );
    println!("Average setup time: {:?} seconds", setup_avg);
    println!("Average proving time: {:?} seconds", proving_avg);
    println!("Average verifying time: {:?} seconds", verifying_avg);

    wtr.write_record(&[
        format!("{}", num_inputs),
        format!("{}", num_constraints),
        format!("{}", setup_avg),
        format!("{}", proving_avg),
        format!("{}", verifying_avg),
    ])?;
    wtr.flush()?;
    Ok(())
}
