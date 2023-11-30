use ink::prelude::vec::Vec;
use ink::primitives::AccountId;

use crate::data::{Balance, Id};
use crate::errors::PSP37Error;

#[ink::trait_definition]
pub trait PSP37 {
    /// Returns the amount of tokens of token type `id` owned by `account`.
    ///  If `id` is `None` returns the total number of `owner`'s tokens.
    #[ink(message)]
    fn balance_of(&self, owner: AccountId, id: Option<Id>) -> Balance;

    /// Returns the total amount of token type `id` in the supply.
    /// If `id` is `None` returns the total number of tokens.
    #[ink(message)]
    fn total_supply(&self, id: Option<Id>) -> Balance;

    #[ink(message)]
    fn allowance(&self, owner: AccountId, operator: AccountId, id: Option<Id>) -> Balance;

    #[ink(message)]
    fn approve(&mut self, operator: AccountId, id: Option<Id>, value: Balance) -> Result<(), PSP37Error>;

    #[ink(message)]
    fn transfer(&mut self, to: AccountId, id: Id, value: u128, data: Vec<u8>) -> Result<(), PSP37Error>;


    #[ink(message)]
    fn transfer_from(
        &mut self,
        from: AccountId,
        to: AccountId,
        id: Id,
        value: u128,
        data: Vec<u8>,
    ) -> Result<(), PSP37Error>;
}