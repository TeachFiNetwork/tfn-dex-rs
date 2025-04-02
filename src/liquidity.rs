use crate::common::{self, config::*, errors::*};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait LiquidityModule:
common::config::ConfigModule
+super::helpers::HelpersModule
{
    #[endpoint(addLiquidity)]
    #[payable("*")]
    fn add_liquidity(&self) {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);

        let payments = self.call_value().all_esdt_transfers();
        require!(payments.len() == 2, ERROR_WRONG_PAYMENT);

        let mut pair = match self.get_pair_by_tickers(&payments.get(0).token_identifier, &payments.get(1).token_identifier) {
            Option::Some(pair) => pair,
            Option::None => sc_panic!(ERROR_PAIR_NOT_FOUND),
        };
        require!(pair.state != PairState::Inactive, ERROR_PAIR_NOT_ACTIVE);

        if pair.lp_supply == 0 {
            self.only_owner_or_launchpad();
        }

        let caller = self.blockchain().get_caller();
        let (mut base_amount, mut token_amount) = if payments.get(0).token_identifier == pair.token {
            (payments.get(1).amount, payments.get(0).amount)
        } else {
            (payments.get(0).amount, payments.get(1).amount)
        };
        let lp_token_amount = if pair.lp_supply == BigUint::zero() {
            base_amount.clone()
        } else {
            let base_optimal = self.quote(&token_amount, &pair.liquidity_token, &pair.liquidity_base);
            let (token_added, base_added) = if base_optimal < base_amount {
                (token_amount.clone(), base_optimal)
            } else {
                let token_optimal = self.quote(&base_amount, &pair.liquidity_base, &pair.liquidity_token);

                (token_optimal, base_amount.clone())
            };
            // return surplus tokens
            if token_added < token_amount {
                self.send().direct_esdt(&caller, &pair.token, 0, &(&token_amount - &token_added));
                token_amount = token_added;
            }
            if base_added < base_amount {
                self.send().direct_esdt(&caller, &pair.base_token, 0, &(&base_amount - &base_added));
                base_amount = base_added;
            }

            let first_potential_lp = &token_amount * &pair.lp_supply / &pair.liquidity_token;
            let second_potential_lp = &base_amount * &pair.lp_supply / &pair.liquidity_base;

            core::cmp::min(first_potential_lp, second_potential_lp)
        };
        pair.liquidity_base += &base_amount;
        pair.liquidity_token += &token_amount;
        pair.lp_supply += &lp_token_amount;
        self.pair(pair.id).set(&pair);

        self.send().esdt_local_mint(&pair.lp_token, 0, &lp_token_amount);
        self.send().direct_esdt(&caller, &pair.lp_token, 0, &lp_token_amount);
    }

    #[endpoint(removeLiquidity)]
    #[payable("*")]
    fn remove_liquidity(&self) {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);

        let payment = self.call_value().single_esdt();
        let mut pair = match self.get_pair_by_lp_token(&payment.token_identifier) {
            Option::Some(pair) => pair,
            Option::None => sc_panic!(ERROR_WRONG_PAYMENT),
        };
        require!(pair.state != PairState::Inactive, ERROR_PAIR_NOT_ACTIVE);

        let caller = self.blockchain().get_caller();
        let lp_token_amount = payment.amount;
        let base_amount = &pair.liquidity_base * &lp_token_amount / &pair.lp_supply;
        let token_amount = &pair.liquidity_token * &lp_token_amount / &pair.lp_supply;

        pair.liquidity_base -= &base_amount;
        pair.liquidity_token -= &token_amount;
        pair.lp_supply -= &lp_token_amount;
        if pair.lp_supply == 0 {
            pair.state = PairState::ActiveNoSwap;
        }
        self.pair(pair.id).set(&pair);

        self.send().esdt_local_burn(&pair.lp_token, 0, &lp_token_amount);
        self.send().direct_esdt(&caller, &pair.base_token, 0, &base_amount);
        self.send().direct_esdt(&caller, &pair.token, 0, &token_amount);
    }
}
