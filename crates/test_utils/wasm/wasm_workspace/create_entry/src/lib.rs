use hdk3::prelude::*;

#[hdk_entry(
    id = "post",
    required_validations = 5,
    required_validation_type = "full"
)]
struct Post(String);

#[hdk_entry(
    id = "msg",
    required_validations = 5,
    required_validation_type = "sub_chain"
)]
struct Msg(String);

#[hdk_entry(
    id = "priv_msg",
    required_validations = 5,
    required_validation_type = "full",
    visibility = "private"
)]
struct PrivMsg(String);

entry_defs![Post::entry_def(), Msg::entry_def(), PrivMsg::entry_def()];

fn post() -> Post {
    Post("foo".into())
}

fn msg() -> Msg {
    Msg("hello".into())
}

fn priv_msg() -> PrivMsg {
    PrivMsg("Don't tell anyone".into())
}

#[hdk_extern]
fn create_entry(_: ()) -> ExternResult<HeaderHash> {
    Ok(hdk3::prelude::create_entry(&post())?)
}

#[hdk_extern]
fn create_post(post: Post) -> ExternResult<HeaderHash> {
    Ok(hdk3::prelude::create_entry(&post)?)
}

#[hdk_extern]
fn get_entry(_: ()) -> ExternResult<GetOutput> {
    Ok(GetOutput::new(get(hash_entry(&post())?, GetOptions)?))
}

#[hdk_extern]
fn create_msg(_: ()) -> ExternResult<HeaderHash> {
    Ok(hdk3::prelude::create_entry(&msg())?)
}

#[hdk_extern]
fn create_priv_msg(_: ()) -> ExternResult<HeaderHash> {
    Ok(hdk3::prelude::create_entry(&priv_msg())?)
}

#[hdk_extern]
fn validate_create_entry_post(
    validation_data: ValidateData,
) -> ExternResult<ValidateCallbackResult> {
    let element = validation_data.element;
    let r = match element.entry().to_app_option::<Post>() {
        Ok(Some(post)) if &post.0 == "Banana" => {
            ValidateCallbackResult::Invalid("No Bananas!".to_string())
        }
        _ => ValidateCallbackResult::Valid,
    };
    Ok(r)
}

#[hdk_extern]
fn get_activity(input: test_wasm_common::AgentActivitySearch) -> ExternResult<AgentActivity> {
    Ok(get_agent_activity(input.agent, input.query, input.request)?)
}

#[hdk_extern]
fn init(_: ()) -> ExternResult<InitCallbackResult> {
    // grant unrestricted access to accept_cap_claim so other agents can send us claims
    let mut functions: GrantedFunctions = HashSet::new();
    functions.insert((zome_info()?.zome_name, "create_entry".into()));
    create_cap_grant(CapGrantEntry {
        tag: "".into(),
        // empty access converts to unrestricted
        access: ().into(),
        functions,
    })?;

    Ok(InitCallbackResult::Pass)
}

/// Create a post entry then
/// create another post through a
/// call
#[hdk_extern]
fn call_create_entry(_: ()) -> ExternResult<HeaderHash> {
    // Create an entry directly via. the hdk.
    hdk3::prelude::create_entry(&post())?;
    // Create an entry via a `call`.
    Ok(call(
        None,
        "create_entry".to_string().into(),
        "create_entry".to_string().into(),
        None,
        &(),
    )?)
}

#[hdk_extern]
fn call_create_entry_remotely(agent: AgentPubKey) -> ExternResult<HeaderHash> {
    Ok(call_remote(
        agent,
        "create_entry".to_string().into(),
        "create_entry".to_string().into(),
        None,
        &(),
    )?)
}
