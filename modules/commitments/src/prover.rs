use crate::errors::Error;
use crate::prelude::*;
use crate::{
    StateCommitment, StateCommitmentProof, UpdateClientCommitment, UpdateClientCommitmentProof,
};
use crypto::{Signer, Verifier};

pub fn prove_update_client_commitment(
    signer: &dyn Signer,
    commitment: UpdateClientCommitment,
) -> Result<UpdateClientCommitmentProof, Error> {
    let commitment_bytes = commitment.to_vec();
    let signature = signer.sign(&commitment_bytes).map_err(Error::crypto)?;
    let mut signer_address = Default::default();
    signer.use_verifier(&mut |verifier: &dyn Verifier| {
        signer_address = verifier.get_address();
    });
    Ok(UpdateClientCommitmentProof::new(
        commitment_bytes,
        signer_address,
        signature,
    ))
}

pub fn prove_state_commitment(
    signer: &dyn Signer,
    commitment: StateCommitment,
) -> Result<StateCommitmentProof, Error> {
    let commitment_bytes = commitment.to_vec();
    let signature = signer.sign(&commitment_bytes).map_err(Error::crypto)?;
    let mut signer_address = Default::default();
    signer.use_verifier(&mut |verifier: &dyn Verifier| {
        signer_address = verifier.get_address();
    });
    Ok(StateCommitmentProof {
        commitment_bytes,
        signer: signer_address,
        signature,
    })
}
