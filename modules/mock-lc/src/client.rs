use crate::errors::Error;
use crate::prelude::*;
use commitments::{gen_state_id, gen_state_id_from_any, UpdateClientCommitment};
use ibc::core::ics02_client::client_consensus::AnyConsensusState;
use ibc::core::ics02_client::client_def::{AnyClient, ClientDef};
use ibc::core::ics02_client::client_state::{
    AnyClientState, ClientState, MOCK_CLIENT_STATE_TYPE_URL,
};
use ibc::core::ics02_client::client_type::ClientType;
use ibc::core::ics02_client::error::Error as ICS02Error;
use ibc::core::ics02_client::header::{AnyHeader, Header};
use ibc::core::ics24_host::identifier::ClientId;
use lcp_types::{Any, Height, Time};
use light_client::{
    ClientReader, CreateClientResult, Error as LightClientError, LightClient,
    StateVerificationResult, UpdateClientResult,
};
use light_client_registry::LightClientRegistry;
use validation_context::ValidationParams;

#[derive(Default)]
pub struct MockLightClient;

impl LightClient for MockLightClient {
    fn create_client(
        &self,
        _: &dyn ClientReader,
        any_client_state: Any,
        any_consensus_state: Any,
    ) -> Result<CreateClientResult, LightClientError> {
        let state_id = gen_state_id_from_any(&any_client_state, &any_consensus_state)
            .map_err(Error::commitment)?;
        let client_state = match AnyClientState::try_from(any_client_state.clone()) {
            Ok(AnyClientState::Mock(client_state)) => AnyClientState::Mock(client_state),
            #[allow(unreachable_patterns)]
            Ok(s) => return Err(Error::unexpected_client_type(s.client_type().to_string()).into()),
            Err(e) => return Err(Error::ics02(e).into()),
        };
        let consensus_state = match AnyConsensusState::try_from(any_consensus_state) {
            Ok(AnyConsensusState::Mock(consensus_state)) => {
                AnyConsensusState::Mock(consensus_state)
            }
            #[allow(unreachable_patterns)]
            Ok(s) => return Err(Error::unexpected_client_type(s.client_type().to_string()).into()),
            Err(e) => return Err(Error::ics02(e).into()),
        };

        let height = client_state.latest_height().into();
        let timestamp: Time = consensus_state.timestamp().into();
        Ok(CreateClientResult {
            height,
            commitment: UpdateClientCommitment {
                prev_state_id: None,
                new_state_id: state_id,
                new_state: Some(any_client_state.into()),
                prev_height: None,
                new_height: height,
                timestamp,
                validation_params: ValidationParams::Empty,
            },
            prove: false,
        })
    }

    fn update_client(
        &self,
        ctx: &dyn ClientReader,
        client_id: ClientId,
        any_header: Any,
    ) -> Result<UpdateClientResult, LightClientError> {
        let ctx = ctx.as_ibc_client_reader();
        let header = match AnyHeader::try_from(any_header) {
            Ok(AnyHeader::Mock(header)) => AnyHeader::Mock(header),
            #[allow(unreachable_patterns)]
            Ok(h) => return Err(Error::unexpected_client_type(h.client_type().to_string()).into()),
            Err(e) => return Err(Error::ics02(e).into()),
        };

        // Read client type from the host chain store. The client should already exist.
        let client_type = ctx.client_type(&client_id).map_err(Error::ics02)?;

        let client_def = AnyClient::from_client_type(client_type);

        // Read client state from the host chain store.
        let client_state = ctx.client_state(&client_id).map_err(Error::ics02)?;

        if client_state.is_frozen() {
            return Err(Error::ics02(ICS02Error::client_frozen(client_id)).into());
        }

        let height = header.height().into();
        let header_timestamp = header.timestamp().into();

        let latest_height = client_state.latest_height();

        // Read consensus state from the host chain store.
        let latest_consensus_state =
            ctx.consensus_state(&client_id, latest_height)
                .map_err(|_| {
                    Error::ics02(ICS02Error::consensus_state_not_found(
                        client_id.clone(),
                        latest_height,
                    ))
                })?;

        // Use client_state to validate the new header against the latest consensus_state.
        // This function will return the new client_state (its latest_height changed) and a
        // consensus_state obtained from header. These will be later persisted by the keeper.
        let (new_client_state, new_consensus_state) = client_def
            .check_header_and_update_state(ctx, client_id, client_state.clone(), header)
            .map_err(|e| Error::ics02(ICS02Error::header_verification_failure(e.to_string())))?;

        let prev_state_id =
            gen_state_id(client_state, latest_consensus_state).map_err(Error::commitment)?;
        let new_state_id = gen_state_id(new_client_state.clone(), new_consensus_state.clone())
            .map_err(Error::commitment)?;
        let new_any_client_state = Any::try_from(new_client_state).unwrap();

        Ok(UpdateClientResult {
            new_any_client_state: new_any_client_state.clone(),
            new_any_consensus_state: Any::try_from(new_consensus_state).unwrap(),
            height,
            commitment: UpdateClientCommitment {
                prev_state_id: Some(prev_state_id),
                new_state_id,
                new_state: new_any_client_state.into(),
                prev_height: Some(latest_height.into()),
                new_height: height,
                timestamp: header_timestamp,
                validation_params: ValidationParams::Empty,
            },
            prove: true,
        })
    }

    #[allow(unused_variables)]
    fn verify_membership(
        &self,
        ctx: &dyn ClientReader,
        client_id: ClientId,
        prefix: Vec<u8>,
        path: String,
        value: Vec<u8>,
        proof_height: Height,
        proof: Vec<u8>,
    ) -> Result<StateVerificationResult, LightClientError> {
        todo!()
    }

    #[allow(unused_variables)]
    fn verify_non_membership(
        &self,
        ctx: &dyn ClientReader,
        client_id: ClientId,
        prefix: Vec<u8>,
        path: String,
        proof_height: Height,
        proof: Vec<u8>,
    ) -> Result<StateVerificationResult, LightClientError> {
        todo!()
    }

    fn client_type(&self) -> String {
        ClientType::Mock.as_str().to_owned()
    }

    fn latest_height(
        &self,
        ctx: &dyn ClientReader,
        client_id: &ClientId,
    ) -> Result<Height, LightClientError> {
        let client_state = ctx
            .as_ibc_client_reader()
            .client_state(client_id)
            .map_err(Error::ics02)?;
        Ok(client_state.latest_height().into())
    }
}

pub fn register_implementations(registry: &mut dyn LightClientRegistry) {
    registry
        .put_light_client(
            MOCK_CLIENT_STATE_TYPE_URL.to_string(),
            Box::new(MockLightClient),
        )
        .unwrap()
}
