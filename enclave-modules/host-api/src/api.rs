use crate::prelude::*;
use crate::{errors::Error, ffi};
use ocall_commands::{Command, CommandResult};
use sgx_types::*;

pub fn execute_command(cmd: Command) -> Result<CommandResult, Error> {
    let cmd_vec = bincode::serialize(&cmd).map_err(Error::bincode)?;
    let mut ret: sgx_status_t = sgx_status_t::SGX_ERROR_UNEXPECTED;
    let mut output_len = 0;
    let output_maxlen = 65536;
    let mut output_buf = Vec::with_capacity(output_maxlen);
    let output_ptr = output_buf.as_mut_ptr();

    let result = unsafe {
        ffi::ocall_execute_command(
            &mut ret,
            cmd_vec.as_ptr(),
            cmd_vec.len() as u32,
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
