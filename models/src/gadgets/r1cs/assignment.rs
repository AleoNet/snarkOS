use snarkvm_errors::gadgets::SynthesisError;

pub trait Assignment<T> {
    fn get(self) -> Result<T, SynthesisError>;
}

impl<T> Assignment<T> for Option<T> {
    fn get(self) -> Result<T, SynthesisError> {
        match self {
            Some(v) => Ok(v),
            None => Err(SynthesisError::AssignmentMissing),
        }
    }
}
