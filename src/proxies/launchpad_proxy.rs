multiversx_sc::imports!();

#[multiversx_sc::proxy]
pub trait LaunchpadProxy {
    #[view(getGovernanceToken)]
    #[storage_mapper("governance_token")]
    fn governance_token(&self) -> SingleValueMapper<TokenIdentifier>;
}
