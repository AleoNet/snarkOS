//TODO (raychu86) The character following `APrivateKey1` has a small range. Need to change
// the last byte to the desired output range.
pub static PRIVATE_KEY_PREFIX: [u8; 10] = [9, 180, 105, 188, 202, 86, 228, 126, 35, 176]; // APrivateKey1
//TODO (raychu86) The character following `AProvingKey1` is always the same. Need to
// further constrain the byte size of the prefix.
pub static _PROVING_KEY_PREFIX: [u8; 10] = [109, 249, 98, 224, 36, 15, 213, 187, 79, 190]; // AProvingKey1(a)
pub static VIEW_KEY_PREFIX: [u8; 7] = [14, 138, 223, 204, 247, 224, 122]; // AViewKey1
pub static ADDRESS_PREFIX: &str = "aleo";
