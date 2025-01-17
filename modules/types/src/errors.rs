use crate::prelude::*;
use flex_error::*;

define_error! {
    #[derive(Debug, Clone, PartialEq, Eq)]
    TypeError {
        HeightBytesConversion
            {
                bz: Vec<u8>,
            }
            |e| {
                format_args!("height bytes length must be 16, but got {:?}", e.bz)
            }
    }
}

define_error! {
    #[derive(Debug, Clone, PartialEq, Eq)]
    TimeError {
        Tendermint
            [tendermint::Error]
            |_| {
                "tendermint error"
            }
    }
}
