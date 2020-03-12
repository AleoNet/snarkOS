use snarkos_errors::objects::AmountError;

use serde::Serialize;
use std::fmt;

// Number of UNITS (base unit) per ALEO
const COIN: i128 = 1_000_000_000_000_000_000;

// Maximum number of ALEO tokens
const MAX_TOKENS: i128 = 1_000_000_000_000;

// Maximum number of UNITS
const MAX_COINS: i128 = MAX_TOKENS * COIN;

/// Represents the amount of ALEOs in UNITS
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct AleoAmount(pub i128);

pub enum Denomination {
    // AU
    UNIT,
    // AB
    BYTE,
    // AC
    CYCLE,
    // AG
    GATE,
    // AN
    NAME,
    // ALEO
    ALEO,
}

impl Denomination {
    /// The number of decimal places more than a Unit.
    fn precision(self) -> u32 {
        match self {
            Denomination::UNIT => 0,
            Denomination::BYTE => 3,
            Denomination::CYCLE => 6,
            Denomination::GATE => 9,
            Denomination::NAME => 12,
            Denomination::ALEO => 18,
        }
    }
}

impl fmt::Display for Denomination {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            Denomination::UNIT => "AU",
            Denomination::BYTE => "AB",
            Denomination::CYCLE => "AC",
            Denomination::GATE => "AG",
            Denomination::NAME => "AN",
            Denomination::ALEO => "ALEO",
        })
    }
}

impl AleoAmount {
    /// Exactly one ALEO.
    pub const ONE_ALEO: AleoAmount = AleoAmount(COIN);
    /// Exactly one unit.
    pub const ONE_UNIT: AleoAmount = AleoAmount(1);
    /// The zero amount.
    pub const ZERO: AleoAmount = AleoAmount(0);

    pub fn from_units(units: i128) -> Result<Self, AmountError> {
        if -MAX_COINS <= units && units <= MAX_COINS {
            Ok(Self(units))
        } else {
            return Err(AmountError::AmountOutOfBounds(units.to_string(), MAX_COINS.to_string()));
        }
    }

    pub fn from_byte(byte_value: i128) -> Result<Self, AmountError> {
        let units = byte_value * 10_i128.pow(Denomination::BYTE.precision());

        Self::from_units(units)
    }

    pub fn from_cycle(cycle_value: i128) -> Result<Self, AmountError> {
        let units = cycle_value * 10_i128.pow(Denomination::CYCLE.precision());

        Self::from_units(units)
    }

    pub fn from_gate(gate_value: i128) -> Result<Self, AmountError> {
        let units = gate_value * 10_i128.pow(Denomination::GATE.precision());

        Self::from_units(units)
    }

    pub fn from_name(name_value: i128) -> Result<Self, AmountError> {
        let units = name_value * 10_i128.pow(Denomination::NAME.precision());

        Self::from_units(units)
    }

    pub fn from_aleo(aleo_value: i128) -> Result<Self, AmountError> {
        let units = aleo_value * 10_i128.pow(Denomination::ALEO.precision());

        Self::from_units(units)
    }

    pub fn add(self, b: Self) -> Result<Self, AmountError> {
        Self::from_units(self.0 + b.0)
    }

    pub fn sub(self, b: AleoAmount) -> Result<Self, AmountError> {
        Self::from_units(self.0 - b.0)
    }
}

impl fmt::Display for AleoAmount {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_from_units(units: i128, expected_amount: AleoAmount) {
        let amount = AleoAmount::from_units(units).unwrap();
        assert_eq!(expected_amount, amount)
    }

    fn test_from_byte(byte_value: i128, expected_amount: AleoAmount) {
        let amount = AleoAmount::from_byte(byte_value).unwrap();
        assert_eq!(expected_amount, amount)
    }

    fn test_from_cycle(cycle_value: i128, expected_amount: AleoAmount) {
        let amount = AleoAmount::from_cycle(cycle_value).unwrap();
        assert_eq!(expected_amount, amount)
    }

    fn test_from_gate(gate_value: i128, expected_amount: AleoAmount) {
        let amount = AleoAmount::from_gate(gate_value).unwrap();
        assert_eq!(expected_amount, amount)
    }

    fn test_from_name(name_value: i128, expected_amount: AleoAmount) {
        let amount = AleoAmount::from_name(name_value).unwrap();
        assert_eq!(expected_amount, amount)
    }

    fn test_from_aleo(aleo_value: i128, expected_amount: AleoAmount) {
        let amount = AleoAmount::from_aleo(aleo_value).unwrap();
        assert_eq!(expected_amount, amount)
    }

    fn test_addition(a: &i128, b: &i128, result: &i128) {
        let a = AleoAmount::from_units(*a).unwrap();
        let b = AleoAmount::from_units(*b).unwrap();
        let result = AleoAmount::from_units(*result).unwrap();

        assert_eq!(result, a.add(b).unwrap());
    }

    fn test_subtraction(a: &i128, b: &i128, result: &i128) {
        let a = AleoAmount::from_units(*a).unwrap();
        let b = AleoAmount::from_units(*b).unwrap();
        let result = AleoAmount::from_units(*result).unwrap();

        assert_eq!(result, a.sub(b).unwrap());
    }

    pub struct AmountDenominationTestCase {
        unit: i128,
        byte: i128,
        cycle: i128,
        gate: i128,
        name: i128,
        aleo: i128,
    }

    mod valid_conversions {
        use super::*;

        const TEST_AMOUNTS: [AmountDenominationTestCase; 5] = [
            AmountDenominationTestCase {
                unit: 0,
                byte: 0,
                cycle: 0,
                gate: 0,
                name: 0,
                aleo: 0,
            },
            AmountDenominationTestCase {
                unit: 1_000_000_000_000_000_000,
                byte: 1_000_000_000_000_000,
                cycle: 1_000_000_000_000,
                gate: 1_000_000_000,
                name: 1_000_000,
                aleo: 1,
            },
            AmountDenominationTestCase {
                unit: 1_000_000_000_000_000_000_000,
                byte: 1_000_000_000_000_000_000,
                cycle: 1_000_000_000_000_000,
                gate: 1_000_000_000_000,
                name: 1_000_000_000,
                aleo: 1_000,
            },
            AmountDenominationTestCase {
                unit: 1_234_567_000_000_000_000_000_000_000,
                byte: 1_234_567_000_000_000_000_000_000,
                cycle: 1_234_567_000_000_000_000_000,
                gate: 1_234_567_000_000_000_000,
                name: 1_234_567_000_000_000,
                aleo: 1_234_567_000,
            },
            AmountDenominationTestCase {
                unit: MAX_COINS,
                byte: 1_000_000_000_000_000_000_000_000_000,
                cycle: 1_000_000_000_000_000_000_000_000,
                gate: 1_000_000_000_000_000_000_000,
                name: 1_000_000_000_000_000_000,
                aleo: 1_000_000_000_000,
            },
        ];

        #[test]
        fn test_unit_conversion() {
            TEST_AMOUNTS
                .iter()
                .for_each(|amounts| test_from_units(amounts.unit, AleoAmount(amounts.unit)));
        }

        #[test]
        fn test_byte_conversion() {
            TEST_AMOUNTS
                .iter()
                .for_each(|amounts| test_from_byte(amounts.byte, AleoAmount(amounts.unit)));
        }

        #[test]
        fn test_cycle_conversion() {
            TEST_AMOUNTS
                .iter()
                .for_each(|amounts| test_from_cycle(amounts.cycle, AleoAmount(amounts.unit)));
        }

        #[test]
        fn test_gate_conversion() {
            TEST_AMOUNTS
                .iter()
                .for_each(|amounts| test_from_gate(amounts.gate, AleoAmount(amounts.unit)));
        }

        #[test]
        fn test_name_conversion() {
            TEST_AMOUNTS
                .iter()
                .for_each(|amounts| test_from_name(amounts.name, AleoAmount(amounts.unit)));
        }

        #[test]
        fn test_aleo_conversion() {
            TEST_AMOUNTS
                .iter()
                .for_each(|amounts| test_from_aleo(amounts.aleo, AleoAmount(amounts.unit)));
        }
    }

    mod valid_arithmetic {
        use super::*;

        const TEST_VALUES: [(i128, i128, i128); 7] = [
            (0, 0, 0),
            (1, 2, 3),
            (100000, 0, 100000),
            (123456789, 987654321, 1111111110),
            (100000000000000, 1000000000000000, 1100000000000000),
            (-100000000000000, -1000000000000000, -1100000000000000),
            (MAX_COINS, -MAX_COINS, 0),
        ];

        #[test]
        fn test_valid_addition() {
            TEST_VALUES.iter().for_each(|(a, b, c)| test_addition(a, b, c));
        }

        #[test]
        fn test_valid_subtraction() {
            TEST_VALUES.iter().for_each(|(a, b, c)| test_subtraction(c, b, a));
        }
    }

    mod test_invalid {
        use super::*;

        mod test_out_of_bounds {
            use super::*;

            const INVALID_TEST_AMOUNTS: [AmountDenominationTestCase; 4] = [
                AmountDenominationTestCase {
                    unit: 1_000_000_000_001_000_000_000_000_000_000,
                    byte: 1_000_000_000_001_000_000_000_000_000,
                    cycle: 1_000_000_000_001_000_000_000_000,
                    gate: 1_000_000_000_001_000_000_000,
                    name: 1_000_000_000_001_000_000,
                    aleo: 1_000_000_000_001,
                },
                AmountDenominationTestCase {
                    unit: -1_000_000_000_001_000_000_000_000_000_000,
                    byte: -1_000_000_000_001_000_000_000_000_000,
                    cycle: -1_000_000_000_001_000_000_000_000,
                    gate: -1_000_000_000_001_000_000_000,
                    name: -1_000_000_000_001_000_000,
                    aleo: -1_000_000_000_001,
                },
                AmountDenominationTestCase {
                    unit: 1_000_000_000_000_001_000_000_000_000_000_000,
                    byte: 1_000_000_000_000_001_000_000_000_000_000,
                    cycle: 1_000_000_000_000_001_000_000_000_000,
                    gate: 1_000_000_000_000_001_000_000_000,
                    name: 1_000_000_000_000_001_000_000,
                    aleo: 1_000_000_000_000_001,
                },
                AmountDenominationTestCase {
                    unit: -1_000_000_000_000_001_000_000_000_000_000_000,
                    byte: -1_000_000_000_000_001_000_000_000_000_000,
                    cycle: -1_000_000_000_000_001_000_000_000_000,
                    gate: -1_000_000_000_000_001_000_000_000,
                    name: -1_000_000_000_000_001_000_000,
                    aleo: -1_000_000_000_000_001,
                },
            ];

            #[should_panic(expected = "AmountOutOfBounds")]
            #[test]
            fn test_invalid_unit_conversion() {
                INVALID_TEST_AMOUNTS
                    .iter()
                    .for_each(|amounts| test_from_units(amounts.unit, AleoAmount(amounts.unit)));
            }

            #[should_panic(expected = "AmountOutOfBounds")]
            #[test]
            fn test_invalid_byte_conversion() {
                INVALID_TEST_AMOUNTS
                    .iter()
                    .for_each(|amounts| test_from_byte(amounts.byte, AleoAmount(amounts.unit)));
            }

            #[should_panic(expected = "AmountOutOfBounds")]
            #[test]
            fn test_invalid_cycle_conversion() {
                INVALID_TEST_AMOUNTS
                    .iter()
                    .for_each(|amounts| test_from_cycle(amounts.cycle, AleoAmount(amounts.unit)));
            }

            #[should_panic(expected = "AmountOutOfBounds")]
            #[test]
            fn test_invalid_gate_conversion() {
                INVALID_TEST_AMOUNTS
                    .iter()
                    .for_each(|amounts| test_from_gate(amounts.gate, AleoAmount(amounts.unit)));
            }

            #[should_panic(expected = "AmountOutOfBounds")]
            #[test]
            fn test_invalid_name_conversion() {
                INVALID_TEST_AMOUNTS
                    .iter()
                    .for_each(|amounts| test_from_name(amounts.name, AleoAmount(amounts.unit)));
            }

            #[should_panic(expected = "AmountOutOfBounds")]
            #[test]
            fn test_invalid_aleo_conversion() {
                INVALID_TEST_AMOUNTS
                    .iter()
                    .for_each(|amounts| test_from_aleo(amounts.aleo, AleoAmount(amounts.unit)));
            }
        }

        mod test_invalid_conversion {
            use super::*;

            const INVALID_TEST_AMOUNTS: [AmountDenominationTestCase; 4] = [
                AmountDenominationTestCase {
                    unit: 1,
                    byte: 1,
                    cycle: 1,
                    gate: 1,
                    name: 1,
                    aleo: 1,
                },
                AmountDenominationTestCase {
                    unit: 1,
                    byte: 10,
                    cycle: 100,
                    gate: 1000,
                    name: 1000000,
                    aleo: 100000000,
                },
                AmountDenominationTestCase {
                    unit: 123456789,
                    byte: 1234567,
                    cycle: 1234,
                    gate: 123,
                    name: 12,
                    aleo: 1,
                },
                AmountDenominationTestCase {
                    unit: 1_000_000_000_000_000_000_000_000_000_000,
                    byte: 1_000_000_000_000_000_000_000_000_000,
                    cycle: 1_000_000_000_000_000_000_000_000,
                    gate: 1_000_000_000_000_000_000_000,
                    name: 1_000_000_000_000_000_000,
                    aleo: 999_999_999_999,
                },
            ];

            #[should_panic]
            #[test]
            fn test_invalid_byte_conversion() {
                INVALID_TEST_AMOUNTS
                    .iter()
                    .for_each(|amounts| test_from_byte(amounts.byte, AleoAmount(amounts.unit)));
            }

            #[should_panic]
            #[test]
            fn test_invalid_cycle_conversion() {
                INVALID_TEST_AMOUNTS
                    .iter()
                    .for_each(|amounts| test_from_cycle(amounts.cycle, AleoAmount(amounts.unit)));
            }

            #[should_panic]
            #[test]
            fn test_invalid_gate_conversion() {
                INVALID_TEST_AMOUNTS
                    .iter()
                    .for_each(|amounts| test_from_gate(amounts.gate, AleoAmount(amounts.unit)));
            }

            #[should_panic]
            #[test]
            fn test_invalid_name_conversion() {
                INVALID_TEST_AMOUNTS
                    .iter()
                    .for_each(|amounts| test_from_name(amounts.name, AleoAmount(amounts.unit)));
            }

            #[should_panic]
            #[test]
            fn test_invalid_aleo_conversion() {
                INVALID_TEST_AMOUNTS
                    .iter()
                    .for_each(|amounts| test_from_aleo(amounts.aleo, AleoAmount(amounts.unit)));
            }
        }

        mod invalid_arithmetic {
            use super::*;

            const TEST_VALUES: [(i128, i128, i128); 8] = [
                (0, 0, 1),
                (1, 2, 5),
                (100000, 1, 100000),
                (123456789, 123456789, 123456789),
                (-1000, -1000, 2000),
                (MAX_COINS, 1, 1_000_000_000_000_000_000_000_000_000_001),
                (MAX_COINS, MAX_COINS, 2_000_000_000_000_000_000_000_000_000_000),
                (-MAX_COINS, -MAX_COINS, -2_000_000_000_000_000_000_000_000_000_001),
            ];

            #[should_panic]
            #[test]
            fn test_invalid_addition() {
                TEST_VALUES.iter().for_each(|(a, b, c)| test_addition(a, b, c));
            }

            #[should_panic]
            #[test]
            fn test_invalid_subtraction() {
                TEST_VALUES.iter().for_each(|(a, b, c)| test_subtraction(a, b, c));
            }
        }
    }
}
