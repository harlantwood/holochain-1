use crate::core::ribosome::CallContext;
use crate::core::ribosome::RibosomeT;
use crate::core::ribosome::{error::RibosomeResult, ZomeCallInvocation};
use holochain_zome_types::{CallInput, ZomeCallResponse};
use holochain_zome_types::{CallOutput, ExternInput};
use std::sync::Arc;

pub fn call(
    _ribosome: Arc<impl RibosomeT>,
    call_context: Arc<CallContext>,
    input: CallInput,
) -> RibosomeResult<CallOutput> {
    // Get the input
    let call = input.into_inner();

    // Get the conductor handle
    let host_access = call_context.host_access();
    let conductor_handle = host_access.call_zome_handle();
    let workspace = host_access.workspace();

    // Get the cell id if it's not passed in
    let cell_id = call
        .to_cell
        .unwrap_or_else(|| conductor_handle.cell_id().clone());

    // Create the invocation for this call
    let invocation = ZomeCallInvocation {
        cell_id,
        zome_name: call.zome_name,
        cap: call.cap,
        fn_name: call.fn_name,
        payload: ExternInput::new(call.request),
        provenance: call.provenance,
    };

    // Make the call using this workspace
    let result: ZomeCallResponse = tokio_safe_block_on::tokio_safe_block_forever_on(async move {
        conductor_handle
            .call_zome(invocation, workspace)
            .await
            .map_err(Box::new)
    })??;

    Ok(CallOutput::new(result))
}

#[cfg(test)]
pub mod wasm_test {
    use std::convert::{TryFrom, TryInto};

    use hdk3::prelude::{AgentInfo, CellId};
    use holo_hash::HeaderHash;
    use holochain_serialized_bytes::SerializedBytes;
    use holochain_types::{
        app::InstalledCell,
        dna::{DnaDef, DnaFile},
    };
    use holochain_wasm_test_utils::TestWasm;
    use holochain_zome_types::{test_utils::fake_agent_pubkey_2, ExternInput, ZomeCallResponse};
    use matches::assert_matches;

    use crate::{
        conductor::ConductorHandle,
        core::{ribosome::ZomeCallInvocation, state::element_buf::ElementBuf},
        test_utils::{conductor_setup::ConductorTestData, install_app, new_invocation},
    };

    #[tokio::test(threaded_scheduler)]
    async fn call_test() {
        observability::test_run().ok();

        let zomes = vec![TestWasm::WhoAmI];
        let mut conductor_test = ConductorTestData::two_agents(zomes, true).await;
        let handle = conductor_test.handle();
        let bob_cell_id = conductor_test.bob_call_data().unwrap().cell_id.clone();
        let alice_call_data = conductor_test.alice_call_data();
        let alice_cell_id = &alice_call_data.cell_id;
        let alice_agent_id = alice_cell_id.agent_pubkey();
        let bob_agent_id = bob_cell_id.agent_pubkey();

        // BOB INIT (to do cap grant)

        let _ = handle
            .call_zome(ZomeCallInvocation {
                cell_id: bob_cell_id.clone(),
                zome_name: TestWasm::WhoAmI.into(),
                cap: None,
                fn_name: "set_access".into(),
                payload: ExternInput::new(().try_into().unwrap()),
                provenance: bob_agent_id.clone(),
            })
            .await
            .unwrap();

        // ALICE DOING A CALL

        let output = handle
            .call_zome(ZomeCallInvocation {
                cell_id: alice_cell_id.clone(),
                zome_name: TestWasm::WhoAmI.into(),
                cap: None,
                fn_name: "who_are_they_local".into(),
                payload: ExternInput::new(bob_cell_id.clone().try_into().unwrap()),
                provenance: alice_agent_id.clone(),
            })
            .await
            .unwrap()
            .unwrap();

        match output {
            ZomeCallResponse::Ok(guest_output) => {
                let response: SerializedBytes = guest_output.into_inner();
                let agent_info: AgentInfo = response.try_into().unwrap();
                assert_eq!(
                    agent_info,
                    AgentInfo {
                        agent_initial_pubkey: bob_agent_id.clone(),
                        agent_latest_pubkey: bob_agent_id.clone(),
                    },
                );
            }
            _ => unreachable!(),
        }
        conductor_test.shutdown_conductor().await;
    }

    /// When calling the same cell we need to make sure
    /// the "as at" doesn't cause the original zome call to fail
    /// when they are both writing (moving the source chain forward)
    #[tokio::test(threaded_scheduler)]
    async fn call_the_same_cell() {
        observability::test_run().ok();

        let zomes = vec![TestWasm::WhoAmI, TestWasm::Create];
        let mut conductor_test = ConductorTestData::two_agents(zomes, false).await;
        let handle = conductor_test.handle();
        let alice_call_data = conductor_test.alice_call_data();
        let alice_cell_id = &alice_call_data.cell_id;

        let invocation =
            new_invocation(&alice_cell_id, "call_create_entry", (), TestWasm::Create).unwrap();
        let result = handle.call_zome(invocation).await;
        assert_matches!(result, Ok(Ok(ZomeCallResponse::Ok(_))));

        // Get the header hash of that entry
        let header_hash: HeaderHash =
            unwrap_to::unwrap_to!(result.unwrap().unwrap() => ZomeCallResponse::Ok)
                .clone()
                .into_inner()
                .try_into()
                .unwrap();

        // Check alice's source chain contains the new value
        let alice_source_chain =
            ElementBuf::authored(alice_call_data.env.clone().into(), true).unwrap();
        let el = alice_source_chain.get_element(&header_hash).unwrap();
        assert_matches!(el, Some(_));

        conductor_test.shutdown_conductor().await;
    }

    /// test calling a different zome
    /// in a different cell.
    #[tokio::test(threaded_scheduler)]
    async fn bridge_call() {
        observability::test_run().ok();

        let zomes = vec![TestWasm::Create];
        let mut conductor_test = ConductorTestData::two_agents(zomes, false).await;
        let handle = conductor_test.handle();
        let alice_call_data = conductor_test.alice_call_data();
        let alice_cell_id = &alice_call_data.cell_id;

        // Install a different dna for bob
        let zomes = vec![TestWasm::WhoAmI];
        let bob_cell_id = install_new_app("bobs_dna", zomes, &handle).await;

        // Call create_entry in the create_entry zome from the whoami zome
        let invocation = new_invocation(
            &bob_cell_id,
            "call_create_entry",
            alice_cell_id.clone(),
            TestWasm::WhoAmI,
        )
        .unwrap();
        let result = handle.call_zome(invocation).await;
        assert_matches!(result, Ok(Ok(ZomeCallResponse::Ok(_))));

        // Get the header hash of that entry
        let header_hash: HeaderHash =
            unwrap_to::unwrap_to!(result.unwrap().unwrap() => ZomeCallResponse::Ok)
                .clone()
                .into_inner()
                .try_into()
                .unwrap();

        // Check alice's source chain contains the new value
        let alice_source_chain =
            ElementBuf::authored(alice_call_data.env.clone().into(), true).unwrap();
        let el = alice_source_chain.get_element(&header_hash).unwrap();
        assert_matches!(el, Some(_));

        conductor_test.shutdown_conductor().await;
    }

    async fn install_new_app(
        dna_name: &str,
        zomes: Vec<TestWasm>,
        handle: &ConductorHandle,
    ) -> CellId {
        let dna_file = DnaFile::new(
            DnaDef {
                name: dna_name.to_string(),
                uuid: "ba1d046d-ce29-4778-914b-47e6010d2faf".to_string(),
                properties: SerializedBytes::try_from(()).unwrap(),
                zomes: zomes.clone().into_iter().map(Into::into).collect(),
            },
            zomes.into_iter().map(Into::into),
        )
        .await
        .unwrap();
        let bob_agent_id = fake_agent_pubkey_2();
        let bob_cell_id = CellId::new(dna_file.dna_hash().to_owned(), bob_agent_id.clone());
        let bob_installed_cell = InstalledCell::new(bob_cell_id.clone(), "bob_handle".into());
        let cell_data = vec![(bob_installed_cell, None)];
        install_app("bob_app", cell_data, vec![dna_file], handle.clone()).await;
        bob_cell_id
    }
}
