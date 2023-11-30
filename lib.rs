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


    /// This is how you'd write end-to-end (E2E) or integration tests for ink! contracts.
    ///
    /// When running these you need to make sure that you:
    /// - Compile the tests with the `e2e-tests` feature flag enabled (`--features e2e-tests`)
    /// - Are running a Substrate node which contains `pallet-contracts` in the background
    #[cfg(all(test, feature = "e2e-tests"))]
    mod e2e_tests {
        // /// A helper function used for calling contract messages.
        // use ink_e2e::build_message;
        //
        // /// Imports all the definitions from the outer scope so we can use them here.
        // use super::*;
        //
        // /// The End-to-End test `Result` type.
        // type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;
        //
        // /// We test that we can upload and instantiate the contract using its default constructor.
        // #[ink_e2e::test]
        // async fn default_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
        //     // Given
        //     let constructor = Psp37Ref::default();
        //
        //     // When
        //     let contract_account_id = client
        //         .instantiate("psp37", &ink_e2e::alice(), constructor, 0, None)
        //         .await
        //         .expect("instantiate failed")
        //         .account_id;
        //
        //     // Then
        //     let get = build_message::<Psp37Ref>(contract_account_id.clone())
        //         .call(|psp37| psp37.get());
        //     let get_result = client.call_dry_run(&ink_e2e::alice(), &get, 0, None).await;
        //     assert!(matches!(get_result.return_value(), false));
        //
        //     Ok(())
        // }
        //
        // /// We test that we can read and write a value from the on-chain contract contract.
        // #[ink_e2e::test]
        // async fn it_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
        //     // Given
        //     let constructor = Psp37Ref::new(false);
        //     let contract_account_id = client
        //         .instantiate("psp37", &ink_e2e::bob(), constructor, 0, None)
        //         .await
        //         .expect("instantiate failed")
        //         .account_id;
        //
        //     let get = build_message::<Psp37Ref>(contract_account_id.clone())
        //         .call(|psp37| psp37.get());
        //     let get_result = client.call_dry_run(&ink_e2e::bob(), &get, 0, None).await;
        //     assert!(matches!(get_result.return_value(), false));
        //
        //     // When
        //     let flip = build_message::<Psp37Ref>(contract_account_id.clone())
        //         .call(|psp37| psp37.flip());
        //     let _flip_result = client
        //         .call(&ink_e2e::bob(), flip, 0, None)
        //         .await
        //         .expect("flip failed");
        //
        //     // Then
        //     let get = build_message::<Psp37Ref>(contract_account_id.clone())
        //         .call(|psp37| psp37.get());
        //     let get_result = client.call_dry_run(&ink_e2e::bob(), &get, 0, None).await;
        //     assert!(matches!(get_result.return_value(), true));
        //
        //     Ok(())
        // }
    }
}
