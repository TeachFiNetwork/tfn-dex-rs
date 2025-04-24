multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::common::{errors::*, consts::*};
use crate::proxies::launchpad_proxy::{self};

#[type_abi]
#[derive(ManagedVecItem, TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Eq, Copy, Clone, Debug)]
pub enum State {
    Inactive,
    Active,
}

#[type_abi]
#[derive(ManagedVecItem, TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Eq, Copy, Clone, Debug)]
pub enum PairState {
    Inactive,
    ActiveNoSwap,
    Active,
}

#[type_abi]
#[derive(ManagedVecItem, TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Eq, Clone, Debug)]
pub struct Pair<M: ManagedTypeApi> {
    pub id: usize,
    pub state: PairState,
    pub token: TokenIdentifier<M>,
    pub base_token: TokenIdentifier<M>,
    pub lp_token: TokenIdentifier<M>,
    pub lp_supply: BigUint<M>,
    pub liquidity_token: BigUint<M>,
    pub liquidity_base: BigUint<M>,
}

#[multiversx_sc::module]
pub trait ConfigModule {
    // state
    #[only_owner]
    #[endpoint(setStateActive)]
    fn set_state_active(&self) {
        require!(!self.launchpad_address().is_empty(), ERROR_LAUNCHPAD_ADDRESS_NOT_SET);
        require!(!self.base_tokens().is_empty(), ERROR_NO_BASE_TOKENS);

        self.state().set(State::Active);
    }

    #[only_owner]
    #[endpoint(setStateInactive)]
    fn set_state_inactive(&self) {
        self.state().set(State::Inactive);
    }

    #[view(getState)]
    #[storage_mapper("state")]
    fn state(&self) -> SingleValueMapper<State>;

    // launchpad sc
    #[view(getLaunchpadAddress)]
    #[storage_mapper("launchpad_address")]
    fn launchpad_address(&self) -> SingleValueMapper<ManagedAddress>;

    // should only be called by the Launchpad contract at initialization
    #[endpoint(setLaunchpadAddress)]
    fn set_launchpad_address(&self) {
        require!(self.launchpad_address().is_empty(), ERROR_LAUNCHPAD_ADDRESS_ALREADY_SET);

        let address = self.blockchain().get_caller();
        self.launchpad_address().set(&address);
        let governance_token: TokenIdentifier = self.launchpad_contract_proxy()
            .contract(address)
            .governance_token()
            .execute_on_dest_context();
        if !self.base_tokens().contains(&governance_token) {
            self.base_tokens().insert(governance_token);
        }
        self.set_state_active();
    }

    // fees
    #[view(getLPFee)]
    #[storage_mapper("lp_fee")]
    fn lp_fee(&self) -> SingleValueMapper<u64>;

    #[only_owner]
    #[endpoint(setLPFee)]
    fn set_lp_fee(&self, fee: u64) {
        require!(fee + self.owner_fee().get() < MAX_PERCENT, ERROR_WRONG_FEE);

        self.lp_fee().set(fee);
    }

    #[view(getOwnerFee)]
    #[storage_mapper("owner_fee")]
    fn owner_fee(&self) -> SingleValueMapper<u64>;

    #[only_owner]
    #[endpoint(setOwnerFee)]
    fn set_owner_fee(&self, fee: u64) {
        require!(fee + self.lp_fee().get() < MAX_PERCENT, ERROR_WRONG_FEE);

        self.owner_fee().set(fee);
    }

    #[view(getCummulatedFees)]
    #[storage_mapper("cummulated_fees")]
    fn cummulated_fees(&self) -> MapMapper<TokenIdentifier, BigUint>;

    #[only_owner]
    #[endpoint(withdrawFees)]
    fn withdraw_fees(&self) {
        let caller = self.blockchain().get_caller();
        let mut payments: ManagedVec<EsdtTokenPayment> = ManagedVec::new();
        for (token, amount) in self.cummulated_fees().iter() {
            payments.push(EsdtTokenPayment::new(token, 0, amount));
        }
        self.cummulated_fees().clear();
        self.send().direct_multi(&caller, &payments);
    }

    // base tokens
    #[view(getBaseTokens)]
    #[storage_mapper("base_tokens")]
    fn base_tokens(&self) -> UnorderedSetMapper<TokenIdentifier>;

    // pairs
    #[view(getPair)]
    #[storage_mapper("pairs")]
    fn pairs(&self, id: usize) -> SingleValueMapper<Pair<Self::Api>>;

    #[view(getLastPairId)]
    #[storage_mapper("last_pair_id")]
    fn last_pair_id(&self) -> SingleValueMapper<usize>;

    #[view(getPairs)]
    fn get_pairs(&self) -> ManagedVec<Pair<Self::Api>> {
        let last_pair_id = self.last_pair_id().get();
        let mut pairs = ManagedVec::new();
        for id in 0..last_pair_id {
            pairs.push(self.pairs(id).get());
        }

        pairs
    }

    #[view(getPairByTickers)]
    fn get_pair_by_tickers(&self, base_token: &TokenIdentifier, token: &TokenIdentifier) -> Option<Pair<Self::Api>> {
        let last_pair_id = self.last_pair_id().get();
        for id in 0..last_pair_id {
            let pair = self.pairs(id).get();
            if &pair.base_token == base_token && &pair.token == token {
                return Some(pair);
            }
            if &pair.token == base_token && &pair.base_token == token {
                return Some(pair);
            }
        }

        None
    }

    #[view(getPairByLpToken)]
    fn get_pair_by_lp_token(&self, lp_token: &TokenIdentifier) -> Option<Pair<Self::Api>> {
        let last_pair_id = self.last_pair_id().get();
        for id in 0..last_pair_id {
            let pair = self.pairs(id).get();
            if &pair.lp_token == lp_token {
                return Some(pair);
            }
        }

        None
    }

    // proxies
    #[proxy]
    fn launchpad_contract_proxy(&self) -> launchpad_proxy::Proxy<Self::Api>;
}
