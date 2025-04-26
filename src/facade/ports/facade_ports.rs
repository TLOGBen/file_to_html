use std::io;
use crate::models::conversion::{ConversionInput, ConversionOutput};

// Facade 接口
pub trait ConversionFacadeTrait: Send + Sync {
    fn execute_conversion(&self, input: ConversionInput) -> io::Result<ConversionOutput>;
}
