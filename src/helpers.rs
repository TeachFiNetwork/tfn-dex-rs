use crate::common::{config, errors::*};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait HelpersModule:
config::ConfigModule
{
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

    fn only_owner_or_launchpad(&self) {
        let caller = self.blockchain().get_caller();
        require!(
            caller == self.blockchain().get_owner_address() || caller == self.launchpad_address().get(),
            ERROR_ONLY_OWNER_OR_LAUNCHPAD
        );
    }
}
