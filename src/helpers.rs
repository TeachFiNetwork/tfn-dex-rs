use crate::common::{config, consts::*, errors::*};

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

    // returns lp fee, owner fee, total fee calculated from amount
    fn get_fee_amounts(&self, amount: &BigUint, is_input: bool) -> (BigUint, BigUint, BigUint) {
        let lp_fee = self.lp_fee().get();
        let owner_fee = self.owner_fee().get();
        let total_fee = lp_fee + owner_fee;

        if is_input {
            (amount * lp_fee / MAX_PERCENT, amount * owner_fee / MAX_PERCENT, amount * total_fee / MAX_PERCENT)
        } else {
            let total_fee_amount = amount * total_fee / (MAX_PERCENT - total_fee);
            let lp_fee_amount = &total_fee_amount * lp_fee / total_fee;
            let owner_fee_amount = &total_fee_amount - &lp_fee_amount;

            (lp_fee_amount, owner_fee_amount, total_fee_amount)
        }
    }

    fn only_owner_or_launchpad(&self) {
        let caller = self.blockchain().get_caller();
        require!(
            caller == self.blockchain().get_owner_address() || caller == self.launchpad_address().get(),
            ERROR_ONLY_OWNER_OR_LAUNCHPAD
        );
    }
}
