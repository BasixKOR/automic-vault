pub const NAME: &str = "aws";

pub fn credential_helper(invocation: crate::isotope::CredentialHelperInvocation<'_>) -> Result<(), String> {
    if invocation.args.iter().any(|arg| arg == "--help" || arg == "--version") {
        return Ok(());
    }
    let token = invocation.caller.token.as_deref().ok_or_else(|| "missing approval token".to_string())?;
    if token.len() < 32 {
        return Err("invalid approval token".to_string());
    }
    let parent = invocation
        .caller
        .parent_executable_path
        .as_deref()
        .ok_or_else(|| "missing helper parent executable".to_string())?;
    if !parent.contains("aws") {
        return Err("credential helper invoked by unexpected launcher".to_string());
    }
    Ok(())
}
