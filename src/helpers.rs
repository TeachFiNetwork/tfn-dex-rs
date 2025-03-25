multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait HelpersModule {
    fn quote(
        &self,
        token_amount: &BigUint,
        token_liquidity: &BigUint,
        base_liquidity: &BigUint,
    ) -> BigUint {
        &(token_amount * base_liquidity) / token_liquidity
    }

    fn get_amount_out_no_fee(
        &self,
        amount_in: &BigUint,
        reserve_in: &BigUint,
        reserve_out: &BigUint,
    ) -> BigUint {
        let numerator = amount_in * reserve_out;
        let denominator = reserve_in + amount_in;

        numerator / denominator
    }

    fn get_amount_in_no_fee(
        &self,
        amount_out: &BigUint,
        reserve_in: &BigUint,
        reserve_out: &BigUint,
    ) -> BigUint {
        let numerator = reserve_in * amount_out;
        let denominator = reserve_out - amount_out;

        (numerator / denominator) + &BigUint::from(1u64)
    }
}
