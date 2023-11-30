use ink::{
    prelude::{string::String, vec, vec::Vec},
    storage::Mapping,
};
use ink::primitives::AccountId;
#[cfg(feature = "std")]
use ink::storage::traits::StorageLayout;

use crate::PSP37Error;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, StorageLayout))]
pub enum Id {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    Bytes(Vec<u8>),
}

enum AllowanceValue {
    Infinite,
    Finite(u128),
    None,
}

// `u128` must be enough to cover most of the use-cases of standard tokens.
pub type Balance = u128;


#[derive(Debug, PartialOrd, PartialEq)]
pub enum PSP37Event {
    Transfer {
        from: Option<AccountId>,
        to: Option<AccountId>,
        id: Id,
        value: Balance,
    },
    TransferBatch {
        from: Option<AccountId>,
        to: Option<AccountId>,
        ids_amounts: Vec<(Id, Balance)>,
    },
    Approval {
        owner: AccountId,
        operator: AccountId,
        id: Option<Id>,
        value: Balance,
    },
    AttributeSet {
        id: Id,
        key: String,
        data: String,
    },
}

pub type ApprovalKey = (AccountId, AccountId, Option<Id>);

#[ink::storage_item]
#[derive(Debug, Default)]
pub struct PSP37Data {
    token_owner: Mapping<Id, AccountId>,
    owned_serials_count: Mapping<(AccountId, Id), u128>,
    owned_tokens_count_by_account: Mapping<AccountId, u128>,
    operator_approvals: Mapping<ApprovalKey, u128>,
    total_supply_by_id: Mapping<Id, u128>,
    total_token_count: u128,
}

impl PSP37Data {
    pub fn new() -> PSP37Data {
        Default::default()
    }

    pub fn collection_id(&self, account_id: AccountId) -> Id {
        Id::Bytes(<_ as AsRef<[u8; 32]>>::as_ref(&account_id).to_vec())
    }

    pub fn owner_of(&self, id: &Id) -> Option<AccountId> {
        self.token_owner.get(id)
    }

    fn balance_by_id(&self, owner: AccountId, id: &Id) -> Balance {
        self.owned_serials_count.get((owner, id)).unwrap_or_default()
    }

    fn balance_by_account(&self, owner: AccountId) -> Balance {
        self.owned_tokens_count_by_account.get(owner).unwrap_or_default()
    }

    pub fn balance_of(&self, owner: AccountId, id: Option<Id>) -> Balance {
        match id {
            None => {
                self.owned_tokens_count_by_account.get(owner).unwrap_or_default()
            }
            Some(id) => {
                self.owned_serials_count.get((owner, id)).unwrap_or_default()
            }
        }
    }

    pub fn total_supply(&self, id: Option<Id>) -> u128 {
        match id {
            None => {
                self.total_token_count
            }
            Some(id) => {
                self.total_supply_by_id.get(id).unwrap_or_default()
            }
        }
    }


    pub fn allowance(&self, owner: AccountId, operator: AccountId, id: Option<Id>) -> Balance {
        self.operator_approvals.get((owner, operator, id)).unwrap_or_default()
    }

    fn allowance_value_wrapped(&self, owner: AccountId, operator: AccountId, id: &Id) -> AllowanceValue {
        self.operator_approvals.get((owner, operator, Some(id)))
            .map(AllowanceValue::Finite)
            .or_else(|| {
                self.operator_approvals.get((owner, operator, &None)).map(|_| AllowanceValue::Infinite)
            }).unwrap_or(AllowanceValue::None)
    }

    pub fn approve(&mut self, owner: AccountId, operator: AccountId, id: Option<Id>, value: Balance) -> Result<Vec<PSP37Event>, PSP37Error> {
        if owner == operator {
            return Ok(vec![]);
        }

        let allowance_value = match id {
            None => Balance::MAX,
            Some(_) => value
        };

        self.operator_approvals.insert((owner, operator, id.clone()), &allowance_value);

        Ok(vec![
            PSP37Event::Approval {
                owner,
                operator,
                id,
                value: allowance_value,
            }
        ])
    }

    fn transfer_internal(
        &mut self,
        caller: AccountId,
        to: AccountId,
        id: Id,
        value: u128,
        _data: Vec<u8>,
    ) -> Result<Vec<PSP37Event>, PSP37Error> {
        let owner = self.owner_of(&id).ok_or(PSP37Error::TokenNotExists)?;

        if owner == to || value == 0 {
            return Ok(vec![]);
        }

        if owner != caller {
            return Err(PSP37Error::NotApproved);
        }

        let from_balance = self.balance_by_id(owner, &id);
        let from_token_balance = self.balance_by_account(owner);

        let balance_after = from_balance.checked_sub(value).ok_or(PSP37Error::InsufficientBalance)?;

        self.owned_serials_count.insert((owner, id.clone()), &balance_after);

        if balance_after == 0 {
            let tokens_count_after = from_token_balance.saturating_sub(1);
            self.owned_tokens_count_by_account.insert(owner, &tokens_count_after);
        }

        self.token_owner.remove(&id);
        self.token_owner.insert(&id, &to);

        let to_balance = self.balance_of(to, Some(id.clone()));

        self.owned_serials_count
            .insert((to, id.clone()), &(to_balance.checked_add(1).unwrap()));

        Ok(vec![PSP37Event::Transfer {
            from: Some(caller),
            to: Some(to),
            id,
            value,
        }])
    }

    fn handle_transfer_allowance_internal(&mut self, owner: AccountId, caller: AccountId, id: &Id, value: Balance) -> Result<(), PSP37Error> {
        let allowance_balance_wrapped = self.allowance_value_wrapped(owner, caller, &id);

        if let AllowanceValue::Finite(allowance_balance) = allowance_balance_wrapped {
            if owner != caller && allowance_balance < value {
                return Err(PSP37Error::NotApproved);
            }
            if owner != caller {
                let allowance_after = allowance_balance.saturating_sub(value);
                self.operator_approvals.insert((owner, caller, Some(id.clone())), &allowance_after);
            }
        }
        Ok(())
    }

    pub fn transfer(
        &mut self,
        caller: AccountId,
        to: AccountId,
        id: Id,
        value: u128,
        _data: Vec<u8>,
    ) -> Result<Vec<PSP37Event>, PSP37Error> {
        self.transfer_internal(caller, to, id, value, _data)
    }


    pub fn transfer_from(
        &mut self,
        caller: AccountId,
        to: AccountId,
        id: Id,
        value: u128,
        _data: Vec<u8>,
    ) -> Result<Vec<PSP37Event>, PSP37Error> {
        let owner = self.owner_of(&id).ok_or(PSP37Error::TokenNotExists)?;

        if owner == to || value == 0 {
            return Ok(vec![]);
        }

        let from_balance = self.balance_by_id(owner, &id);
        let from_token_balance = self.balance_by_account(owner);

        let balance_after = from_balance.checked_sub(value).ok_or(PSP37Error::InsufficientBalance)?;

        self.owned_serials_count.insert((owner, id.clone()), &balance_after);

        if balance_after == 0 {
            let tokens_count_after = from_token_balance.saturating_sub(1);
            self.owned_tokens_count_by_account.insert(owner, &tokens_count_after);
        }

        self.handle_transfer_allowance_internal(owner, caller, &id, value)?;

        self.token_owner.remove(&id);
        self.token_owner.insert(&id, &to);

        let to_balance = self.balance_of(to, Some(id.clone()));

        self.owned_serials_count
            .insert((to, id.clone()), &(to_balance.checked_add(1).unwrap()));

        Ok(vec![PSP37Event::Transfer {
            from: Some(caller),
            to: Some(to),
            id,
            value,
        }])
    }
}

#[cfg(test)]
mod tests {
    /// Imports all the definitions from the outer scope so we can use them here.
    use super::*;

    #[ink::test]
    fn transfer_works() {
        let mut psp37 = PSP37Data::new();
        let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

        psp37.token_owner.insert(Id::U8(1), &accounts.alice);
        psp37.owned_serials_count.insert((accounts.alice, Id::U8(1)), &1);
        psp37.owned_tokens_count_by_account.insert(accounts.alice, &1);
        psp37.total_supply_by_id.insert(Id::U8(1), &1);
        psp37.total_token_count = 1;

        let events = psp37.transfer(accounts.alice, accounts.bob, Id::U8(1), 1, vec![]).unwrap();

        assert_eq!(psp37.balance_of(accounts.alice, Some(Id::U8(1))), 0);
        assert_eq!(psp37.balance_of(accounts.bob, Some(Id::U8(1))), 1);

        assert_eq!(psp37.token_owner.get(Id::U8(1)), Some(accounts.bob));

        assert_eq!(events.len(), 1);
        assert_eq!(events[0], PSP37Event::Transfer {
            from: Some(accounts.alice),
            to: Some(accounts.bob),
            id: Id::U8(1),
            value: 1,
        });
    }

    #[ink::test]
    fn transfer_works_same_target() {
        let mut psp37 = PSP37Data::new();
        let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

        psp37.token_owner.insert(Id::U8(1), &accounts.alice);
        psp37.owned_serials_count.insert((accounts.alice, Id::U8(1)), &1);
        psp37.owned_tokens_count_by_account.insert(accounts.alice, &1);
        psp37.total_supply_by_id.insert(Id::U8(1), &1);
        psp37.total_token_count = 1;

        let events = psp37.transfer(accounts.alice, accounts.alice, Id::U8(1), 1, vec![]).unwrap();

        assert_eq!(psp37.balance_of(accounts.alice, Some(Id::U8(1))), 1);
        assert_eq!(psp37.token_owner.get(Id::U8(1)), Some(accounts.alice));

        assert_eq!(events, vec![]);
    }

    #[ink::test]
    fn transfer_not_enough_balance() {
        let mut psp37 = PSP37Data::new();
        let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

        psp37.token_owner.insert(Id::U8(1), &accounts.alice);
        psp37.owned_serials_count.insert((accounts.alice, Id::U8(1)), &1);
        psp37.owned_tokens_count_by_account.insert(accounts.alice, &1);
        psp37.total_supply_by_id.insert(Id::U8(1), &1);
        psp37.total_token_count = 1;

        let transfer_result = psp37.transfer(accounts.alice, accounts.charlie, Id::U8(1), 123, vec![]);

        assert_eq!(psp37.balance_of(accounts.alice, Some(Id::U8(1))), 1);
        assert_eq!(psp37.token_owner.get(Id::U8(1)), Some(accounts.alice));

        assert_eq!(psp37.balance_of(accounts.charlie, Some(Id::U8(1))), 0);

        assert!(transfer_result.is_err());
        assert_eq!(transfer_result.unwrap_err(), PSP37Error::InsufficientBalance);
    }

    #[ink::test]
    fn transfer_id_not_found() {
        let mut psp37 = PSP37Data::new();
        let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

        psp37.token_owner.insert(Id::U8(1), &accounts.alice);
        psp37.owned_serials_count.insert((accounts.alice, Id::U8(1)), &1);
        psp37.owned_tokens_count_by_account.insert(accounts.alice, &1);
        psp37.total_supply_by_id.insert(Id::U8(1), &1);
        psp37.total_token_count = 1;

        let transfer_result = psp37.transfer(accounts.alice, accounts.charlie, Id::U8(123), 1, vec![]);

        assert_eq!(psp37.balance_of(accounts.alice, Some(Id::U8(1))), 1);
        assert_eq!(psp37.token_owner.get(Id::U8(1)), Some(accounts.alice));

        assert_eq!(psp37.balance_of(accounts.charlie, Some(Id::U8(1))), 0);

        assert!(transfer_result.is_err());
        assert_eq!(transfer_result.unwrap_err(), PSP37Error::TokenNotExists);
    }

    #[ink::test]
    fn transfer_from_works() {
        let mut psp37 = PSP37Data::new();
        let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

        psp37.token_owner.insert(Id::U8(1), &accounts.alice);
        psp37.owned_serials_count.insert((accounts.alice, Id::U8(1)), &1);
        psp37.owned_tokens_count_by_account.insert(accounts.alice, &1);
        psp37.total_supply_by_id.insert(Id::U8(1), &1);
        psp37.total_token_count = 1;

        psp37.transfer_from(accounts.alice, accounts.bob, Id::U8(1), 1, vec![]).unwrap();

        assert_eq!(psp37.balance_of(accounts.alice, Some(Id::U8(1))), 0);
        assert_eq!(psp37.balance_of(accounts.bob, Some(Id::U8(1))), 1);

        assert_eq!(psp37.token_owner.get(Id::U8(1)), Some(accounts.bob));
    }


    #[ink::test]
    fn approve_works_finite_amount() {
        let mut psp37 = PSP37Data::new();
        let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

        let events = psp37.approve(accounts.alice, accounts.bob, Some(Id::U8(1)), 23).unwrap();

        assert_eq!(psp37.allowance(accounts.alice, accounts.bob, Some(Id::U8(1))), 23);

        assert_eq!(psp37.operator_approvals.get((accounts.alice, accounts.bob, Some(Id::U8(1)))), Some(23));
        assert_eq!(psp37.operator_approvals.get((accounts.alice, accounts.bob, &None)), None);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0], PSP37Event::Approval {
            owner: accounts.alice,
            operator: accounts.bob,
            id: Some(Id::U8(1)),
            value: 23,
        });
    }

    #[ink::test]
    fn approve_works_caller_is_operator() {
        let mut psp37 = PSP37Data::new();
        let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

        let events = psp37.approve(accounts.charlie, accounts.charlie, Some(Id::U8(1)), 12).unwrap();

        assert_eq!(events.len(), 0);
        assert!(psp37.operator_approvals.get((accounts.charlie, accounts.charlie, Some(Id::U8(1)))).is_none());
    }

    #[ink::test]
    fn approve_works_all_tokens() {
        let mut psp37 = PSP37Data::new();
        let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

        let events = psp37.approve(accounts.alice, accounts.bob, None, 2).unwrap();

        assert_eq!(psp37.operator_approvals.get((accounts.alice, accounts.bob, &None)), Some(Balance::MAX));

        assert_eq!(events.len(), 1);
        assert_eq!(events[0], PSP37Event::Approval {
            owner: accounts.alice,
            operator: accounts.bob,
            id: None,
            value: Balance::MAX,
        });
    }

    #[ink::test]
    fn allowance_works_default_value() {
        let psp37 = PSP37Data::new();
        let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

        let allowance = psp37.allowance(accounts.alice, accounts.bob, Some(Id::U8(1)));

        assert_eq!(allowance, 0);
    }

    #[ink::test]
    fn allowance_works_explicit_token() {
        let mut psp37 = PSP37Data::new();
        let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
        let allowance_value = 23;

        psp37.operator_approvals.insert((accounts.alice, accounts.bob, Some(Id::U8(1))), &allowance_value);

        let allowance = psp37.allowance(accounts.alice, accounts.bob, Some(Id::U8(1)));

        assert_eq!(allowance, allowance_value);
    }

    #[ink::test]
    fn balance_of_works_default_value() {
        let psp37 = PSP37Data::new();
        let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
        let balance = psp37.balance_of(accounts.alice, Some(Id::U8(1)));

        assert_eq!(balance, 0);
    }

    #[ink::test]
    fn balance_of_works() {
        let mut psp37 = PSP37Data::new();
        let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();

        psp37.token_owner.insert(Id::U8(1), &accounts.alice);
        psp37.owned_serials_count.insert((accounts.alice, Id::U8(1)), &105);
        psp37.owned_tokens_count_by_account.insert(accounts.alice, &1);
        psp37.total_supply_by_id.insert(Id::U8(1), &105);
        psp37.total_token_count = 1;

        assert_eq!(psp37.balance_of(accounts.alice, Some(Id::U8(1))), 105);

        assert_eq!(psp37.balance_of(accounts.alice, None), 1);
    }
}