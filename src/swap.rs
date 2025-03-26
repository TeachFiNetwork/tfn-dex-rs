use crate::common::{self, config::*, errors::*};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait SwapModule:
common::config::ConfigModule
+super::helpers::HelpersModule
{
    #[payable("*")]
    #[endpoint(swapFixedInput)]
    fn swap_fixed_input(
        &self,
        token_out: TokenIdentifier,
        min_amount_out: BigUint,
    ) {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);

        let payment = self.call_value().single_esdt();
        let mut pair = match self.get_pair_by_tickers(&payment.token_identifier, &token_out) {
            Some(pair) => pair,
            None => sc_panic!(ERROR_PAIR_NOT_FOUND),
        };
        require!(pair.state == PairState::Active, ERROR_PAIR_NOT_ACTIVE);

        let fee_in = self.base_tokens().contains(&payment.token_identifier);
        let (amount_out, new_token_liquidity, new_base_liquidity, owner_fee) =
            if token_out == pair.base_token {
                self.do_swap_fixed_input(&payment.amount, &pair.liquidity_token, &pair.liquidity_base, fee_in)
            } else {
                let (amount_out, new_base_liquidity, new_token_liquidity, owner_fee) =
                    self.do_swap_fixed_input(&payment.amount, &pair.liquidity_base, &pair.liquidity_token, fee_in);

                (amount_out, new_token_liquidity, new_base_liquidity, owner_fee)
            };
        require!(amount_out >= min_amount_out, ERROR_INSUFFICIENT_OUTPUT_AMOUNT);

        self.cummulated_fees().insert(pair.base_token.clone(), self.cummulated_fees().get(&pair.base_token).unwrap_or(BigUint::zero()) + owner_fee);
        pair.liquidity_token = new_token_liquidity;
        pair.liquidity_base = new_base_liquidity;
        self.pair(pair.id).set(&pair);

        self.send().direct_esdt(&self.blockchain().get_caller(), &token_out, 0, &amount_out);
    }

    #[payable("*")]
    #[endpoint(swapFixedOutput)]
    fn swap_fixed_output(
        &self,
        token_out: TokenIdentifier,
        amount_out_wanted: BigUint,
    ) {
        require!(self.state().get() == State::Active, ERROR_NOT_ACTIVE);

        let payment = self.call_value().single_esdt();
        let mut pair = match self.get_pair_by_tickers(&payment.token_identifier, &token_out) {
            Some(pair) => pair,
            None => sc_panic!(ERROR_PAIR_NOT_FOUND),
        };
        require!(pair.state == PairState::Active, ERROR_PAIR_NOT_ACTIVE);

        let fee_in = self.base_tokens().contains(&payment.token_identifier);
        let (amount_in, new_token_liquidity, new_base_liquidity, owner_fee) =
            if token_out == pair.base_token {
                self.do_swap_fixed_output(&amount_out_wanted, &pair.liquidity_token, &pair.liquidity_base, fee_in)
            } else {
                let (amount_in, new_base_liquidity, new_token_liquidity, owner_fee) =
                    self.do_swap_fixed_output(&amount_out_wanted, &pair.liquidity_base, &pair.liquidity_token, fee_in);

                (amount_in, new_token_liquidity, new_base_liquidity, owner_fee)
            };
        require!(amount_in > BigUint::zero() && amount_in <= payment.amount, ERROR_INSUFFICIENT_INPUT_AMOUNT);

        self.cummulated_fees().insert(pair.base_token.clone(), self.cummulated_fees().get(&pair.base_token).unwrap_or(BigUint::zero()) + owner_fee);
        pair.liquidity_token = new_token_liquidity;
        pair.liquidity_base = new_base_liquidity;
        self.pair(pair.id).set(&pair);

        let caller = self.blockchain().get_caller();
        self.send().direct_esdt(&caller, &token_out, 0, &amount_out_wanted);
        if amount_in < payment.amount {
            self.send().direct_esdt(&caller, &payment.token_identifier, 0, &(payment.amount - amount_in));
        }
    }

    fn do_swap_fixed_input(
        &self,
        amount_in: &BigUint,
        liquidity_in: &BigUint,
        liquidity_out: &BigUint,
        fee_in: bool,
    ) -> (BigUint, BigUint, BigUint, BigUint) {
        if fee_in {
            let (lp_fee, owner_fee, total_fee) = self.get_fee_amounts(amount_in, true);
            let left_amount_in = amount_in - &total_fee;
            let amount_out = self.get_amount_out_no_fee(&left_amount_in, liquidity_in, liquidity_out);
            let new_liquidity_in = liquidity_in + &left_amount_in + lp_fee;
            let new_liquidity_out = liquidity_out - &amount_out;

            (amount_out, new_liquidity_in, new_liquidity_out, owner_fee)
        } else {
            let amount_out = self.get_amount_out_no_fee(amount_in, liquidity_in, liquidity_out);
            let (lp_fee, owner_fee, total_fee) = self.get_fee_amounts(&amount_out, true);
            let left_amount_out = &amount_out - &total_fee;
            let new_liquidity_in = liquidity_in + amount_in;
            let new_liquidity_out = liquidity_out - &amount_out + lp_fee;

            (left_amount_out, new_liquidity_in, new_liquidity_out, owner_fee)
        }
    }

    fn do_swap_fixed_output(
        &self,
        amount_out: &BigUint,
        liquidity_in: &BigUint,
        liquidity_out: &BigUint,
        fee_in: bool,
    ) -> (BigUint, BigUint, BigUint, BigUint) {
        if fee_in {
            let amount_in_no_fee = self.get_amount_in_no_fee(amount_out, liquidity_in, liquidity_out);
            let (lp_fee, owner_fee, total_fee) = self.get_fee_amounts(&amount_in_no_fee, false);
            let amount_in = &amount_in_no_fee + &total_fee;
            let new_liquidity_in = liquidity_in + &amount_in_no_fee + lp_fee;
            let new_liquidity_out = liquidity_out - amount_out;

            (amount_in, new_liquidity_in, new_liquidity_out, owner_fee)
        } else {
            let (lp_fee, owner_fee, total_fee) = self.get_fee_amounts(amount_out, false);
            let left_amount_out = amount_out + &total_fee;
            let amount_in = self.get_amount_in_no_fee(&left_amount_out, liquidity_in, liquidity_out);
            let new_liquidity_in = liquidity_in + &amount_in;
            let new_liquidity_out = liquidity_out - &left_amount_out + lp_fee;

            (amount_in, new_liquidity_in, new_liquidity_out, owner_fee)
        }
    }

    #[view(getAmountOut)]
    fn get_amount_out_view(
        &self,
        token_in: &TokenIdentifier,
        token_out: &TokenIdentifier,
        amount_in: BigUint,
    ) -> BigUint {
        require!(amount_in > 0, ERROR_ZERO_AMOUNT);

        let pair = match self.get_pair_by_tickers(token_in, token_out) {
            Some(pair) => pair,
            None => sc_panic!(ERROR_PAIR_NOT_FOUND),
        };
        let fee_in = self.base_tokens().contains(token_in);
        if token_in == &pair.token {
            require!(pair.liquidity_base > 0, ERROR_NO_LIQUIDITY);

            self.get_amount_out(&amount_in, &pair.liquidity_token, &pair.liquidity_base, fee_in)
        } else {
            require!(pair.liquidity_token > 0, ERROR_NO_LIQUIDITY);

            self.get_amount_out(&amount_in, &pair.liquidity_base, &pair.liquidity_token, fee_in)
        }
    }

    #[view(getAmountIn)]
    fn get_amount_in_view(
        &self,
        token_in: &TokenIdentifier,
        token_out: &TokenIdentifier,
        amount_out: BigUint,
    ) -> BigUint {
        require!(amount_out > 0, ERROR_ZERO_AMOUNT);

        let pair = match self.get_pair_by_tickers(token_in, token_out) {
            Some(pair) => pair,
            None => sc_panic!(ERROR_PAIR_NOT_FOUND),
        };
        let fee_in = self.base_tokens().contains(token_in);
        if token_in == &pair.token {
            require!(pair.liquidity_base > 0, ERROR_NO_LIQUIDITY);

            self.get_amount_in(&amount_out, &pair.liquidity_token, &pair.liquidity_base, fee_in)
        } else {
            require!(pair.liquidity_token > 0, ERROR_NO_LIQUIDITY);

            self.get_amount_in(&amount_out, &pair.liquidity_base, &pair.liquidity_token, fee_in)
        }
    }
}
