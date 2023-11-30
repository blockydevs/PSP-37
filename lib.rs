#![cfg_attr(not(feature = "std"), no_std, no_main)]

pub use data::{Id, PSP37Data, PSP37Event};
pub use errors::PSP37Error;
pub use traits::PSP37;

mod data;
mod errors;
mod traits;

#[ink::contract]
mod token {
    use ink::prelude::string::String;
    use ink::prelude::vec::Vec;

    use crate::{Id, PSP37, PSP37Data, PSP37Error, PSP37Event};

    #[ink(storage)]
    pub struct Token {
        data: PSP37Data,
    }

    impl Token {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                data: PSP37Data::new()
            }
        }

        fn emit_events(&self, events: Vec<PSP37Event>) {
            for event in events {
                match event {
                    PSP37Event::Transfer { from, to, id, value } => {
                        self.env().emit_event(Transfer { from, to, id, value })
                    }
                    PSP37Event::Approval {
                        owner,
                        operator,
                        id,
                        value
                    } => self.env().emit_event(Approval {
                        owner,
                        operator,
                        id,
                        value,
                    }),
                    PSP37Event::TransferBatch { from, to, ids_amounts } => {
                        self.env().emit_event(TransferBatch {
                            from,
                            to,
                            ids_amounts,
                        })
                    }
                    PSP37Event::AttributeSet { id, key, data } => {
                        self.env().emit_event(AttributeSet {
                            id,
                            key,
                            data,
                        })
                    }
                }
            }
        }
    }

    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        #[ink(topic)]
        id: Id,
        #[ink(topic)]
        value: Balance,
    }

    #[ink(event)]
    pub struct TransferBatch {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        #[ink(topic)]
        ids_amounts: Vec<(Id, Balance)>,
    }

    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        operator: AccountId,
        #[ink(topic)]
        id: Option<Id>,
        #[ink(topic)]
        value: Balance,
    }

    #[ink(event)]
    pub struct AttributeSet {
        id: Id,
        key: String,
        data: String,
    }

    impl PSP37 for Token {
        #[ink(message)]
        fn balance_of(&self, owner: AccountId, id: Option<Id>) -> u128 {
            self.data.balance_of(owner, id)
        }

        #[ink(message)]
        fn total_supply(&self, id: Option<Id>) -> u128 {
            self.data.total_supply(id)
        }

        #[ink(message)]
        fn allowance(&self, owner: AccountId, operator: AccountId, id: Option<Id>) -> u128 {
            self.data.allowance(owner, operator, id)
        }

        #[ink(message)]
        fn approve(&mut self, operator: AccountId, id: Option<Id>, value: Balance) -> Result<(), PSP37Error> {
            let events = self.data.approve(self.env().caller(), operator, id, value)?;
            self.emit_events(events);
            Ok(())
        }

        #[ink(message)]
        fn transfer(&mut self, to: AccountId, id: Id, value: u128, data: Vec<u8>) -> Result<(), PSP37Error> {
            let events = self.data.transfer(self.env().caller(), to, id, value, data)?;
            self.emit_events(events);
            Ok(())
        }


        #[ink(message)]
        fn transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            id: Id,
            value: u128,
            data: Vec<u8>,
        ) -> Result<(), PSP37Error> {
            let events = self.data.transfer_from(from, to, id, value, data)?;
            self.emit_events(events);
            Ok(())
        }
    }


    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        #[ink::test]
        fn new_works() {
            let psp37 = Token::new();

            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

            assert_eq!(psp37.balance_of(accounts.alice, None), 0);
            assert_eq!(psp37.balance_of(accounts.alice, Some(Id::U8(1))), 0);
        }
    }


    #[cfg(all(test, feature = "e2e-tests"))]
    mod e2e_tests {
    }
}
