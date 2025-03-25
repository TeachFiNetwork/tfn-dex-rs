use crate::common::{self, config::*, consts::*, errors::*};

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
        let (amount_out, new_token_liquidity, new_base_liquidity) =
            if token_out == pair.base_token {
                self.do_swap_fixed_input(&payment.amount, &pair.liquidity_token, &pair.liquidity_base, fee_in)
            } else {
                let (amount_out, new_base_liquidity, new_token_liquidity) =
                    self.do_swap_fixed_input(&payment.amount, &pair.liquidity_base, &pair.liquidity_token, fee_in);

                (amount_out, new_token_liquidity, new_base_liquidity)
            };
        require!(amount_out >= min_amount_out, ERROR_INSUFFICIENT_OUTPUT_AMOUNT);

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
        let (amount_in, new_token_liquidity, new_base_liquidity) =
            if token_out == pair.base_token {
                self.do_swap_fixed_output(&amount_out_wanted, &pair.liquidity_token, &pair.liquidity_base, fee_in)
            } else {
                let (amount_in, new_base_liquidity, new_token_liquidity) =
                    self.do_swap_fixed_output(&amount_out_wanted, &pair.liquidity_base, &pair.liquidity_token, fee_in);

                (amount_in, new_token_liquidity, new_base_liquidity)
            };

        pair.liquidity_token = new_token_liquidity;
        pair.liquidity_base = new_base_liquidity;
        self.pair(pair.id).set(&pair);

        self.send().direct_esdt(&self.blockchain().get_caller(), &payment.token_identifier, 0, &amount_in);
    }

    fn do_swap_fixed_input(
        &self,
        amount_in: &BigUint,
        liquidity_in: &BigUint,
        liquidity_out: &BigUint,
        fee_in: bool,
    ) -> (BigUint, BigUint, BigUint) {
        if fee_in {
            let fee_amount = amount_in * self.fee().get() / MAX_PERCENT;
            let left_amount_in = amount_in - &fee_amount;
            let amount_out = self.get_amount_out_no_fee(&left_amount_in, liquidity_in, liquidity_out);
            let new_liquidity_in = liquidity_in + &left_amount_in;
            let new_liquidity_out = liquidity_out - &amount_out;

            (amount_out, new_liquidity_in, new_liquidity_out)
        } else {
            let amount_out = self.get_amount_out_no_fee(amount_in, liquidity_in, liquidity_out);
            let fee_amount = &amount_out * self.fee().get() / MAX_PERCENT;
            let left_amount_out = &amount_out - &fee_amount;
            let new_liquidity_in = liquidity_in + amount_in;
            let new_liquidity_out = liquidity_out - &amount_out;

            (left_amount_out, new_liquidity_in, new_liquidity_out)
        }
    }

    fn do_swap_fixed_output(
        &self,
        amount_out: &BigUint,
        liquidity_in: &BigUint,
        liquidity_out: &BigUint,
        fee_in: bool,
    ) -> (BigUint, BigUint, BigUint) {
        if fee_in {
            let amount_in_no_fee = self.get_amount_in_no_fee(amount_out, liquidity_in, liquidity_out);
            let fee_amount = &amount_in_no_fee * self.fee().get() / MAX_PERCENT;
            let amount_in = &amount_in_no_fee + &fee_amount;
            let new_liquidity_in = liquidity_in + &amount_in_no_fee;
            let new_liquidity_out = liquidity_out - amount_out;

            (amount_in, new_liquidity_in, new_liquidity_out)
        } else {
            let fee_amount = amount_out * self.fee().get() / MAX_PERCENT;
            let left_amount_out = amount_out + &fee_amount;
            let amount_in = self.get_amount_in_no_fee(&left_amount_out, liquidity_in, liquidity_out);
            let new_liquidity_in = liquidity_in + &amount_in;
            let new_liquidity_out = liquidity_out - &left_amount_out;

            (amount_in, new_liquidity_in, new_liquidity_out)
        }
    }
}
