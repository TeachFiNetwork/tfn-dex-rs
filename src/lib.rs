#![no_std]

multiversx_sc::imports!();

pub mod common;
pub mod swap;
pub mod liquidity;
pub mod helpers;
pub mod proxies;

use common::{config::*, consts::*, errors::*};

#[multiversx_sc::contract]
pub trait TFNDEXContract<ContractReader>:
common::config::ConfigModule
+helpers::HelpersModule
+liquidity::LiquidityModule
+swap::SwapModule
{
    #[init]
    fn init(&self) {
    }

    #[upgrade]
    fn upgrade(&self) {
    }

    #[payable("EGLD")]
    #[endpoint(createPair)]
    fn create_pair(&self, base_token: TokenIdentifier, token: TokenIdentifier) {
        self.only_owner_or_launchpad();
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        require!(self.base_tokens().contains(&base_token), ERROR_WRONG_BASE_TOKEN);
        require!(base_token != token, ERROR_WRONG_BASE_TOKEN);
        require!(self.get_pair_by_tickers(&token, &base_token).is_none(), ERROR_PAIR_EXISTS);

        let mut lp_ticker = token.ticker().concat(base_token.ticker());
        let prefix_suffix_len = LP_TOKEN_PREFIX.len() + LP_TOKEN_SUFFIX.len();
        if lp_ticker.len() > 20 - prefix_suffix_len {
            lp_ticker = lp_ticker.copy_slice(0, 20 - prefix_suffix_len).unwrap();
        }
        let lp_name = ManagedBuffer::from(LP_TOKEN_PREFIX)
            .concat(lp_ticker.clone())
            .concat(ManagedBuffer::from(LP_TOKEN_SUFFIX));
        if lp_ticker.len() > 10 {
            lp_ticker = lp_ticker.copy_slice(0, 10).unwrap();
        }
        let issue_cost = self.call_value().egld_value().clone_value();
        require!(issue_cost == BigUint::from(TOKEN_ISSUE_COST), ERROR_WRONG_ISSUE_COST);

        self.send()
            .esdt_system_sc_proxy()
            .issue_fungible(
                issue_cost,
                lp_name,
                lp_ticker,
                BigUint::zero(),
                FungibleTokenProperties {
                    num_decimals: LP_TOKEN_DECIMALS,
                    can_freeze: true,
                    can_wipe: true,
                    can_pause: true,
                    can_mint: true,
                    can_burn: true,
                    can_change_owner: true,
                    can_upgrade: true,
                    can_add_special_roles: true,
                },
            )
            .with_callback(self.callbacks().lp_token_issue_callback(self.blockchain().get_caller(), &base_token, &token))
            .async_call_and_exit();
    }

    #[callback]
    fn lp_token_issue_callback(
        &self,
        caller: ManagedAddress,
        base_token: &TokenIdentifier,
        token: &TokenIdentifier,
        #[call_result] result: ManagedAsyncCallResult<TokenIdentifier>,
    ) {
        match result {
            ManagedAsyncCallResult::Ok(lp_token) => {
                let id = self.last_pair_id().get();
                let pair = Pair {
                    id,
                    state: PairState::ActiveNoSwap,
                    token: token.clone(),
                    base_token: base_token.clone(),
                    lp_token,
                    lp_supply: BigUint::zero(),
                    liquidity_token: BigUint::zero(),
                    liquidity_base: BigUint::zero(),
                };
                self.last_pair_id().set(id + 1);
                self.pairs(id).set(pair);
            }
            ManagedAsyncCallResult::Err(_) => {
                let issue_cost = self.call_value().egld_value();
                self.send().direct_egld(&caller, &issue_cost);
            }
        }
    }

    // function only used by tests
    // fn test_create_pair(&self, base_token: TokenIdentifier, token: TokenIdentifier) -> TokenIdentifier {
    //     let mut lp_ticker = token.ticker().concat(base_token.ticker());
    //     if lp_ticker.len() > 10 {
    //         lp_ticker = lp_ticker.copy_slice(0, 10).unwrap();
    //     }
    //     let lp_token = TokenIdentifier::from(lp_ticker.concat(ManagedBuffer::from("-123456")));

    //     let id = self.last_pair_id().get();
    //     let pair = Pair {
    //         id,
    //         state: PairState::ActiveNoSwap,
    //         token: token.clone(),
    //         base_token: base_token.clone(),
    //         lp_token: lp_token.clone(),
    //         lp_supply: BigUint::zero(),
    //         liquidity_token: BigUint::zero(),
    //         liquidity_base: BigUint::zero(),
    //     };
    //     self.last_pair_id().set(id + 1);
    //     self.pair(id).set(pair);

    //     lp_token
    // }

    #[endpoint(setPairActive)]
    fn set_pair_active(&self, id: usize) {
        self.only_owner_or_launchpad();
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        require!(!self.pairs(id).is_empty(), ERROR_PAIR_NOT_FOUND);

        let mut pair = self.pairs(id).get();
        require!(pair.lp_supply > 0, ERROR_NO_LIQUIDITY);

        pair.state = PairState::Active;
        self.pairs(id).set(pair);
    }

    #[endpoint(setPairActiveNoSwap)]
    fn set_pair_active_no_swap(&self, id: usize) {
        self.only_owner_or_launchpad();
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        require!(!self.pairs(id).is_empty(), ERROR_PAIR_NOT_FOUND);

        let mut pair = self.pairs(id).get();
        require!(pair.lp_supply > 0, ERROR_NO_LIQUIDITY);

        pair.state = PairState::ActiveNoSwap;
        self.pairs(id).set(pair);
    }

    #[endpoint(setPairInactive)]
    fn set_pair_inactive(&self, id: usize) {
        self.only_owner_or_launchpad();
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        require!(!self.pairs(id).is_empty(), ERROR_PAIR_NOT_FOUND);

        let mut pair = self.pairs(id).get();
        pair.state = PairState::Inactive;
        self.pairs(id).set(pair);
    }

    #[only_owner]
    #[endpoint(addBaseToken)]
    fn add_base_token(&self, token: TokenIdentifier) {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        require!(!self.base_tokens().contains(&token), ERROR_BASE_TOKEN_EXISTS);

        self.base_tokens().insert(token);
    }

    #[only_owner]
    #[endpoint(removeBaseToken)]
    fn remove_base_token(&self, token: TokenIdentifier) {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);
        require!(self.base_tokens().contains(&token), ERROR_WRONG_BASE_TOKEN);

        for pair_id in 0..self.last_pair_id().get() {
            if self.pairs(pair_id).is_empty() {
                continue;
            }

            let pair = self.pairs(pair_id).get();
            require!(pair.base_token != token, ERROR_BASE_TOKEN_IN_USE);
        }
        self.base_tokens().swap_remove(&token);
    }
}
