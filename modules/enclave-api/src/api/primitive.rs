use crate::{
    enclave::{EnclaveInfo, HostStoreTxManager},
    ffi, Error, Result,
};
use ecall_commands::{Command, CommandParams, CommandResult, ECallCommand};
use log::*;
use sgx_types::{sgx_enclave_id_t, sgx_status_t};
use store::transaction::{CommitStore, Tx};

pub trait EnclavePrimitiveAPI<S: CommitStore>: EnclaveInfo + HostStoreTxManager<S> {
    /// execute_command runs a given command in the enclave
    fn execute_command(&self, cmd: Command, update_key: Option<String>) -> Result<CommandResult> {
        debug!("execute_command: cmd={:?} update_key={:?}", cmd, update_key);
        let tx = self.begin_tx(update_key)?;
        let ecmd = ECallCommand::new(CommandParams::new(self.get_home(), tx.get_id()), cmd);
        match raw_execute_command(self.get_eid(), ecmd) {
            Ok(res) => {
                self.commit_tx(tx)?;
                debug!("execute_command succeeded: res={:?}", res);
                Ok(res)
            }
            Err(e) => {
                self.rollback_tx(tx);
                debug!("execute_command failed: err={:?}", e);
                Err(e)
            }
        }
    }
}

fn raw_execute_command(eid: sgx_enclave_id_t, cmd: ECallCommand) -> Result<CommandResult> {
    let mut output_len = 0;
    let output_maxlen = 65536;
    let mut output_buf = Vec::with_capacity(output_maxlen);
    let output_ptr = output_buf.as_mut_ptr();
    let mut ret = sgx_status_t::SGX_SUCCESS;

    let command_bytes = bincode::serialize(&cmd).map_err(Error::bincode)?;
    let result = unsafe {
        ffi::ecall_execute_command(
            eid,
            &mut ret,
            command_bytes.as_ptr(),
            command_bytes.len() as u32,
            output_ptr,
            output_maxlen as u32,
            &mut output_len,
        )
    };
    if result != sgx_status_t::SGX_SUCCESS {
        Err(Error::sgx_error(result))
    } else {
        assert!((output_len as usize) < output_maxlen);
        unsafe {
            output_buf.set_len(output_len as usize);
        }
        let res =
            bincode::deserialize(&output_buf[..output_len as usize]).map_err(Error::bincode)?;

        if ret == sgx_status_t::SGX_SUCCESS {
            Ok(res)
        } else if let CommandResult::CommandError(descr) = res {
            Err(Error::command(ret, descr))
        } else {
            unreachable!()
        }
    }
}
