use once_cell::sync::Lazy;
use rand::SeedableRng;
use rand_xorshift::XorShiftRng;
use snarkos_algorithms::snark::generate_random_parameters;
use snarkos_objects::pedersen_merkle_tree::PARAMS;
use snarkos_utilities::{bytes::ToBytes, to_bytes};
use std::{
    fs::File,
    io::{Result as IoResult, Write},
    marker::PhantomData,
    path::PathBuf,
};

use snarkos_posw::{ProvingKey, VerifyingKey, POSW};
use snarkos_profiler::{end_timer, start_timer};

// Public parameters for the POSW SNARK
pub static POSW_PP: Lazy<(ProvingKey, VerifyingKey)> = Lazy::new(|| {
    let mut path = std::env::current_dir().unwrap();
    path.push("../consensus/src/test_data/");
    let test_pk_path = path.clone().join("test_posw.params");
    let test_vk_path = path.clone().join("test_posw_vk.params");
    let generation_timer = start_timer!(|| "POSW setup");

    let (params, vk) = if test_pk_path.exists() {
        let vk = VerifyingKey::read(&File::open(test_vk_path).unwrap()).unwrap();
        let pk = ProvingKey::read(&File::open(test_pk_path).unwrap(), false).unwrap();

        (pk, vk)
    } else {
        let params = generate_random_parameters(
            POSW {
                leaves: vec![None; 0],
                merkle_parameters: PARAMS.clone(),
                mask: None,
                root: None,
                field_type: PhantomData,
                crh_gadget_type: PhantomData,
                circuit_parameters_type: PhantomData,
            },
            &mut XorShiftRng::seed_from_u64(1234567),
        )
        .unwrap();

        let vk = params.vk.clone();

        store(&params, &test_pk_path).unwrap();
        store(&vk, &test_vk_path).unwrap();

        (params, vk)
    };

    end_timer!(generation_timer);
    (params, vk)
});

fn store<T: ToBytes>(data: T, path: &PathBuf) -> IoResult<()> {
    let mut file = File::create(path)?;
    file.write_all(&to_bytes![data]?)?;
    Ok(())
}
