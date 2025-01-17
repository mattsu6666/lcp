use crate::prelude::*;
use flex_error::*;
use sgx_types::sgx_status_t;

define_error! {
    #[derive(Debug, Clone, PartialEq, Eq)]
    Error {
        SgxError
        {
            status: sgx_status_t,
        }
        |e| {
            format_args!("SGX error: status={:?}", e.status)
        },

        EnclaveKeyNotFound
        |_| { "Enclave Key not found" },

        Crypto
        [crypto::Error]
        |_| { "Crypto error" },

        AttestationReport
        [attestation_report::Error]
        |_| { "AttestationReport error" },

        RemoteAttestation
        [enclave_remote_attestation::Error]
        |_| { "RemoteAttestation error" },

        Time
        [lcp_types::TimeError]
        |_| { "Time error" },

        InvalidSpIdLength
        {
            length: usize
        }
        |e| {
            format_args!("invalid SPID length: expected=32 actual={}", e.length)
        }
    }
}

impl From<enclave_remote_attestation::Error> for Error {
    fn from(err: enclave_remote_attestation::Error) -> Self {
        Error::remote_attestation(err)
    }
}

impl From<attestation_report::Error> for Error {
    fn from(err: attestation_report::Error) -> Self {
        Error::attestation_report(err)
    }
}

impl From<crypto::Error> for Error {
    fn from(err: crypto::Error) -> Self {
        Error::crypto(err)
    }
}
