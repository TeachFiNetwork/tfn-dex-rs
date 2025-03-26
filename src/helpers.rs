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
        liquidity_in: &BigUint,
        liquidity_out: &BigUint,
    ) -> BigUint {
        let numerator = amount_in * liquidity_out;
        let denominator = liquidity_in + amount_in;

        numerator / denominator
    }

    fn get_amount_in_no_fee(
        &self,
        amount_out: &BigUint,
        liquidity_in: &BigUint,
        liquidity_out: &BigUint,
    ) -> BigUint {
        let numerator = liquidity_in * amount_out;
        let denominator = liquidity_out - amount_out;

        (numerator / denominator) + &BigUint::from(1u64)
    }

    fn get_amount_out(
        &self,
        amount_in: &BigUint,
        liquidity_in: &BigUint,
        liquidity_out: &BigUint,
        fee_in: bool,
    ) -> BigUint {
        let total_fee = self.lp_fee().get() + self.owner_fee().get();
        if fee_in {
            let amount_in_with_fee = amount_in * (MAX_PERCENT - total_fee);
            let numerator = &amount_in_with_fee * liquidity_out;
            let denominator = liquidity_in * MAX_PERCENT + amount_in_with_fee;

            numerator / denominator
        } else {
            let amount_out_no_fee = self.get_amount_out_no_fee(amount_in, liquidity_in, liquidity_out);

            amount_out_no_fee * (MAX_PERCENT - total_fee) / MAX_PERCENT
        }
    }

    fn get_amount_in(
        &self,
        amount_out: &BigUint,
        liquidity_in: &BigUint,
        liquidity_out: &BigUint,
        fee_in: bool,
    ) -> BigUint {
        let total_fee = self.lp_fee().get() + self.owner_fee().get();
        if fee_in {
            let numerator = amount_out * liquidity_in * MAX_PERCENT;
            let denominator = (liquidity_out - amount_out) * (MAX_PERCENT - total_fee);

            (numerator / denominator) + 1u64
        } else {
            let amount_out_with_fee = amount_out * MAX_PERCENT / (MAX_PERCENT - total_fee);

            self.get_amount_in_no_fee(&amount_out_with_fee, liquidity_in, liquidity_out)
        }
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
