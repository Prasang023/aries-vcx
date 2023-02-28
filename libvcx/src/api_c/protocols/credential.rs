use std::ptr;

use futures::future::BoxFuture;
use libc::c_char;

use libvcx_core::api_vcx::api_handle::credential;
use libvcx_core::errors;
use libvcx_core::errors::error::{LibvcxError, LibvcxErrorKind};

use crate::api_c::cutils::cstring::CStringUtils;
use crate::api_c::cutils::current_error::set_current_error_vcx;
use crate::api_c::cutils::runtime::{execute, execute_async};
use crate::api_c::types::CommandHandle;
use crate::error::SUCCESS_ERR_CODE;

/*
    The API represents a Holder side in credential issuance process.
    Assumes that pairwise connection between Issuer and Holder is already established.

    # State

    The set of object states, messages and transitions depends on the communication method is used.
    The communication method can be specified as a config option on one of *_init functions.
        VcxStateType::VcxStateRequestReceived - once `vcx_credential_create_with_offer` (create Credential object) is called.

        VcxStateType::VcxStateOfferSent - once `vcx_credential_send_request` (send `CredentialRequest` message) is called.

        VcxStateType::VcxStateAccepted - once `Credential` messages is received.
        VcxStateType::None - once `ProblemReport` messages is received.
                                                use `vcx_credential_update_state` or `vcx_credential_update_state_with_message` functions for state updates.

    # Transitions
    RFC - https://github.com/hyperledger/aries-rfcs/tree/7b6b93acbaf9611d3c892c4bada142fe2613de6e/features/0036-issue-credential
        VcxStateType::None - `vcx_credential_create_with_offer` - VcxStateType::VcxStateRequestReceived

        VcxStateType::VcxStateRequestReceived - `vcx_issuer_send_credential_offer` - VcxStateType::VcxStateOfferSent

        VcxStateType::VcxStateOfferSent - received `Credential` - VcxStateType::VcxStateAccepted
        VcxStateType::VcxStateOfferSent - received `ProblemReport` - VcxStateType::None

    # Messages
        CredentialProposal - https://github.com/hyperledger/aries-rfcs/tree/7b6b93acbaf9611d3c892c4bada142fe2613de6e/features/0036-issue-credential#propose-credential
        CredentialOffer - https://github.com/hyperledger/aries-rfcs/tree/7b6b93acbaf9611d3c892c4bada142fe2613de6e/features/0036-issue-credential#offer-credential
        CredentialRequest - https://github.com/hyperledger/aries-rfcs/tree/7b6b93acbaf9611d3c892c4bada142fe2613de6e/features/0036-issue-credential#request-credential
        Credential - https://github.com/hyperledger/aries-rfcs/tree/7b6b93acbaf9611d3c892c4bada142fe2613de6e/features/0036-issue-credential#issue-credential
        ProblemReport - https://github.com/hyperledger/aries-rfcs/tree/7b6b93acbaf9611d3c892c4bada142fe2613de6e/features/0035-report-problem#the-problem-report-message-type
        Ack - https://github.com/hyperledger/aries-rfcs/tree/master/features/0015-acks#explicit-acks
*/

/// Create a Credential object that requests and receives a credential for an institution
///
/// #Params
/// command_handle: command handle to map callback to user context.
///
/// source_id: Institution's personal identification for the credential, should be unique.
///
/// offer: credential offer received via "vcx_credential_get_offers"
///
/// # Example
/// offer -> depends on communication method:
///     aries:
///         {"@type":"did:sov:BzCbsNYhMrjHiqZDTUASHg;spec/issue-credential/1.0/offer-credential", "@id":"<uuid-of-offer-message>", "comment":"somecomment", "credential_preview":<json-ldobject>, "offers~attach":[{"@id":"libindy-cred-offer-0", "mime-type":"application/json", "data":{"base64":"<bytesforbase64>"}}]}
///
/// cb: Callback that provides credential handle or error status
///
/// #Returns
/// Error code as a u32
#[no_mangle]
#[allow(unused_variables, unused_mut)]
pub extern "C" fn vcx_credential_create_with_offer(
    command_handle: CommandHandle,
    source_id: *const c_char,
    offer: *const c_char,
    cb: Option<extern "C" fn(xcommand_handle: CommandHandle, err: u32, credential_handle: u32)>,
) -> u32 {
    info!("vcx_credential_create_with_offer >>>");

    check_useful_c_callback!(cb, LibvcxErrorKind::InvalidOption);
    check_useful_c_str!(source_id, LibvcxErrorKind::InvalidOption);
    check_useful_c_str!(offer, LibvcxErrorKind::InvalidOption);

    trace!(
        "vcx_credential_create_with_offer(command_handle: {}, source_id: {}, offer: {})",
        command_handle,
        source_id,
        secret!(&offer)
    );

    execute(move || {
        match credential::credential_create_with_offer(&source_id, &offer) {
            Ok(err) => {
                trace!(
                    "vcx_credential_create_with_offer_cb(command_handle: {}, source_id: {}, rc: {}, handle: {})",
                    command_handle,
                    source_id,
                    SUCCESS_ERR_CODE,
                    err
                );
                cb(command_handle, SUCCESS_ERR_CODE, err)
            }
            Err(err) => {
                set_current_error_vcx(&err);
                error!(
                    "vcx_credential_create_with_offer_cb(command_handle: {}, source_id: {}, rc: {}, handle: {})",
                    command_handle, source_id, err, 0
                );
                cb(command_handle, err.into(), 0);
            }
        };

        Ok(())
    });

    SUCCESS_ERR_CODE
}

/// Retrieve information about a stored credential in user's wallet, including credential id and the credential itself.
///
/// #Params
/// command_handle: command handle to map callback to user context.
///
/// credential_handle: credential handle that was provided during creation. Used to identify credential object
///
/// cb: Callback that provides error status of api call, or returns the credential in json format of "{uuid:credential}".
///
/// # Example
/// credential -> depends on communication method:
///     aries:
///         https://github.com/hyperledger/aries-rfcs/tree/master/features/0036-issue-credential#issue-credential
///
/// #Returns
/// Error code as a u32
#[no_mangle]
#[allow(unused_variables, unused_mut)]
pub extern "C" fn vcx_get_credential(
    command_handle: CommandHandle,
    credential_handle: u32,
    cb: Option<extern "C" fn(xcommand_handle: CommandHandle, err: u32, credential: *const c_char)>,
) -> u32 {
    info!("vcx_get_credential >>>");

    check_useful_c_callback!(cb, LibvcxErrorKind::InvalidOption);
    let source_id = credential::get_source_id(credential_handle).unwrap_or_default();
    trace!(
        "vcx_get_credential(command_handle: {}, credential_handle: {}) source_id: {})",
        command_handle,
        credential_handle,
        source_id
    );

    execute(move || {
        match credential::get_credential(credential_handle) {
            Ok(s) => {
                trace!(
                    "vcx_get_credential_cb(commmand_handle: {}, rc: {}, msg: {}) source_id: {}",
                    command_handle,
                    SUCCESS_ERR_CODE,
                    s,
                    source_id
                );
                let msg = CStringUtils::string_to_cstring(s);
                cb(command_handle, SUCCESS_ERR_CODE, msg.as_ptr());
            }
            Err(err) => {
                set_current_error_vcx(&err);
                error!(
                    "vcx_get_credential_cb(commmand_handle: {}, rc: {}, msg: {}) source_id: {}",
                    command_handle,
                    err,
                    "".to_string(),
                    source_id
                );
                cb(command_handle, err.into(), ptr::null_mut());
            }
        };

        Ok(())
    });

    SUCCESS_ERR_CODE
}

/// Delete a Credential from the wallet and release its handle.
///
/// # Params
/// command_handle: command handle to map callback to user context.
///
/// credential_handle: handle of the credential to delete.
///
/// cb: Callback that provides error status of delete credential request
///
/// # Returns
/// Error code as a u32
#[no_mangle]
#[allow(unused_assignments)]
pub extern "C" fn vcx_delete_credential(
    command_handle: CommandHandle,
    credential_handle: u32,
    cb: Option<extern "C" fn(xcommand_handle: CommandHandle, err: u32)>,
) -> u32 {
    info!("vcx_delete_credential >>>");

    check_useful_c_callback!(cb, LibvcxErrorKind::InvalidOption);
    let source_id = credential::get_source_id(credential_handle).unwrap_or_default();
    trace!(
        "vcx_delete_credential(command_handle: {}, credential_handle: {}), source_id: {})",
        command_handle,
        credential_handle,
        source_id
    );

    execute_async::<BoxFuture<'static, Result<(), ()>>>(Box::pin(async move {
        match credential::delete_credential(credential_handle).await {
            Ok(()) => {
                trace!(
                    "vcx_delete_credential_cb(command_handle: {}, rc: {}), credential_handle: {}, source_id: {})",
                    command_handle,
                    SUCCESS_ERR_CODE,
                    credential_handle,
                    source_id
                );
                cb(command_handle, SUCCESS_ERR_CODE);
            }
            Err(err) => {
                set_current_error_vcx(&err);
                trace!(
                    "vcx_delete_credential_cb(command_handle: {}, rc: {}), credential_handle: {}, source_id: {})",
                    command_handle,
                    err,
                    credential_handle,
                    source_id
                );
                cb(command_handle, err.into());
            }
        }

        Ok(())
    }));

    SUCCESS_ERR_CODE
}

#[no_mangle]
pub extern "C" fn vcx_credential_get_attributes(
    command_handle: CommandHandle,
    credential_handle: u32,
    cb: Option<extern "C" fn(xcommand_handle: CommandHandle, err: u32, attributes: *const c_char)>,
) -> u32 {
    info!(
        "vcx_credential_get_attributes >>> credential_handle: {:?}",
        credential_handle
    );

    check_useful_c_callback!(cb, LibvcxErrorKind::InvalidOption);
    let source_id = credential::get_source_id(credential_handle).unwrap_or_default();
    trace!(
        "vcx_credential_get_attributes(command_handle: {}, credential_handle: {}) source_id: {})",
        command_handle,
        credential_handle,
        source_id
    );

    execute(move || {
        match credential::get_attributes(credential_handle) {
            Ok(s) => {
                trace!(
                    "vcx_credential_get_attribute_cb(commmand_handle: {}, rc: {}, attributes: {}) source_id: {}",
                    command_handle,
                    SUCCESS_ERR_CODE,
                    s,
                    source_id
                );
                let attrs = CStringUtils::string_to_cstring(s);
                cb(command_handle, SUCCESS_ERR_CODE, attrs.as_ptr());
            }
            Err(err) => {
                set_current_error_vcx(&err);
                error!(
                    "vcx_credential_get_attributes_cb(commmand_handle: {}, rc: {}, attributes: {}) source_id: {}",
                    command_handle,
                    err,
                    "".to_string(),
                    source_id
                );
                cb(command_handle, err.into(), ptr::null_mut());
            }
        };

        Ok(())
    });

    SUCCESS_ERR_CODE
}

#[no_mangle]
pub extern "C" fn vcx_credential_get_attachment(
    command_handle: CommandHandle,
    credential_handle: u32,
    cb: Option<extern "C" fn(xcommand_handle: CommandHandle, err: u32, attachment: *const c_char)>,
) -> u32 {
    info!(
        "vcx_credential_get_attachment >>> credential_handle: {:?}",
        credential_handle
    );

    check_useful_c_callback!(cb, LibvcxErrorKind::InvalidOption);
    let source_id = credential::get_source_id(credential_handle).unwrap_or_default();
    trace!(
        "vcx_credential_get_attachment(command_handle: {}, credential_handle: {}) source_id: {})",
        command_handle,
        credential_handle,
        source_id
    );

    execute(move || {
        match credential::get_attachment(credential_handle) {
            Ok(s) => {
                trace!(
                    "vcx_credential_get_attachment_cb(commmand_handle: {}, rc: {}, attachment: {}) source_id: {}",
                    command_handle,
                    SUCCESS_ERR_CODE,
                    s,
                    source_id
                );
                let attach = CStringUtils::string_to_cstring(s);
                cb(command_handle, SUCCESS_ERR_CODE, attach.as_ptr());
            }
            Err(err) => {
                set_current_error_vcx(&err);
                error!(
                    "vcx_credential_get_attachment_cb(commmand_handle: {}, rc: {}, attachment: {}) source_id: {}",
                    command_handle,
                    err,
                    "".to_string(),
                    source_id
                );
                cb(command_handle, err.into(), ptr::null_mut());
            }
        };

        Ok(())
    });

    SUCCESS_ERR_CODE
}

#[no_mangle]
pub extern "C" fn vcx_credential_get_tails_location(
    command_handle: CommandHandle,
    credential_handle: u32,
    cb: Option<extern "C" fn(xcommand_handle: CommandHandle, err: u32, location: *const c_char)>,
) -> u32 {
    info!(
        "vcx_credential_get_tails_location >>> credential_handle: {:?}",
        credential_handle
    );

    check_useful_c_callback!(cb, LibvcxErrorKind::InvalidOption);
    let source_id = credential::get_source_id(credential_handle).unwrap_or_default();
    trace!(
        "vcx_credential_get_tails_location(command_handle: {}, credential_handle: {}) source_id: {})",
        command_handle,
        credential_handle,
        source_id
    );

    execute(move || {
        match credential::get_tails_location(credential_handle) {
            Ok(s) => {
                trace!(
                    "vcx_credential_get_tails_location_cb(commmand_handle: {}, rc: {}, location: {}) source_id: {}",
                    command_handle,
                    SUCCESS_ERR_CODE,
                    s,
                    source_id
                );
                let location = CStringUtils::string_to_cstring(s);
                cb(command_handle, SUCCESS_ERR_CODE, location.as_ptr());
            }
            Err(err) => {
                set_current_error_vcx(&err);
                error!(
                    "vcx_credential_get_tails_location_cb(commmand_handle: {}, rc: {}, location: {}) source_id: {}",
                    command_handle,
                    err,
                    "".to_string(),
                    source_id
                );
                cb(command_handle, err.into(), ptr::null_mut());
            }
        };

        Ok(())
    });

    SUCCESS_ERR_CODE
}

#[no_mangle]
pub extern "C" fn vcx_credential_get_tails_hash(
    command_handle: CommandHandle,
    credential_handle: u32,
    cb: Option<extern "C" fn(xcommand_handle: CommandHandle, err: u32, hash: *const c_char)>,
) -> u32 {
    info!(
        "vcx_credential_get_tails_hash >>> credential_handle: {:?}",
        credential_handle
    );

    check_useful_c_callback!(cb, LibvcxErrorKind::InvalidOption);
    let source_id = credential::get_source_id(credential_handle).unwrap_or_default();
    trace!(
        "vcx_credential_get_tails_hash(command_handle: {}, credential_handle: {}) source_id: {})",
        command_handle,
        credential_handle,
        source_id
    );

    execute(move || {
        match credential::get_tails_hash(credential_handle) {
            Ok(s) => {
                trace!(
                    "vcx_credential_get_tails_hash_cb(commmand_handle: {}, rc: {}, hash: {}) source_id: {}",
                    command_handle,
                    SUCCESS_ERR_CODE,
                    s,
                    source_id
                );
                let hash = CStringUtils::string_to_cstring(s);
                cb(command_handle, SUCCESS_ERR_CODE, hash.as_ptr());
            }
            Err(err) => {
                set_current_error_vcx(&err);
                error!(
                    "vcx_credential_get_tails_hash_cb(commmand_handle: {}, rc: {}, hash: {}) source_id: {}",
                    command_handle,
                    err,
                    "".to_string(),
                    source_id
                );
                cb(command_handle, err.into(), ptr::null_mut());
            }
        };

        Ok(())
    });

    SUCCESS_ERR_CODE
}

#[no_mangle]
pub extern "C" fn vcx_credential_get_rev_reg_id(
    command_handle: CommandHandle,
    credential_handle: u32,
    cb: Option<extern "C" fn(xcommand_handle: CommandHandle, err: u32, rev_reg_id: *const c_char)>,
) -> u32 {
    info!(
        "vcx_credential_get_rev_reg_id >>> credential_handle: {:?}",
        credential_handle
    );

    check_useful_c_callback!(cb, LibvcxErrorKind::InvalidOption);
    let source_id = credential::get_source_id(credential_handle).unwrap_or_default();
    trace!(
        "vcx_credential_get_rev_reg_id(command_handle: {}, credential_handle: {}) source_id: {})",
        command_handle,
        credential_handle,
        source_id
    );

    execute(move || {
        match credential::get_rev_reg_id(credential_handle) {
            Ok(s) => {
                trace!(
                    "vcx_credential_get_rev_reg_id_cb(commmand_handle: {}, rc: {}, rev_reg_id: {}) source_id: {}",
                    command_handle,
                    SUCCESS_ERR_CODE,
                    s,
                    source_id
                );
                let rev_reg_id = CStringUtils::string_to_cstring(s);
                cb(command_handle, SUCCESS_ERR_CODE, rev_reg_id.as_ptr());
            }
            Err(err) => {
                set_current_error_vcx(&err);
                error!(
                    "vcx_credential_get_rev_reg_id_cb(commmand_handle: {}, rc: {}, rev_reg_id: {}) source_id: {}",
                    command_handle,
                    err,
                    "".to_string(),
                    source_id
                );
                cb(command_handle, err.into(), ptr::null_mut());
            }
        };

        Ok(())
    });

    SUCCESS_ERR_CODE
}

#[no_mangle]
pub extern "C" fn vcx_credential_get_thread_id(
    command_handle: CommandHandle,
    credential_handle: u32,
    cb: Option<extern "C" fn(xcommand_handle: CommandHandle, err: u32, thread_id: *const c_char)>,
) -> u32 {
    info!(
        "vcx_credential_get_thread_id >>> credential_handle: {:?}",
        credential_handle
    );

    check_useful_c_callback!(cb, LibvcxErrorKind::InvalidOption);
    let source_id = credential::get_source_id(credential_handle).unwrap_or_default();
    trace!(
        "vcx_credential_get_thread_id(command_handle: {}, credential_handle: {}) source_id: {})",
        command_handle,
        credential_handle,
        source_id
    );

    execute(move || {
        match credential::get_thread_id(credential_handle) {
            Ok(s) => {
                trace!(
                    "vcx_credential_get_thread_id_cb(commmand_handle: {}, rc: {}, thread_id: {}) source_id: {}",
                    command_handle,
                    SUCCESS_ERR_CODE,
                    s,
                    source_id
                );
                let thread_id = CStringUtils::string_to_cstring(s);
                cb(command_handle, SUCCESS_ERR_CODE, thread_id.as_ptr());
            }
            Err(err) => {
                error!(
                    "vcx_credential_get_thread_id_cb(commmand_handle: {}, rc: {}, thread_id: {}) source_id: {}",
                    command_handle,
                    err,
                    "".to_string(),
                    source_id
                );
                cb(command_handle, err.into(), ptr::null_mut());
            }
        };

        Ok(())
    });

    SUCCESS_ERR_CODE
}

#[no_mangle]
pub extern "C" fn vcx_credential_is_revokable(
    command_handle: CommandHandle,
    credential_handle: u32,
    cb: Option<extern "C" fn(xcommand_handle: CommandHandle, err: u32, revokable: bool)>,
) -> u32 {
    info!(
        "vcx_credential_is_revokable >>> credential_handle: {:?}",
        credential_handle
    );

    check_useful_c_callback!(cb, LibvcxErrorKind::InvalidOption);
    let source_id = credential::get_source_id(credential_handle).unwrap_or_default();
    trace!(
        "vcx_credential_is_revokable(command_handle: {}, credential_handle: {}) source_id: {})",
        command_handle,
        credential_handle,
        source_id
    );

    execute_async::<BoxFuture<'static, Result<(), ()>>>(Box::pin(async move {
        match credential::is_revokable(credential_handle).await {
            Ok(revokable) => {
                trace!(
                    "vcx_credential_is_revokable_cb(commmand_handle: {}, rc: {}, revokable: {}) source_id: {}",
                    command_handle,
                    SUCCESS_ERR_CODE,
                    revokable,
                    source_id
                );
                cb(command_handle, SUCCESS_ERR_CODE, revokable);
            }
            Err(err) => {
                error!(
                    "vcx_credential_is_revokable_cb(commmand_handle: {}, rc: {}) source_id: {}",
                    command_handle, err, source_id
                );
                cb(command_handle, err.into(), false);
            }
        };

        Ok(())
    }));

    SUCCESS_ERR_CODE
}

/// Create a Credential object based off of a known message id for a given connection.
///
/// #Params
/// command_handle: command handle to map callback to user context.
///
/// source_id: Institution's personal identification for the credential, should be unique.
///
/// connection_handle: connection to query for credential offer
///
/// msg_id: msg_id that contains the credential offer
///
/// cb: Callback that provides credential handle or error status
///
/// #Returns
/// Error code as a u32
#[no_mangle]
#[allow(unused_variables, unused_mut)]
pub extern "C" fn vcx_credential_create_with_msgid(
    command_handle: CommandHandle,
    source_id: *const c_char,
    connection_handle: u32,
    msg_id: *const c_char,
    cb: Option<extern "C" fn(xcommand_handle: CommandHandle, err: u32, credential_handle: u32, offer: *const c_char)>,
) -> u32 {
    info!("vcx_credential_create_with_msgid >>>");

    check_useful_c_callback!(cb, LibvcxErrorKind::InvalidOption);
    check_useful_c_str!(source_id, LibvcxErrorKind::InvalidOption);
    check_useful_c_str!(msg_id, LibvcxErrorKind::InvalidOption);

    trace!(
        "vcx_credential_create_with_msgid(command_handle: {}, source_id: {}, connection_handle: {}, msg_id: {})",
        command_handle,
        source_id,
        connection_handle,
        msg_id
    );

    execute_async::<BoxFuture<'static, Result<(), ()>>>(Box::pin(async move {
        match credential::credential_create_with_msgid(&source_id, connection_handle, &msg_id).await {
            Ok((handle, offer_string)) => {
                let c_offer = CStringUtils::string_to_cstring(offer_string);
                trace!("vcx_credential_create_with_offer_cb(command_handle: {}, source_id: {}, rc: {}, handle: {}) source_id: {}", command_handle, source_id, SUCCESS_ERR_CODE, handle, source_id);
                cb(command_handle, SUCCESS_ERR_CODE, handle, c_offer.as_ptr())
            }
            Err(err) => {
                error!("vcx_credential_create_with_offer_cb(command_handle: {}, source_id: {}, rc: {}, handle: {}) source_id: {}", command_handle, source_id, err, 0, source_id);
                cb(command_handle, err.into(), 0, ptr::null_mut());
            }
        };

        Ok(())
    }));

    SUCCESS_ERR_CODE
}

/// Approves the credential offer and submits a credential request. The result will be a credential stored in the prover's wallet.
///
/// #params
/// command_handle: command handle to map callback to user context
///
/// credential_handle: credential handle that was provided during creation. Used to identify credential object
///
/// connection_handle: Connection handle that identifies pairwise connection
///
/// cb: Callback that provides error status of credential request
///
/// #Returns
/// Error code as a u32
#[no_mangle]
pub extern "C" fn vcx_credential_send_request(
    command_handle: CommandHandle,
    credential_handle: u32,
    connection_handle: u32,
    _payment_handle: u32,
    cb: Option<extern "C" fn(xcommand_handle: CommandHandle, err: u32)>,
) -> u32 {
    info!("vcx_credential_send_request >>>");

    check_useful_c_callback!(cb, LibvcxErrorKind::InvalidOption);

    let source_id = credential::get_source_id(credential_handle).unwrap_or_default();
    trace!("vcx_credential_send_request(command_handle: {}, credential_handle: {}, connection_handle: {}), source_id: {:?}", command_handle, credential_handle, connection_handle, source_id);

    execute_async::<BoxFuture<'static, Result<(), ()>>>(Box::pin(async move {
        match credential::send_credential_request(credential_handle, connection_handle).await {
            Ok(()) => {
                trace!(
                    "vcx_credential_send_request_cb(command_hanndle: {}, rc: {}) source_id: {}",
                    command_handle,
                    SUCCESS_ERR_CODE,
                    source_id
                );
                cb(command_handle, SUCCESS_ERR_CODE);
            }
            Err(err) => {
                error!(
                    "vcx_credential_send_request_cb(command_handle: {}, rc: {}) source_id: {}",
                    command_handle, err, source_id
                );
                cb(command_handle, err.into());
            }
        };

        Ok(())
    }));

    SUCCESS_ERR_CODE
}

/// Approves the credential offer and gets the credential request message that can be sent to the specified connection
///
/// #params
/// command_handle: command handle to map callback to user context
///
/// credential_handle: credential handle that was provided during creation. Used to identify credential object
///
/// my_pw_did: Use Connection api (vcx_connection_get_pw_did) with specified connection_handle to retrieve your pw_did
///
/// their_pw_did: Use Connection api (vcx_connection_get_their_pw_did) with specified connection_handle to retrieve theri pw_did
///
/// cb: Callback that provides error status of credential request
///
/// #Returns
/// Error code as a u32
#[no_mangle]
pub extern "C" fn vcx_credential_get_request_msg(
    command_handle: CommandHandle,
    credential_handle: u32,
    my_pw_did: *const c_char,
    their_pw_did: *const c_char,
    _payment_handle: u32,
    cb: Option<extern "C" fn(xcommand_handle: CommandHandle, err: u32, msg: *const c_char)>,
) -> u32 {
    info!("vcx_credential_get_request_msg >>>");

    check_useful_c_str!(my_pw_did, LibvcxErrorKind::InvalidOption);
    check_useful_opt_c_str!(their_pw_did, LibvcxErrorKind::InvalidOption);
    check_useful_c_callback!(cb, LibvcxErrorKind::InvalidOption);

    let source_id = credential::get_source_id(credential_handle).unwrap_or_default();
    trace!("vcx_credential_get_request_msg(command_handle: {}, credential_handle: {}, my_pw_did: {}, their_pw_did: {:?}), source_id: {:?}", command_handle, credential_handle, my_pw_did, their_pw_did, source_id);

    execute(move || {
        match credential::generate_credential_request_msg(
            credential_handle,
            &my_pw_did,
            &their_pw_did.unwrap_or_default(),
        ) {
            Ok(msg) => {
                let msg = CStringUtils::string_to_cstring(msg);
                trace!(
                    "vcx_credential_get_request_msg_cb(command_handle: {}, rc: {}) source_id: {}",
                    command_handle,
                    SUCCESS_ERR_CODE,
                    source_id
                );
                cb(command_handle, SUCCESS_ERR_CODE, msg.as_ptr());
            }
            Err(err) => {
                error!(
                    "vcx_credential_get_request_msg_cb(command_handle: {}, rc: {}) source_id: {}",
                    command_handle, err, source_id
                );
                cb(command_handle, err.into(), ptr::null_mut());
            }
        };

        Ok(())
    });

    SUCCESS_ERR_CODE
}

/// Queries agency for credential offers from the given connection.
///
/// #Params
/// command_handle: command handle to map callback to user context.
///
/// connection_handle: Connection to query for credential offers.
///
/// cb: Callback that provides any credential offers and error status of query
///
/// # Example offers -> "[[{"msg_type": "CREDENTIAL_OFFER","version": "0.1","to_did": "...","from_did":"...","credential": {"account_num": ["...."],"name_on_account": ["Alice"]},"schema_seq_no": 48,"issuer_did": "...","credential_name": "Account Certificate","credential_id": "3675417066","msg_ref_id": "ymy5nth"}]]"
///
/// #Returns
/// Error code as a u32
#[no_mangle]
pub extern "C" fn vcx_credential_get_offers(
    command_handle: CommandHandle,
    connection_handle: u32,
    cb: Option<extern "C" fn(xcommand_handle: CommandHandle, err: u32, credential_offers: *const c_char)>,
) -> u32 {
    info!("vcx_credential_get_offers >>>");

    check_useful_c_callback!(cb, LibvcxErrorKind::InvalidOption);

    trace!(
        "vcx_credential_get_offers(command_handle: {}, connection_handle: {})",
        command_handle,
        connection_handle
    );

    execute_async::<BoxFuture<'static, Result<(), ()>>>(Box::pin(async move {
        match credential::get_credential_offer_messages_with_conn_handle(connection_handle).await {
            Ok(err) => {
                trace!(
                    "vcx_credential_get_offers_cb(command_handle: {}, rc: {}, msg: {})",
                    command_handle,
                    err.to_string(),
                    err
                );
                let msg = CStringUtils::string_to_cstring(err);
                cb(command_handle, SUCCESS_ERR_CODE, msg.as_ptr());
            }
            Err(err) => {
                set_current_error_vcx(&err);
                error!(
                    "vcx_credential_get_offers_cb(command_handle: {}, rc: {}, msg: null)",
                    command_handle, err
                );
                cb(command_handle, err.into(), ptr::null_mut());
            }
        };

        Ok(())
    }));

    SUCCESS_ERR_CODE
}

#[no_mangle]
pub extern "C" fn vcx_credential_decline_offer(
    command_handle: CommandHandle,
    credential_handle: u32,
    connection_handle: u32,
    comment: *const c_char,
    cb: Option<extern "C" fn(xcommand_handle: CommandHandle, err: u32)>,
) -> u32 {
    info!("vcx_credential_decline_offer >>>");

    check_useful_c_callback!(cb, LibvcxErrorKind::InvalidOption);
    check_useful_opt_c_str!(comment, LibvcxErrorKind::InvalidOption);

    let source_id = credential::get_source_id(credential_handle).unwrap_or_default();
    trace!("vcx_credential_decline_offer(command_handle: {}, credential_handle: {}, connection_handle: {}), source_id: {:?}", command_handle, credential_handle, connection_handle, source_id);

    execute_async::<BoxFuture<'static, Result<(), ()>>>(Box::pin(async move {
        match credential::decline_offer(credential_handle, connection_handle, comment.as_deref()).await {
            Ok(()) => {
                trace!(
                    "vcx_credential_decline_offer_cb(command_handle: {}, rc: {}) source_id: {}",
                    command_handle,
                    SUCCESS_ERR_CODE,
                    source_id
                );
                cb(command_handle, SUCCESS_ERR_CODE);
            }
            Err(err) => {
                error!(
                    "vcx_credential_decline_offer_cb(command_handle: {}, rc: {}) source_id: {}",
                    command_handle, err, source_id
                );
                cb(command_handle, err.into());
            }
        };

        Ok(())
    }));

    SUCCESS_ERR_CODE
}

/// Query the agency for the received messages.
/// Checks for any messages changing state in the credential object and updates the state attribute.
/// If it detects a credential it will store the credential in the wallet.
///
/// #Params
/// command_handle: command handle to map callback to user context.
///
/// credential_handle: Credential handle that was provided during creation. Used to identify credential object
///
/// connection_handle: Connection handle of the credential interaction is associated with.
///
/// cb: Callback that provides most current state of the credential and error status of request
///
/// #Returns
/// Error code as a u32
#[no_mangle]
pub extern "C" fn vcx_v2_credential_update_state(
    command_handle: CommandHandle,
    credential_handle: u32,
    connection_handle: u32,
    cb: Option<extern "C" fn(xcommand_handle: CommandHandle, err: u32, state: u32)>,
) -> u32 {
    info!("vcx_v2_credential_update_state >>>");

    check_useful_c_callback!(cb, LibvcxErrorKind::InvalidOption);

    let source_id = credential::get_source_id(credential_handle).unwrap_or_default();
    trace!("vcx_v2_credential_update_state(command_handle: {}, credential_handle: {}, connection_handle: {}), source_id: {:?}", command_handle, credential_handle, connection_handle, source_id);

    execute_async::<BoxFuture<'static, Result<(), ()>>>(Box::pin(async move {
        match credential::update_state(credential_handle, None, connection_handle).await {
            Ok(state) => {
                trace!(
                    "vcx_v2_credential_update_state_cb(command_handle: {}, rc: {}, state: {}), source_id: {:?}",
                    command_handle,
                    SUCCESS_ERR_CODE,
                    state,
                    source_id
                );
                cb(command_handle, SUCCESS_ERR_CODE, state)
            }
            Err(err) => {
                error!(
                    "vcx_v2_credential_update_state_cb(command_handle: {}, rc: {}, state: {}), source_id: {:?}",
                    command_handle, err, 0, source_id
                );
                cb(command_handle, err.into(), 0)
            }
        }
        Ok(())
    }));

    SUCCESS_ERR_CODE
}

/// Update the state of the credential based on the given message.
///
/// #Params
/// command_handle: command handle to map callback to user context.
///
/// credential_handle: Credential handle that was provided during creation. Used to identify credential object
///
/// connection_handle: Connection handle of the credential interaction is associated with.
///
/// message: message to process for state changes
///
/// cb: Callback that provides most current state of the credential and error status of request
///
/// #Returns
/// Error code as a u32
#[no_mangle]
pub extern "C" fn vcx_v2_credential_update_state_with_message(
    command_handle: CommandHandle,
    credential_handle: u32,
    connection_handle: u32,
    message: *const c_char,
    cb: Option<extern "C" fn(xcommand_handle: CommandHandle, err: u32, state: u32)>,
) -> u32 {
    info!("vcx_v2_credential_update_state_with_message >>>");

    check_useful_c_callback!(cb, LibvcxErrorKind::InvalidOption);
    check_useful_c_str!(message, LibvcxErrorKind::InvalidOption);

    let source_id = credential::get_source_id(credential_handle).unwrap_or_default();
    trace!(
        "vcx_v2_credential_update_state_with_message(command_handle: {}, credential_handle: {}), source_id: {:?}",
        command_handle,
        credential_handle,
        source_id
    );

    execute_async::<BoxFuture<'static, Result<(), ()>>>(Box::pin(async move {
        match credential::update_state(credential_handle, Some(&message), connection_handle).await {
            Ok(state) => {
                trace!("vcx_v2_credential_update_state_with_message_cb(command_handle: {}, rc: {}, state: {}), source_id: {:?}", command_handle, SUCCESS_ERR_CODE, state, source_id);
                cb(command_handle, SUCCESS_ERR_CODE, state)
            }
            Err(err) => {
                error!("vcx_v2_credential_update_state_with_message_cb(command_handle: {}, rc: {}, state: {}), source_id: {:?}", command_handle, err, 0, source_id);
                cb(command_handle, err.into(), 0)
            }
        }
        Ok(())
    }));

    SUCCESS_ERR_CODE
}

/// Get the current state of the credential object
///
/// #Params
/// command_handle: command handle to map callback to user context.
///
/// proof_handle: Credential handle that was provided during creation.
///
/// cb: Callback that provides most current state of the credential and error status of request
///     Credential statuses:
///         2 - Request Sent
///         3 - Request Received
///         4 - Accepted
///
/// #Returns
#[no_mangle]
pub extern "C" fn vcx_credential_get_state(
    command_handle: CommandHandle,
    handle: u32,
    cb: Option<extern "C" fn(xcommand_handle: CommandHandle, err: u32, state: u32)>,
) -> u32 {
    info!("vcx_credential_get_state >>>");

    check_useful_c_callback!(cb, LibvcxErrorKind::InvalidOption);

    let source_id = credential::get_source_id(handle).unwrap_or_default();
    trace!(
        "vcx_credential_get_state(command_handle: {}, credential_handle: {}), source_id: {:?}",
        command_handle,
        handle,
        source_id
    );

    execute(move || {
        match credential::get_state(handle) {
            Ok(state) => {
                trace!(
                    "vcx_credential_get_state_cb(command_handle: {}, rc: {}, state: {}), source_id: {:?}",
                    command_handle,
                    SUCCESS_ERR_CODE,
                    state,
                    source_id
                );
                cb(command_handle, SUCCESS_ERR_CODE, state)
            }
            Err(err) => {
                error!(
                    "vcx_credential_get_state_cb(command_handle: {}, rc: {}, state: {}), source_id: {:?}",
                    command_handle, err, 0, source_id
                );
                cb(command_handle, err.into(), 0)
            }
        };

        Ok(())
    });

    SUCCESS_ERR_CODE
}

/// Takes the credential object and returns a json string of all its attributes
///
/// #Params
/// command_handle: command handle to map callback to user context.
///
/// handle: Credential handle that was provided during creation. Used to identify credential object
///
/// cb: Callback that provides json string of the credential's attributes and provides error status
///
/// #Returns
/// Error code as a u32
#[no_mangle]
pub extern "C" fn vcx_credential_serialize(
    command_handle: CommandHandle,
    handle: u32,
    cb: Option<extern "C" fn(xcommand_handle: CommandHandle, err: u32, data: *const c_char)>,
) -> u32 {
    info!("vcx_credential_serialize >>>");

    check_useful_c_callback!(cb, LibvcxErrorKind::InvalidOption);

    let source_id = credential::get_source_id(handle).unwrap_or_default();
    trace!(
        "vcx_credential_serialize(command_handle: {}, credential_handle: {}), source_id: {:?}",
        command_handle,
        handle,
        source_id
    );

    execute(move || {
        match credential::to_string(handle) {
            Ok(serialized) => {
                trace!(
                    "vcx_credential_serialize_cb(command_handle: {}, rc: {}, data: {}), source_id: {:?}",
                    command_handle,
                    SUCCESS_ERR_CODE,
                    serialized,
                    source_id
                );
                let c_serialized = CStringUtils::string_to_cstring(serialized);
                cb(command_handle, SUCCESS_ERR_CODE, c_serialized.as_ptr());
            }
            Err(err) => {
                set_current_error_vcx(&err);
                error!(
                    "vcx_credential_serialize_cb(command_handle: {}, rc: {}, data: {}), source_id: {:?}",
                    command_handle, err, 0, source_id
                );
                cb(command_handle, err.into(), ptr::null_mut());
            }
        };

        Ok(())
    });

    SUCCESS_ERR_CODE
}

/// Takes a json string representing an credential object and recreates an object matching the json
///
/// #Params
/// command_handle: command handle to map callback to user context.
///
/// credential_data: json string representing a credential object
///
///
/// cb: Callback that provides credential handle and provides error status
///
/// #Returns
/// Error code as a u32
#[no_mangle]
pub extern "C" fn vcx_credential_deserialize(
    command_handle: CommandHandle,
    credential_data: *const c_char,
    cb: Option<extern "C" fn(xcommand_handle: CommandHandle, err: u32, handle: u32)>,
) -> u32 {
    info!("vcx_credential_deserialize >>>");

    check_useful_c_callback!(cb, LibvcxErrorKind::InvalidOption);
    check_useful_c_str!(credential_data, LibvcxErrorKind::InvalidOption);

    trace!(
        "vcx_credential_deserialize(command_handle: {}, credential_data: {})",
        command_handle,
        credential_data
    );

    execute(move || {
        match credential::from_string(&credential_data) {
            Ok(handle) => {
                trace!(
                    "vcx_credential_deserialize_cb(command_handle: {}, rc: {}, credential_handle: {}) source_id: {}",
                    command_handle,
                    SUCCESS_ERR_CODE,
                    handle,
                    credential::get_source_id(handle).unwrap_or_default()
                );

                cb(command_handle, SUCCESS_ERR_CODE, handle);
            }
            Err(err) => {
                set_current_error_vcx(&err);
                error!(
                    "vcx_credential_deserialize_cb(command_handle: {}, rc: {}, credential_handle: {}) source_id: {}",
                    command_handle, err, 0, ""
                );
                cb(command_handle, err.into(), 0);
            }
        };

        Ok(())
    });

    SUCCESS_ERR_CODE
}

/// Releases the credential object by de-allocating memory
///
/// #Params
/// handle: Credential handle that was provided during creation. Used to access credential object
///
/// #Returns
/// Success
#[no_mangle]
pub extern "C" fn vcx_credential_release(handle: u32) -> u32 {
    info!("vcx_credential_release >>>");

    let source_id = credential::get_source_id(handle).unwrap_or_default();
    match credential::release(handle) {
        Ok(()) => {
            trace!(
                "vcx_credential_release(handle: {}, rc: {}), source_id: {:?}",
                handle,
                SUCCESS_ERR_CODE,
                source_id
            );
            SUCCESS_ERR_CODE
        }

        Err(err) => {
            error!(
                "vcx_credential_release(handle: {}, rc: {}), source_id: {:?}",
                handle, err, source_id
            );
            err.into()
        }
    }
}

#[cfg(feature = "general_test")]
#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use serde_json::Value;

    use aries_vcx::agency_client::testing::mocking::AgencyMockDecrypted;
    use aries_vcx::protocols::issuance::holder::state_machine::HolderState;
    use aries_vcx::utils::constants::{GET_MESSAGES_DECRYPTED_RESPONSE, V3_OBJECT_SERIALIZE_VERSION};
    use aries_vcx::utils::devsetup::SetupMocks;
    use aries_vcx::utils::mockdata::mockdata_credex::{
        ARIES_CREDENTIAL_OFFER, ARIES_CREDENTIAL_RESPONSE, CREDENTIAL_SM_FINISHED,
    };
    use libvcx_core::api_vcx::api_handle::credential::tests_utils::BAD_CREDENTIAL_OFFER;
    use libvcx_core::api_vcx::api_handle::mediated_connection;
    use libvcx_core::api_vcx::api_handle::mediated_connection::test_utils::{
        build_test_connection_invitee_completed, build_test_connection_inviter_requested,
    };
    use libvcx_core::errors;

    use crate::api_c::cutils::return_types_u32;
    use crate::api_c::cutils::timeout::TimeoutUtils;

    use super::*;

    fn _vcx_credential_create_with_offer_c_closure(offer: &str) -> Result<u32, u32> {
        let cb = return_types_u32::Return_U32_U32::new().unwrap();
        let rc = vcx_credential_create_with_offer(
            cb.command_handle,
            CString::new("test_create").unwrap().into_raw(),
            CString::new(offer).unwrap().into_raw(),
            Some(cb.get_callback()),
        );
        if rc != SUCCESS_ERR_CODE {
            return Err(rc);
        }

        let handle = cb.receive(TimeoutUtils::some_medium());
        handle
    }

    #[test]
    #[cfg(feature = "general_test")]
    fn test_vcx_credential_create_with_offer_success() {
        let _setup = SetupMocks::init();

        let handle = _vcx_credential_create_with_offer_c_closure(ARIES_CREDENTIAL_OFFER).unwrap();
        assert!(handle > 0);
    }

    #[test]
    #[cfg(feature = "general_test")]
    fn test_vcx_credential_create_with_offer_fails() {
        let _setup = SetupMocks::init();

        let err = _vcx_credential_create_with_offer_c_closure(BAD_CREDENTIAL_OFFER).unwrap_err();
        assert_eq!(err, u32::from(LibvcxErrorKind::InvalidJson));
    }

    #[test]
    #[cfg(feature = "general_test")]
    fn test_vcx_credential_serialize_and_deserialize() {
        let _setup = SetupMocks::init();

        let handle = _vcx_credential_create_with_offer_c_closure(ARIES_CREDENTIAL_OFFER).unwrap();

        let cb = return_types_u32::Return_U32_STR::new().unwrap();
        assert_eq!(
            vcx_credential_serialize(cb.command_handle, handle, Some(cb.get_callback())),
            SUCCESS_ERR_CODE
        );
        let credential_json = cb.receive(TimeoutUtils::some_short()).unwrap().unwrap();
        debug!("Serialized credential: {:?}", credential_json);

        let object: Value = serde_json::from_str(&credential_json).unwrap();
        assert_eq!(object["version"], V3_OBJECT_SERIALIZE_VERSION);

        let cb = return_types_u32::Return_U32_U32::new().unwrap();
        assert_eq!(
            vcx_credential_deserialize(
                cb.command_handle,
                CString::new(credential_json).unwrap().into_raw(),
                Some(cb.get_callback()),
            ),
            SUCCESS_ERR_CODE
        );
        let handle = cb.receive(TimeoutUtils::some_short()).unwrap();
        assert!(handle > 0);
    }

    #[tokio::test]
    #[cfg(feature = "general_test")]
    async fn test_vcx_credential_get_new_offers() {
        let _setup = SetupMocks::init();

        let handle_conn = build_test_connection_invitee_completed();

        let cb = return_types_u32::Return_U32_STR::new().unwrap();
        assert_eq!(
            vcx_credential_get_offers(cb.command_handle, handle_conn, Some(cb.get_callback())),
            SUCCESS_ERR_CODE as u32
        );
        cb.receive(TimeoutUtils::some_medium()).unwrap();
    }

    #[tokio::test]
    #[cfg(feature = "general_test")]
    async fn test_vcx_credential_create() {
        let _setup = SetupMocks::init();

        let handle_conn = build_test_connection_invitee_completed();

        let cb = return_types_u32::Return_U32_U32_STR::new().unwrap();
        assert_eq!(
            vcx_credential_create_with_msgid(
                cb.command_handle,
                CString::new("test_vcx_credential_create").unwrap().into_raw(),
                handle_conn,
                CString::new("123").unwrap().into_raw(),
                Some(cb.get_callback()),
            ),
            SUCCESS_ERR_CODE
        );
        cb.receive(TimeoutUtils::some_medium()).unwrap();
    }

    #[test]
    #[cfg(feature = "general_test")]
    fn test_vcx_credential_get_state() {
        let _setup = SetupMocks::init();

        let handle = _vcx_credential_create_with_offer_c_closure(ARIES_CREDENTIAL_OFFER).unwrap();

        let cb = return_types_u32::Return_U32_U32::new().unwrap();
        assert_eq!(
            vcx_credential_get_state(cb.command_handle, handle, Some(cb.get_callback())),
            SUCCESS_ERR_CODE
        );
        assert_eq!(
            cb.receive(TimeoutUtils::some_medium()).unwrap(),
            HolderState::OfferReceived as u32
        );
    }

    #[tokio::test]
    #[cfg(feature = "general_test")]
    async fn test_vcx_credential_update_state() {
        let _setup = SetupMocks::init();

        let handle_conn = build_test_connection_inviter_requested().await;

        let handle_cred = _vcx_credential_create_with_offer_c_closure(ARIES_CREDENTIAL_OFFER).unwrap();
        assert_eq!(
            credential::get_state(handle_cred).unwrap(),
            HolderState::OfferReceived as u32
        );
        debug!("credential handle = {}", handle_cred);

        let cb = return_types_u32::Return_U32::new().unwrap();
        assert_eq!(
            vcx_credential_send_request(cb.command_handle, handle_cred, handle_conn, 0, Some(cb.get_callback())),
            SUCCESS_ERR_CODE
        );
        cb.receive(TimeoutUtils::some_medium()).unwrap();
        assert_eq!(
            credential::get_state(handle_cred).unwrap(),
            HolderState::RequestSent as u32
        );

        AgencyMockDecrypted::set_next_decrypted_response(GET_MESSAGES_DECRYPTED_RESPONSE);
        AgencyMockDecrypted::set_next_decrypted_message(ARIES_CREDENTIAL_RESPONSE);
        let cb = return_types_u32::Return_U32_U32::new().unwrap();
        assert_eq!(
            vcx_v2_credential_update_state(cb.command_handle, handle_cred, handle_conn, Some(cb.get_callback())),
            SUCCESS_ERR_CODE
        );
        cb.receive(TimeoutUtils::some_medium()).unwrap();
        assert_eq!(
            credential::get_state(handle_cred).unwrap(),
            HolderState::Finished as u32
        );

        let cb = return_types_u32::Return_U32_STR::new().unwrap();
        assert_eq!(
            vcx_credential_get_rev_reg_id(cb.command_handle, handle_cred, Some(cb.get_callback())),
            SUCCESS_ERR_CODE
        );
        let rev_reg_id = cb.receive(TimeoutUtils::some_medium()).unwrap().unwrap();
        let rev_reg_id_expected =
            String::from("V4SGRU86Z58d6TV7PBUe6f:4:V4SGRU86Z58d6TV7PBUe6f:3:CL:1281:tag1:CL_ACCUM:tag1");
        assert_eq!(rev_reg_id, rev_reg_id_expected);
    }

    #[tokio::test]
    #[cfg(feature = "general_test")]
    async fn test_get_credential() {
        let _setup = SetupMocks::init();

        let handle_cred = credential::from_string(CREDENTIAL_SM_FINISHED).unwrap();
        let cb = return_types_u32::Return_U32_STR::new().unwrap();
        assert_eq!(
            vcx_get_credential(cb.command_handle, handle_cred, Some(cb.get_callback())),
            SUCCESS_ERR_CODE
        );
        cb.receive(TimeoutUtils::some_medium()).unwrap().unwrap();

        let bad_handle = 1123;
        let cb = return_types_u32::Return_U32_STR::new().unwrap();
        assert_eq!(
            vcx_get_credential(cb.command_handle, bad_handle, Some(cb.get_callback())),
            SUCCESS_ERR_CODE
        );
    }
}