use super::registry::get_light_client_by_client_id;
use crate::light_client::Error;
use commitments::prover::prove_state_commitment;
use context::Context;
use ecall_commands::{
    LightClientResult, VerifyMembershipInput, VerifyMembershipResult, VerifyNonMembershipInput,
    VerifyNonMembershipResult,
};
use light_client_registry::LightClientResolver;
use store::KVStore;

pub fn verify_membership<R: LightClientResolver, S: KVStore>(
    ctx: &mut Context<R, S>,
    input: VerifyMembershipInput,
) -> Result<LightClientResult, Error> {
    let ek = ctx.get_enclave_key();
    let lc = get_light_client_by_client_id(ctx, &input.client_id)?;

    let res = lc.verify_membership(
        ctx,
        input.client_id,
        input.prefix,
        input.path,
        input.value,
        input.proof.0,
        input.proof.1,
    )?;

    Ok(LightClientResult::VerifyMembership(VerifyMembershipResult(
        prove_state_commitment(ek, res.state_commitment)?,
    )))
}

pub fn verify_non_membership<R: LightClientResolver, S: KVStore>(
    ctx: &mut Context<R, S>,
    input: VerifyNonMembershipInput,
) -> Result<LightClientResult, Error> {
    let ek = ctx.get_enclave_key();
    let lc = get_light_client_by_client_id(ctx, &input.client_id)?;

    let res = lc.verify_non_membership(
        ctx,
        input.client_id,
        input.prefix,
        input.path,
        input.proof.0,
        input.proof.1,
    )?;

    Ok(LightClientResult::VerifyNonMembership(
        VerifyNonMembershipResult(prove_state_commitment(ek, res.state_commitment)?),
    ))
}
