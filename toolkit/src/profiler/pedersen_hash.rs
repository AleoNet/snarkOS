// fn pedersen() {
//     let rng = &mut thread_rng();
//     let mut cs = TestConstraintSystem::<Fr>::new();
//
//     let (input, input_bytes, mask_bytes) = generate_input(&mut cs, rng);
//     println!("number of constraints for input: {}", cs.num_constraints());
//
//     let crh = TestCRH::setup(rng);
//     let native_result = crh.hash(&input).unwrap();
//
//     let parameters_gadget: PedersenCRHParametersGadget<EdwardsProjective, Size, Fr, EdwardsBlsGadget> =
//         <TestCRHGadget as CRHGadget<TestCRH, Fr>>::ParametersGadget::alloc(&mut cs.ns(|| "gadget_parameters"), || {
//             Ok(&crh.parameters)
//         })
//             .unwrap();
//     println!("number of constraints for input + params: {}", cs.num_constraints());
//
//     let output_gadget = <TestCRHGadget as CRHGadget<TestCRH, Fr>>::check_evaluation_gadget(
//         &mut cs.ns(|| "gadget_evaluation"),
//         &parameters_gadget,
//         &input_bytes,
//     )
//         .unwrap();
//
//     let masked_output_gadget = <TestCRHGadget as MaskedCRHGadget<TestCRH, Fr>>::check_evaluation_gadget_masked(
//         &mut cs.ns(|| "masked_gadget_evaluation"),
//         &parameters_gadget,
//         &input_bytes,
//         &mask_bytes,
//     )
//         .unwrap();
//
//     println!("number of constraints total: {}", cs.num_constraints());
//
// }
