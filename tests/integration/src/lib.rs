mod relayer;
#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;
    use enclave_api::{Enclave, EnclavePrimitiveAPI};
    use enclave_commands::{
        CommandResult, CommitmentProofPair, EnclaveManageResult, LightClientResult,
        UpdateClientResult,
    };
    use ibc::core::{
        ics23_commitment::{commitment::CommitmentProofBytes, merkle::MerkleProof},
        ics24_host::identifier::{ChannelId, PortId},
    };
    use log::*;
    use std::{str::FromStr, sync::Arc};
    use tempdir::TempDir;
    use tokio::runtime::Runtime as TokioRuntime;

    static ENCLAVE_FILE: &'static str = "../../bin/enclave.signed.so";

    #[test]
    fn test_verification() {
        assert!(_test_verification().is_ok());
    }

    fn _test_verification() -> Result<(), anyhow::Error> {
        env_logger::init();
        let rt = Arc::new(TokioRuntime::new()?);

        let spid = std::env::var("SPID")?;
        let ias_key = std::env::var("IAS_KEY")?;

        let enclave = match host::enclave::load_enclave(ENCLAVE_FILE) {
            Ok(r) => {
                info!("Init Enclave Successful {}!", r.geteid());
                r
            }
            Err(x) => {
                panic!("Init Enclave Failed {}!", x.as_str());
            }
        };

        let tmp_dir = TempDir::new("lcp").unwrap();
        let home = tmp_dir.path().to_str().unwrap().to_string();
        let enclave = Enclave::new(enclave, home);

        let report = match enclave.init_enclave_key(spid.as_bytes(), ias_key.as_bytes()) {
            Ok(CommandResult::EnclaveManage(EnclaveManageResult::InitEnclave(res))) => res.report,
            Err(e) => {
                panic!("ECALL Enclave Failed {:?}!", e);
            }
            _ => unreachable!(),
        };
        let quote = attestation_report::parse_quote_from_report(&report.report).unwrap();
        info!("report={:?}", quote.raw.report_body.report_data.d);

        // register the key into onchain

        let mut rly = relayer::create_relayer(rt, "ibc0")?;

        // XXX use non-latest height here
        let initial_height = rly
            .query_latest_height()?
            .decrement()?
            .decrement()?
            .decrement()?;

        let (client_state, consensus_state) = rly.fetch_state_as_any(initial_height)?;
        info!(
            "initial_height: {:?} client_state: {:?}, consensus_state: {:?}",
            initial_height, client_state, consensus_state
        );

        let proof = if let CommandResult::LightClient(LightClientResult::InitClient(res)) = enclave
            .init_client(client_state.into(), consensus_state.into())
            .unwrap()
        {
            res.0
        } else {
            panic!("unexpected result type")
        };
        let commitment = proof.commitment();
        let client_id = commitment.client_id;

        info!("generated client id is {}", client_id.as_str().to_string());

        let target_header = rly.create_header(
            commitment
                .new_height
                .try_into()
                .map_err(|e| anyhow!("{:?}", e))?,
            commitment
                .new_height
                .increment()
                .try_into()
                .map_err(|e| anyhow!("{:?}", e))?,
        )?;
        let res = enclave.update_client(client_id.clone(), target_header.into())?;

        info!("update_client's result is {:?}", res);

        let height = match res {
            CommandResult::LightClient(LightClientResult::UpdateClient(UpdateClientResult(
                res,
            ))) => res.commitment().new_height,
            _ => unreachable!(),
        };

        info!("current height is {}", height);

        let (port_id, channel_id) = (
            PortId::from_str("cross")?,
            ChannelId::from_str("channel-0")?,
        );
        let res = rly.proven_channel(
            &port_id,
            &channel_id,
            Some(height.try_into().map_err(|e| anyhow!("{:?}", e))?),
        )?;

        info!("expected channel is {:?}", res.0);

        let _ = enclave.verify_channel(
            client_id.clone(),
            res.0,
            "ibc".into(),
            port_id,
            channel_id,
            CommitmentProofPair(
                res.2.try_into().map_err(|e| anyhow!("{:?}", e))?,
                merkle_proof_to_bytes(res.1)?,
            ),
        )?;

        enclave.destroy();
        Ok(())
    }

    fn merkle_proof_to_bytes(proof: MerkleProof) -> Result<Vec<u8>, anyhow::Error> {
        let proof = CommitmentProofBytes::try_from(proof)?;
        Ok(proof.into())
    }
}
