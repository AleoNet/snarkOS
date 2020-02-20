use crate::message::hash::HASH96;
use snarkos_errors::network::message::MessageNameError;

use std::{fmt, str};

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct MessageName(HASH96);

impl MessageName {
    pub fn len(&self) -> usize {
        let trailing_zeros = self.0.iter().rev().take_while(|&x| x == &0).count();
        self.0.len() - trailing_zeros
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_zero()
    }

    pub fn as_bytes(&self) -> [u8; 12] {
        let mut result = [0u8; 12];
        result[..12].copy_from_slice(&self.0[..12]);
        result
    }

    fn as_string(&self) -> String {
        String::from_utf8_lossy(&self.0[..self.len()]).to_ascii_lowercase()
    }
}

impl str::FromStr for MessageName {
    type Err = MessageNameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.is_ascii() || s.len() > 12 {
            return Err(MessageNameError::InvalidLength(s.len()));
        }

        let mut result = HASH96::default();
        result[..s.len()].copy_from_slice(s.as_ref());
        Ok(MessageName(result))
    }
}

impl From<&'static str> for MessageName {
    fn from(s: &'static str) -> Self {
        s.parse().unwrap()
    }
}

impl From<MessageName> for String {
    fn from(c: MessageName) -> Self {
        c.as_string()
    }
}

impl From<[u8; 12]> for MessageName {
    fn from(bytes: [u8; 12]) -> Self {
        Self { 0: HASH96::from(bytes) }
    }
}

impl fmt::Display for MessageName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.as_string())
    }
}

impl<'a> PartialEq<&'a str> for MessageName {
    fn eq(&self, other: &&'a str) -> bool {
        self.len() == other.len() && &self.0[..other.len()] == other.as_ref() as &[u8]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_parse() {
        //        println!("{:?}", serde_json::to_vec(&MessageName::from("helloooo")).unwrap());
        //        println!("{:?}", serde_json::to_vec(&MessageName::from("h")).unwrap());
        let command: MessageName = "ping".into();
        assert_eq!(MessageName("70696e670000000000000000".into()), command);
    }

    #[test]
    fn test_command_to_string() {
        let command: MessageName = "ping".into();
        let expected: String = "ping".into();
        assert_eq!(expected, String::from(command))
    }
}
